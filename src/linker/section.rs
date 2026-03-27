//! セクションマージ＆レイアウト (Step 3〜4)
//!
//! C++ 版: `createPeFromObj` Step 3〜4 (main.cpp)

use std::collections::HashMap;

use crate::binary_io::align_up;
use crate::coff::object_file::ObjectFile;
use crate::coff::section_header::{SectionHeader, ch};
use crate::pe::dos_header::DosHeader;
use crate::pe::pe_file::PeSection;
use crate::pe::pe_header::PeHeader;

use super::options::LinkerOptions;

// ---------------------------------------------------------------------------
// 公開型
// ---------------------------------------------------------------------------

/// あるobjのセクションが、マージ後のPEセクション内のどこに配置されたか
pub struct PeSectionPosition {
    pub section_index: usize, // pe_sections の何番目か
    pub offset: u32,          // そのPEセクション内のバイトオフセット
}

/// (obj_index, section_name) → PeSectionPosition
pub type ObjSectionMap = HashMap<(usize, String), PeSectionPosition>;

pub struct SectionLayout {
    pub pe_sections: Vec<PeSection>,
    /// Step 5 (シンボル解決) で使う obj→PE位置のマッピング
    pub obj_section_map: ObjSectionMap,
    pub size_of_headers: u32,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,
}

// ---------------------------------------------------------------------------
// 非公開: セクション種別
// ---------------------------------------------------------------------------

/// characteristics フラグから判定したセクションの種別
#[derive(PartialEq)]
enum SectionKind {
    Code, // .text — 実行可能コード
    Data, // .data/.rdata — 初期値ありデータ
    Bss,  // .bss  — 初期値なし (ファイル上に実体を持たない)
    Other,
}

impl SectionKind {
    /// COFF セクションヘッダの characteristics フラグからセクション種別を判定する。
    /// 複数のフラグが立っている場合は Code → Data → Bss の優先順で判定する。
    fn from_characteristics(characteristics: u32) -> Self {
        if characteristics & ch::CONTAINS_CODE != 0 {
            SectionKind::Code
        } else if characteristics & ch::CONTAINS_INITIALIZED_DATA != 0 {
            SectionKind::Data
        } else if characteristics & ch::CONTAINS_UNINITIALIZED_DATA != 0 {
            SectionKind::Bss
        } else {
            SectionKind::Other
        }
    }

