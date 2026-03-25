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
// Step 4: ソート (コード→初期化済みデータ→未初期化データ)
// ---------------------------------------------------------------------------

fn section_sort_key(characteristics: u32) -> u32 {
    if characteristics & ch::CONTAINS_CODE != 0 {
        return 0;
    }
    if characteristics & ch::CONTAINS_INITIALIZED_DATA != 0 {
        return 1;
    }
    if characteristics & ch::CONTAINS_UNINITIALIZED_DATA != 0 {
        return 2;
    }
    3
}

// ---------------------------------------------------------------------------
// Step 4: レイアウト計算
// ---------------------------------------------------------------------------

/// セクションをマージ・ソートし、PE上のアドレス/オフセットを確定する。
pub fn merge_and_layout(obj_files: &[ObjectFile], opts: &LinkerOptions) -> SectionLayout {
    // Step 3
    let mut sections = merge_sections(obj_files);
    // Step 4: ソート
    sections.sort_by_key(|s| section_sort_key(s.characteristics));

    // ヘッダ領域のサイズを計算 (import用に+2セクション分を予約)
    let max_sections = (sections.len() + 2) as u32;
    let header_bytes = DosHeader::SIZE + PeHeader::SIZE + SectionHeader::SIZE * max_sections;
    // header_bytes バイトのヘッダを file_alignment バイト単位で何ブロック確保するか
    // 例: header_bytes=530, file_alignment=512 → 2ブロック → size_of_headers=1024
    let size_of_headers = header_bytes.div_ceil(opts.file_alignment) * opts.file_alignment;
    let mut raw_address = size_of_headers;
    let mut virtual_address =
        (size_of_headers / opts.section_alignment + 1) * opts.section_alignment;

    let mut pe_sections: Vec<PeSection> = Vec::new();
    let mut obj_section_map: ObjSectionMap = HashMap::new();
    let mut size_of_code: u32 = 0;
    let mut size_of_initialized_data: u32 = 0;
    let mut size_of_uninitialized_data: u32 = 0;
    let mut base_of_code: u32 = 0;
    let mut base_of_data: u32 = 0;

    for section in &sections {
        let is_bss = section.characteristics & ch::CONTAINS_UNINITIALIZED_DATA != 0;
        let is_code = section.characteristics & ch::CONTAINS_CODE != 0;
        let is_data = section.characteristics & ch::CONTAINS_INITIALIZED_DATA != 0;

        let virtual_size = section.data.len().max(4) as u32;
        let (size_of_raw_data, pointer_to_raw_data) = if is_bss {
            (0, 0)
        } else {
            let raw_size =
                (section.data.len() as u32 / opts.file_alignment + 1) * opts.file_alignment;
            let ptr = raw_address;
            raw_address += raw_size;
            (raw_size, ptr)
        };

        let sec_index = pe_sections.len();
        let section_name = name_to_str(&section.name);

        // objSectionMap に登録
        for obj_sec_ref in &section.obj_sections {
            obj_section_map.insert(
                (obj_sec_ref.obj_index, section_name.clone()),
                PeSectionPosition {
                    section_index: sec_index,
                    offset: obj_sec_ref.offset,
                },
            );
        }

        // sizeOf* の集計
        if is_code {
            size_of_code += size_of_raw_data;
            if base_of_code == 0 {
                base_of_code = virtual_address;
            }
        } else if is_data {
            size_of_initialized_data += size_of_raw_data;
            if base_of_data == 0 {
                base_of_data = virtual_address;
            }
        } else if is_bss {
            size_of_uninitialized_data += virtual_size;
        }

        let aligned_virtual_size =
            align_up(section.data.len() as u32, opts.section_alignment).max(opts.section_alignment);

        let mut header = SectionHeader {
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
        // PE ではリロケーション/ライン番号フィールドは 0
        header.pointer_to_relocations = 0;

        pe_sections.push(PeSection {
            header,
            data: if is_bss {
                Vec::new()
            } else {
                section.data.clone()
            },
        });

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
