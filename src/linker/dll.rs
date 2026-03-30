//! DLL シンボル検索
//!
//! PE ファイルのエクスポートテーブルを直接読み込んでシンボルを検索する。
//! LoadLibraryA / GetProcAddress を使わないため、リンカ自身と
//! ビット数が異なる DLL (例: 64 ビットリンカで 32 ビット DLL) も扱える。
//!
//! C++ 版: main.cpp の tryFindDll 関数 (約 290〜343 行)

use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;

use crate::binary_io::ReadExt;

// ---------------------------------------------------------------------------
// ロード済み DLL
// ---------------------------------------------------------------------------

/// DLL のメタデータとエクスポートシンボル名一覧
pub struct LoadedDll {
    /// フルパス (例: "C:\\Windows\\System32\\kernel32.dll")
    pub path: String,
    /// ベース名 (例: "kernel32.dll")
    pub name: String,
    /// エクスポートされた関数名の集合
    exports: HashSet<String>,
}

impl LoadedDll {
    /// エクスポート関数の総数を返す
    pub fn export_count(&self) -> usize {
        self.exports.len()
    }

    /// エクスポート関数名をアルファベット順で返す
    pub fn exports_sorted(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.exports.iter().map(|s| s.as_str()).collect();
        v.sort_unstable();
        v
    }
}

/// DLL ファイルを開いてエクスポートテーブルを読み込む。失敗した場合は None を返す。
/// C++ 版の LoadLibraryA に相当する処理。
pub fn load_dll(path: &str) -> Option<LoadedDll> {
    let exports = read_exports(path).ok()?;
    let name = std::path::Path::new(path)
        .file_name()?
        .to_string_lossy()
        .into_owned();
    Some(LoadedDll {
        path: path.to_string(),
        name,
        exports,
    })
}

/// DLL に指定の関数名がエクスポートされているか調べる。
/// C++ 版の GetProcAddress に相当する処理。
fn has_proc(dll: &LoadedDll, name: &str) -> bool {
    dll.exports.contains(name)
}

// ---------------------------------------------------------------------------
// シンボル検索
// ---------------------------------------------------------------------------