    /// ソート順キーを返す (小さいほど先頭に来る)。
    /// Code=0 → Data=1 → Bss=2 → Other=3 の順になる。
    fn sort_key(&self) -> u32 {
        match self {
            SectionKind::Code => 0,
            SectionKind::Data => 1,
            SectionKind::Bss => 2,
            SectionKind::Other => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// 非公開: マージ中間データ
// ---------------------------------------------------------------------------

/// あるobjのセクションが MergedSection::data 内のどこから始まるか
#[derive(Debug)]
struct ObjSectionRef {
    obj_index: usize,
    offset: u32,
}

/// 同名セクションをマージした中間データ
#[derive(Debug)]
struct MergedSection {
    name: [u8; 8],
    data: Vec<u8>,
    characteristics: u32,
    obj_sections: Vec<ObjSectionRef>,
}

impl MergedSection {
    /// このセクションの種別を返す
    fn kind(&self) -> SectionKind {
        SectionKind::from_characteristics(self.characteristics)
    }
}

fn name_to_str(name: &[u8; 8]) -> String {
    let end = name.iter().position(|&b| b == 0).unwrap_or(8);
    String::from_utf8_lossy(&name[..end]).into_owned()
}

// ---------------------------------------------------------------------------
// Step 3: 同名セクションのマージ
// ---------------------------------------------------------------------------

fn merge_sections(obj_files: &[ObjectFile]) -> Vec<MergedSection> {
    let mut order: Vec<String> = Vec::new(); // 挿入順を保持
    let mut map: HashMap<String, MergedSection> = HashMap::new();

    for (obj_index, obj) in obj_files.iter().enumerate() {
        for obj_sec in &obj.sections {
            let name = name_to_str(&obj_sec.header.name);

            if let Some(merged) = map.get_mut(&name) {
                // 既存: データを末尾に追記
                let offset = merged.data.len() as u32;
                merged
                    .obj_sections
                    .push(ObjSectionRef { obj_index, offset });
                merged.data.extend_from_slice(&obj_sec.data);
            } else {
                // 新規登録
                order.push(name.clone());
                map.insert(
                    name,
                    MergedSection {
                        name: obj_sec.header.name,
                        data: obj_sec.data.clone(),
                        characteristics: obj_sec.header.characteristics,
                        obj_sections: vec![ObjSectionRef {
                            obj_index,
                            offset: 0,
                        }],
                    },
                );
            }
        }
    }

    order
        .into_iter()
        .map(|name| map.remove(&name).unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// Step 4: レイアウト計算
// ---------------------------------------------------------------------------

/// セクションをマージ・ソートし、PE上のアドレス/オフセットを確定する。
pub fn merge_and_layout(obj_files: &[ObjectFile], opts: &LinkerOptions) -> SectionLayout {
    // Step 3
    let mut sections = merge_sections(obj_files);
    // Step 4: ソート
    sections.sort_by_key(|s| s.kind().sort_key());

    // ヘッダ領域のサイズを計算 (import用に+2セクション分を予約)
    let max_sections = (sections.len() + 2) as u32;
    let header_bytes = DosHeader::SIZE + PeHeader::SIZE + SectionHeader::SIZE * max_sections;
    // header_bytes バイトのヘッダを file_alignment バイト単位で何ブロック確保するか
    // 例: header_bytes=530, file_alignment=512 → 2ブロック → size_of_headers=1024
    let size_of_headers = header_bytes.div_ceil(opts.file_alignment) * opts.file_alignment;
    let mut raw_address = size_of_headers;
    // 最初のセクションの仮想アドレスを求める。
    // ヘッダが何ページ目に収まるかを整数除算で求め、その「次のページ」を開始点にする。
    // 例: size_of_headers=0x400, section_alignment=0x1000
    //   → 0x400 / 0x1000 = 0 (0ページ目に収まる)
    //   → (0 + 1) × 0x1000 = 0x1000 (1ページ目から開始)
    // +1 により、ヘッダがページ境界にぴったりの場合でも重ならず次のページになる。
    // 実際には size_of_headers < section_alignment なので常に 0x1000 になる。
    let mut virtual_address =
        (size_of_headers / opts.section_alignment + 1) * opts.section_alignment;

    // 出力 PE ファイルのセクション一覧 (レイアウト確定後に SectionLayout に格納する)
    let mut pe_sections: Vec<PeSection> = Vec::new();
    // (obj_index, セクション名) → PE 上の位置。リロケーション処理でシンボルアドレスを計算するために使う
    let mut obj_section_map: ObjSectionMap = HashMap::new();
    // PE Optional Header に書き込む値を集計する変数
    // (.text 等コードセクションの合計バイト数)
    let mut size_of_code: u32 = 0;
    // (.data / .rdata 等、初期値ありデータセクションの合計バイト数)
    let mut size_of_initialized_data: u32 = 0;
    // (.bss 等、初期値なしデータセクションの合計バイト数。ファイル上には存在しない)
    let mut size_of_uninitialized_data: u32 = 0;
    // 最初のコードセクションの先頭 RVA (image_base からの相対アドレス)
    let mut base_of_code: u32 = 0;
    // 最初のデータセクションの先頭 RVA
    let mut base_of_data: u32 = 0;

    // マージ済みセクションを1つずつ処理し、PE上のアドレス・オフセットを確定する。
    // 各イテレーションで行うこと:
    //   1. ファイル上のサイズ (size_of_raw_data) とオフセット (pointer_to_raw_data) を決定
    //   2. obj→PE位置のマッピング (obj_section_map) に登録
    //   3. PE Optional Header 用の集計値 (size_of_code 等) を更新
    //   4. SectionHeader と PeSection を構築して pe_sections に追加
    //   5. 次のセクションのために virtual_address を進める
    for section in &sections {
        let kind = section.kind();

        // --- 1. ファイル上のサイズ・オフセットを決定 ---
        let virtual_size = section.data.len().max(4) as u32;
        // .bss はファイル上に実体を持たないので raw_data サイズ・オフセットともに 0
        let (size_of_raw_data, pointer_to_raw_data) = if kind == SectionKind::Bss {
            (0, 0)
        } else {
            // セクションのデータサイズを file_alignment の倍数に切り上げる
            // NOTE: 旧コード
            // NOTE: let raw_size = (section.data.len() as u32 / opts.file_alignment + 1) * opts.file_alignment;
            let raw_size =
                (section.data.len() as u32).div_ceil(opts.file_alignment) * opts.file_alignment;
            let ptr = raw_address;
            raw_address += raw_size;
            (raw_size, ptr)
        };

        // --- 2. obj→PE位置のマッピングに登録 ---
        let sec_index = pe_sections.len();
        let section_name = name_to_str(&section.name);

        for obj_sec_ref in &section.obj_sections {
            obj_section_map.insert(
                (obj_sec_ref.obj_index, section_name.clone()),
                PeSectionPosition {
                    section_index: sec_index,
                    offset: obj_sec_ref.offset,
                },
            );
        }

        // --- 3. PE Optional Header 用の集計値を更新 ---
        match kind {
            SectionKind::Code => {
                size_of_code += size_of_raw_data;
                if base_of_code == 0 {
                    base_of_code = virtual_address;
                }
            }
            SectionKind::Data => {
                size_of_initialized_data += size_of_raw_data;
                if base_of_data == 0 {
                    base_of_data = virtual_address;
                }
            }
            SectionKind::Bss => size_of_uninitialized_data += virtual_size,
            SectionKind::Other => {}
        }

        let aligned_virtual_size =
            align_up(section.data.len() as u32, opts.section_alignment).max(opts.section_alignment);

        // --- 4. SectionHeader と PeSection を構築 ---
        // PE ではリロケーション/ライン番号フィールドは使わないので全て 0
        let header = SectionHeader {
            name: section.name,
            virtual_size,
            virtual_address,
            size_of_raw_data,
            pointer_to_raw_data,
            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,
            characteristics: section.characteristics,
        };

        pe_sections.push(PeSection {
            header,
            // .bss はファイル上に実体なし
            data: if kind == SectionKind::Bss {
                Vec::new()
            } else {
                section.data.clone()
            },
        });

        // --- 5. 次のセクションの仮想アドレスを進める ---
        virtual_address += aligned_virtual_size;
    }

    SectionLayout {
        pe_sections,
        obj_section_map,
        size_of_headers,
        size_of_code,
        size_of_initialized_data,
        size_of_uninitialized_data,
        base_of_code,
        base_of_data,
    }
}