/// `function_name` に対応するシンボルを持つ DLL を検索する。
///
/// 戻り値: `Some((real_name, dll_base_name))` — DLL 内での実際のエクスポート名と DLL ベース名。
///
/// 検索順:
/// 1. `function_name` そのまま
/// 2. `@N` などの非英数字サフィックスを除去した名前
/// 3. 先頭の `_` を順に除去した名前 (stdcall/cdecl マングリング対応)
pub fn try_find_dll<'a>(function_name: &str, dlls: &'a [LoadedDll]) -> Option<(String, &'a str)> {
    // 1. 完全一致
    for dll in dlls {
        if has_proc(dll, function_name) {
            return Some((function_name.to_string(), &dll.name));
        }
    }

    // 英数字とアンダースコアのみに切り詰める (例: _ExitProcess@4 → _ExitProcess)
    let stripped: String = function_name
        .chars()
        .take_while(|&c| c.is_alphanumeric() || c == '_')
        .collect();

    // 先頭の '_' を 1 つずつ剥がした候補リストを構築 (例: _printf → printf)
    let mut candidates: Vec<&str> = vec![stripped.as_str()];
    let mut i = 0;
    while stripped.as_bytes().get(i).copied() == Some(b'_') {
        i += 1;
        candidates.push(&stripped[i..]);
    }

    // 2. 候補名で検索
    for alt in &candidates {
        if *alt == function_name {
            continue; // 既にチェック済み
        }
        for dll in dlls {
            if has_proc(dll, alt) {
                return Some((alt.to_string(), &dll.name));
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// PE エクスポートテーブルの読み込み
// ---------------------------------------------------------------------------

/// (virtual_address, size_of_raw_data, pointer_to_raw_data)
type SectionInfo = (u32, u32, u32);

/// RVA をファイルオフセットに変換する。
/// 各セクションの (VA, 生データサイズ, ファイルオフセット) を使って変換する。
fn rva_to_offset(rva: u32, sections: &[SectionInfo]) -> Option<u32> {
    for &(va, size, ptr) in sections {
        if rva >= va && rva < va + size {
            return Some(rva - va + ptr);
        }
    }
    None
}

/// PE ファイルからエクスポートされた関数名の集合を読み込む。
///
/// PE32 (32 ビット) と PE32+ (64 ビット) の両方に対応する。
fn read_exports(path: &str) -> crate::error::Result<HashSet<String>> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);

    // DOS ヘッダ: e_lfanew (PE ヘッダへのオフセット) を取得
    // e_lfanew は DOS ヘッダの先頭から 0x3c バイト目にある
    r.set_position(0x3c)?;
    let pe_offset = r.read_u32_le()?;

    // PE シグネチャ "PE\0\0" を確認
    r.set_position(pe_offset as u64)?;
    let sig = r.read_u32_le()?;
    if sig != 0x0000_4550 {
        return Err(crate::error::LinkerError::InvalidFormat(format!(
            "{path}: not a PE file"
        )));
    }

    // COFF ファイルヘッダ (20 バイト) を読む
    let _machine = r.read_u16_le()?;
    let number_of_sections = r.read_u16_le()?;
    let _time_date_stamp = r.read_u32_le()?;
    let _pointer_to_symbol_table = r.read_u32_le()?;
    let _number_of_symbols = r.read_u32_le()?;
    let size_of_optional_header = r.read_u16_le()?;
    let _characteristics = r.read_u16_le()?;

    // オプショナルヘッダの先頭位置を記録
    let opt_header_offset = r.position()?;

    // magic でビット数を判別
    // 0x010b = PE32 (32 ビット), 0x020b = PE32+ (64 ビット)
    let magic = r.read_u16_le()?;

    // エクスポートディレクトリの RVA は DataDirectory[0].virtual_address にある。
    // DataDirectory の開始オフセット (オプショナルヘッダ先頭からの距離):
    //   PE32:  96 バイト目 (標準フィールド 28B + Windows 固有フィールド 68B)
    //   PE32+: 112 バイト目 (標準フィールド 24B + Windows 固有フィールド 88B)
    let data_dir_offset: u64 = match magic {
        0x010b => 96,
        0x020b => 112,
        _ => {
            return Err(crate::error::LinkerError::InvalidFormat(format!(
                "{path}: unknown optional header magic {magic:#06x}"
            )));
        }
    };
    r.set_position(opt_header_offset + data_dir_offset)?;
    let export_dir_rva = r.read_u32_le()?;
    let export_dir_size = r.read_u32_le()?;

    if export_dir_rva == 0 || export_dir_size == 0 {
        // エクスポートなし
        return Ok(HashSet::new());
    }

    // セクションヘッダを読む (オプショナルヘッダの直後)
    // セクションヘッダは 40 バイト固定
    r.set_position(opt_header_offset + size_of_optional_header as u64)?;
    let mut sections: Vec<SectionInfo> = Vec::new();
    for _ in 0..number_of_sections {
        let _name = r.read_bytes(8)?;
        let virtual_size = r.read_u32_le()?;
        let virtual_address = r.read_u32_le()?;
        let size_of_raw_data = r.read_u32_le()?;
        let pointer_to_raw_data = r.read_u32_le()?;
        r.read_bytes(16)?; // pointer_to_relocations 〜 characteristics (残り 16 バイト)
        let raw_size = if size_of_raw_data > 0 {
            size_of_raw_data
        } else {
            virtual_size
        };
        sections.push((virtual_address, raw_size, pointer_to_raw_data));
    }

    // エクスポートディレクトリをファイルオフセットに変換して読む
    let export_dir_offset = rva_to_offset(export_dir_rva, &sections).ok_or_else(|| {
        crate::error::LinkerError::InvalidFormat(format!(
            "{path}: export directory RVA {export_dir_rva:#010x} not found in sections"
        ))
    })?;

    // IMAGE_EXPORT_DIRECTORY (40 バイト)
    r.set_position(export_dir_offset as u64)?;
    r.read_bytes(24)?; // Characteristics 〜 Base (先頭 24 バイトはスキップ)
    let number_of_names = r.read_u32_le()?; // offset 24
    let _address_of_functions = r.read_u32_le()?; // offset 28
    let address_of_names = r.read_u32_le()?; // offset 32

    if number_of_names == 0 {
        return Ok(HashSet::new());
    }

    // Name Pointer Table: number_of_names 個の RVA (各 4 バイト)
    // 各 RVA は null 終端の関数名文字列を指す
    let names_offset = rva_to_offset(address_of_names, &sections).ok_or_else(|| {
        crate::error::LinkerError::InvalidFormat(format!(
            "{path}: name pointer table RVA not found"
        ))
    })?;

    r.set_position(names_offset as u64)?;
    let mut name_rvas: Vec<u32> = Vec::with_capacity(number_of_names as usize);
    for _ in 0..number_of_names {
        name_rvas.push(r.read_u32_le()?);
    }

    // 各関数名を読む
    let mut exports = HashSet::new();
    for name_rva in name_rvas {
        let Some(name_offset) = rva_to_offset(name_rva, &sections) else {
            continue;
        };
        r.set_position(name_offset as u64)?;
        let mut name = String::new();
        loop {
            let b = r.read_u8()?;
            if b == 0 {
                break;
            }
            name.push(b as char);
        }
        exports.insert(name);
    }

    Ok(exports)
}
