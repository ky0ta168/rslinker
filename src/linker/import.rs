//! Step 6〜7: DLL シンボル解決 + .dlljmp / .idata セクション生成
//!
//! Step 6: リロケーションエントリを走査し、未定義の External シンボルを
//!         ロード済み DLL から検索する。
//! Step 7: DLL 呼び出し用の 2 セクションを生成する。
//!   .dlljmp : FF 25 <IAT addr> の間接 JMP スタブを並べたコードセクション
//!   .idata  : Import Directory + ILT + IAT + Hint/Name テーブル
//! C++ 版: main.cpp Step 6〜7 (約 480〜701 行)

use std::collections::HashMap;

use crate::binary_io::align_up;
use crate::coff::object_file::ObjectFile;
use crate::coff::section_header::{SectionHeader, ch};
use crate::coff::symbol::storage_class;
use crate::error::{LinkerError, Result};
use crate::pe::optional_header::DataDirectory;
use crate::pe::pe_file::PeSection;

use super::dll::{LoadedDll, try_find_dll};
use super::options::LinkerOptions;
use super::section::SectionLayout;
use super::symbol::SymbolTable;

// ---------------------------------------------------------------------------
// 公開型
// ---------------------------------------------------------------------------

/// インポートされた 1 関数のエントリ
pub struct ImportEntry {
    pub function_name: String,
    pub iat_rva: u32,
}

/// 1 DLL のインポート情報
pub struct ImportDll {
    pub name: String, // DLL ベース名 (例: "kernel32.dll")
    pub imports: Vec<ImportEntry>,
}

/// build_imports の戻り値
pub struct ImportResult {
    /// 表示用: DLL ごとのインポート一覧
    pub dlls: Vec<ImportDll>,
    /// Stage 6 (リロケーション) 用: __imp__Foo → IAT RVA
    pub imp_symbol_to_iat_rva: HashMap<String, u32>,
    /// Stage 6 (リロケーション) 用: symbol_name → .dlljmp VA
    pub symbol_to_jmp_va: HashMap<String, u32>,
    /// PE DataDirectory: Import
    pub import_dir: DataDirectory,
    /// PE DataDirectory: IAT
    pub iat_dir: DataDirectory,
}

// ---------------------------------------------------------------------------
// 内部データ構造
// ---------------------------------------------------------------------------

/// Import Directory の中間データ (1 DLL)
struct DllImportData {
    name: String,           // DLL ベース名
    functions: Vec<String>, // 実関数名 (インポート順)
    ilt_rva: u32,           // ILT 先頭 RVA (計算後)
    iat_rva: u32,           // IAT 先頭 RVA (= ilt_rva + total_ilt_size)
    name_rva: u32,          // DLL 名文字列の RVA (計算後)
}

// ---------------------------------------------------------------------------
// メイン関数
// ---------------------------------------------------------------------------

/// Steps 6〜7: DLL シンボルを解決し、.dlljmp / .idata セクションを構築する。
///
/// - `layout` はインプレースで更新される (.dlljmp 挿入によるオフセット修正)
/// - `symbol_table` はインプレースで更新される (section_index のシフト)
pub fn build_imports(
    obj_files: &[ObjectFile],
    layout: &mut SectionLayout,
    symbol_table: &mut SymbolTable,
    opts: &LinkerOptions,
    dlls: &[LoadedDll],
) -> Result<ImportResult> {
    // -----------------------------------------------------------------------
    // Step 6: リロケーションエントリを走査して DLL シンボルを収集する
    // -----------------------------------------------------------------------

    // __imp__ シンボル名 → 実関数名 (例: "__imp__printf" → "printf")
    let mut imp_symbol_to_real_name: HashMap<String, String> = HashMap::new();
    // 実関数名 → .dlljmp スロットインデックス (JMP スタブ経由の関数のみ)
    let mut jmp_real_name_to_slot: HashMap<String, usize> = HashMap::new();
    // symbol_name → 実関数名 (JMP スタブ経由の場合)
    let mut symbol_to_real_name: HashMap<String, String> = HashMap::new();
    // DLL ベース名 → dll_data 配列インデックス
    let mut dll_name_to_idx: HashMap<String, usize> = HashMap::new();
    let mut dll_data: Vec<DllImportData> = Vec::new();

    for obj in obj_files {
        for section in &obj.sections {
            for reloc in &section.relocations {
                // symbol_index は Aux を含む COFF raw インデックス
                let Some(sym) = obj.symbol_by_raw_index(reloc.symbol_index as usize) else {
                    continue;
                };
                if sym.storage_class != storage_class::EXTERNAL {
                    continue;
                }
                let sym_name = sym.resolve_name(&obj.string_table).to_string();
                if sym_name.is_empty() || symbol_table.contains_key(&sym_name) {
                    continue; // obj 内で定義済み → DLL 不要
                }

                if let Some(stripped) = sym_name.strip_prefix("__imp__") {
                    // __declspec(dllimport) が生成した IAT 直接参照シンボル
                    if imp_symbol_to_real_name.contains_key(&sym_name) {
                        continue; // 既に処理済み
                    }
                    // @N サフィックスも除去
                    let mut real_name = stripped.to_string();
                    if let Some(at) = real_name.find('@') {
                        real_name.truncate(at);
                    }
                    let (found_name, dll_base) = try_find_dll(&real_name, dlls)
                        .ok_or_else(|| LinkerError::UndefinedSymbol(sym_name.clone()))?;
                    imp_symbol_to_real_name.insert(sym_name, found_name.clone());
                    add_function_to_dll(&found_name, dll_base, &mut dll_name_to_idx, &mut dll_data);
                } else {
                    // JMP スタブ経由の参照 (__declspec(dllimport) なし)
                    if symbol_to_real_name.contains_key(&sym_name) {
                        continue; // 既に処理済み
                    }
                    let (found_name, dll_base) = try_find_dll(&sym_name, dlls)
                        .ok_or_else(|| LinkerError::UndefinedSymbol(sym_name.clone()))?;
                    symbol_to_real_name.insert(sym_name, found_name.clone());
                    if !jmp_real_name_to_slot.contains_key(&found_name) {
                        let slot = jmp_real_name_to_slot.len();
                        jmp_real_name_to_slot.insert(found_name.clone(), slot);
                        add_function_to_dll(
                            &found_name,
                            dll_base,
                            &mut dll_name_to_idx,
                            &mut dll_data,
                        );
                    }
                }
            }
        }
    }

    let has_jmp_stubs = !jmp_real_name_to_slot.is_empty();
    let has_dll_imports = !dll_data.is_empty();

    if !has_dll_imports {
        return Ok(ImportResult {
            dlls: Vec::new(),
            imp_symbol_to_iat_rva: HashMap::new(),
            symbol_to_jmp_va: HashMap::new(),
            import_dir: DataDirectory::default(),
            iat_dir: DataDirectory::default(),
        });
    }

    // -----------------------------------------------------------------------
    // Step 7a: .dlljmp セクションを先頭に挿入 (JMP スタブが存在する場合)
    // -----------------------------------------------------------------------
    // .dlljmp の仮想アドレス: 既存セクションを押しのけて先頭に来る
    let jmp_section_va = opts.section_alignment;

    if has_jmp_stubs {
        let num_stubs = jmp_real_name_to_slot.len();
        let jmp_data_size = (num_stubs * 6) as u32; // 各スタブ: FF 25 + 4 バイト IAT addr
        let jmp_raw_size = (jmp_data_size / opts.file_alignment + 1) * opts.file_alignment;
        let jmp_mem_size = (jmp_data_size / opts.section_alignment + 1) * opts.section_alignment;

        // 既存セクションのアドレスをシフト
        for sec in &mut layout.pe_sections {
            sec.header.pointer_to_raw_data += jmp_raw_size;
            sec.header.virtual_address += jmp_mem_size;
        }
        layout.size_of_code += jmp_raw_size;
        layout.base_of_data += jmp_mem_size;

        // obj_section_map と symbol_table のセクションインデックスをシフト (+1)
        for pos in layout.obj_section_map.values_mut() {
            pos.section_index += 1;
        }
        for pos in symbol_table.values_mut() {
            pos.section_index += 1;
        }

        // .dlljmp セクションを先頭に挿入 (データは Step 7c で埋める)
        layout.pe_sections.insert(
            0,
            PeSection {
                header: SectionHeader {
                    name: str_to_name(".dlljmp"),
                    virtual_size: jmp_data_size,
                    virtual_address: jmp_section_va,
                    size_of_raw_data: jmp_raw_size,
                    pointer_to_raw_data: layout.size_of_headers,
                    pointer_to_relocations: 0,
                    pointer_to_line_numbers: 0,
                    number_of_relocations: 0,
                    number_of_line_numbers: 0,
                    characteristics: ch::CONTAINS_CODE | ch::CAN_READ | ch::CAN_EXECUTE,
                },
                data: vec![0u8; num_stubs * 6],
            },
        );
    }

    // -----------------------------------------------------------------------
    // Step 7b: .idata セクションのレイアウト計算
    // -----------------------------------------------------------------------
    let total_functions: usize = dll_data.iter().map(|d| d.functions.len()).sum();
    let num_dlls = dll_data.len();

    // .idata の仮想アドレス (現在の最終セクションの直後)
    let idata_va = next_virtual_address(layout, opts);

    // Import Directory Table: (num_dlls + 1) × 20 バイト (null エントリ込み)
    let dir_size = (num_dlls as u32 + 1) * 20;

    // ILT / IAT 全体サイズ: (totalFunctions + num_dlls) × 4 バイト
    // (各 DLL の関数エントリ + null ターミネータ 1 個)
    let ilt_iat_entries = (total_functions + num_dlls) as u32;
    let ilt_size = ilt_iat_entries * 4; // ILT の総バイト数 (= IAT の総バイト数)

    // ILT / IAT の後に続く Hint/Name テーブルの開始 RVA
    let hint_name_start_rva = idata_va + dir_size + ilt_size * 2; // ILT + IAT

    // 各 DLL の ILT/IAT 先頭 RVA を計算
    let mut ilt_cursor = idata_va + dir_size;
    for d in &mut dll_data {
        d.ilt_rva = ilt_cursor;
        d.iat_rva = ilt_cursor + ilt_size; // IAT は ILT ブロック全体の直後
        ilt_cursor += (d.functions.len() as u32 + 1) * 4; // +1 は null ターミネータ
    }

    // 各関数の Hint/Name エントリ RVA を計算
    let mut hint_name_rva = hint_name_start_rva;
    let mut function_to_hint_rva: HashMap<String, u32> = HashMap::new();
    for d in &dll_data {
        for func in &d.functions {
            function_to_hint_rva.insert(func.clone(), hint_name_rva);
            hint_name_rva += func.len() as u32 + 3; // 2 (hint) + len + 1 (null)
        }
    }

    // DLL 名の RVA を計算
    for d in &mut dll_data {
        d.name_rva = hint_name_rva;
        hint_name_rva += d.name.len() as u32 + 1;
    }

    let idata_size = hint_name_rva - idata_va;
    let idata_raw = (idata_size / opts.file_alignment + 1) * opts.file_alignment;
    let raw_cursor = next_raw_address(layout);

    // -----------------------------------------------------------------------
    // Step 7c: IAT RVA を確定し、imp / jmp マッピングを構築
    // -----------------------------------------------------------------------
    let mut function_name_to_iat_rva: HashMap<String, u32> = HashMap::new();
    for d in &dll_data {
        for (fi, func) in d.functions.iter().enumerate() {
            function_name_to_iat_rva.insert(func.clone(), d.iat_rva + fi as u32 * 4);
        }
    }

    // __imp__ シンボル → IAT RVA
    let mut imp_symbol_to_iat_rva: HashMap<String, u32> = HashMap::new();
    for (imp_sym, real_name) in &imp_symbol_to_real_name {
        if let Some(&iat_rva) = function_name_to_iat_rva.get(real_name) {
            imp_symbol_to_iat_rva.insert(imp_sym.clone(), iat_rva);
        }
    }

    // symbol_name → .dlljmp VA
    let mut symbol_to_jmp_va: HashMap<String, u32> = HashMap::new();
    for (sym_name, real_name) in &symbol_to_real_name {
        if let Some(&slot) = jmp_real_name_to_slot.get(real_name) {
            symbol_to_jmp_va.insert(sym_name.clone(), jmp_section_va + slot as u32 * 6);
        }
    }

    // -----------------------------------------------------------------------
    // Step 7d: .dlljmp データを埋める (FF 25 + IAT 絶対仮想アドレス)
    // -----------------------------------------------------------------------
    if has_jmp_stubs {
        let dlljmp = &mut layout.pe_sections[0];
        for (real_name, &slot) in &jmp_real_name_to_slot {
            if let Some(&iat_rva) = function_name_to_iat_rva.get(real_name) {
                let pos = slot * 6;
                dlljmp.data[pos] = 0xFF;
                dlljmp.data[pos + 1] = 0x25;
                let iat_va = opts.image_base + iat_rva;
                dlljmp.data[pos + 2..pos + 6].copy_from_slice(&iat_va.to_le_bytes());
            }
        }
    }

    // -----------------------------------------------------------------------
    // Step 7e: .idata セクションデータを構築
    // -----------------------------------------------------------------------
    let mut idata_data = vec![0u8; idata_size as usize];

    // Import Directory Table を書き込む
    let mut dir_offset = 0usize;
    for d in &dll_data {
        write_u32(&mut idata_data, dir_offset, d.ilt_rva);
        write_u32(&mut idata_data, dir_offset + 4, 0); // timeDateStamp
        write_u32(&mut idata_data, dir_offset + 8, 0); // forwarderChain
        write_u32(&mut idata_data, dir_offset + 12, d.name_rva);
        write_u32(&mut idata_data, dir_offset + 16, d.iat_rva);
        dir_offset += 20;
    }
    // null ターミネータ (20 バイト) はゼロ初期化済みのため不要

    // ILT / IAT と Hint/Name テーブルを書き込む
    for d in &dll_data {
        for (fi, func) in d.functions.iter().enumerate() {
            let hint_rva = function_to_hint_rva[func];
            // ILT エントリ
            write_u32(
                &mut idata_data,
                (d.ilt_rva - idata_va) as usize + fi * 4,
                hint_rva,
            );
            // IAT エントリ (初期値は ILT と同じ; ロード時に上書きされる)
            write_u32(
                &mut idata_data,
                (d.iat_rva - idata_va) as usize + fi * 4,
                hint_rva,
            );
            // Hint/Name テーブル: hint(2B,0) + name + null
            let hn_off = (hint_rva - idata_va) as usize;
            // hint は 0 (ゼロ初期化済み)
            idata_data[hn_off + 2..hn_off + 2 + func.len()].copy_from_slice(func.as_bytes());
        }
        // DLL 名文字列
        let name_off = (d.name_rva - idata_va) as usize;
        idata_data[name_off..name_off + d.name.len()].copy_from_slice(d.name.as_bytes());
    }

    // .idata セクションを追加
    layout.pe_sections.push(PeSection {
        header: SectionHeader {
            name: str_to_name(".idata"),
            virtual_size: idata_size,
            virtual_address: idata_va,
            size_of_raw_data: idata_raw,
            pointer_to_raw_data: raw_cursor,
            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,
            characteristics: ch::CONTAINS_INITIALIZED_DATA | ch::CAN_READ | ch::CAN_WRITE,
        },
        data: idata_data,
    });
    layout.size_of_initialized_data += idata_raw;

    // -----------------------------------------------------------------------
    // DataDirectory エントリ
    // -----------------------------------------------------------------------
    let import_dir = DataDirectory {
        virtual_address: idata_va,
        size: idata_size,
    };
    // IAT DataDirectory: ILT ブロック先頭から ilt_size バイト分をカバー
    // (C++ 版と同じ: importAddressTableDirectory = {ILT_start, ilt_size})
    let iat_dir = DataDirectory {
        virtual_address: idata_va + dir_size,
        size: ilt_size,
    };

    // -----------------------------------------------------------------------
    // 戻り値の組み立て
    // -----------------------------------------------------------------------
    let result_dlls: Vec<ImportDll> = dll_data
        .into_iter()
        .map(|d| {
            let imports = d
                .functions
                .iter()
                .map(|f| ImportEntry {
                    function_name: f.clone(),
                    iat_rva: *function_name_to_iat_rva.get(f).unwrap_or(&0),
                })
                .collect();
            ImportDll {
                name: d.name,
                imports,
            }
        })
        .collect();

    Ok(ImportResult {
        dlls: result_dlls,
        imp_symbol_to_iat_rva,
        symbol_to_jmp_va,
        import_dir,
        iat_dir,
    })
}

// ---------------------------------------------------------------------------
// ヘルパー
// ---------------------------------------------------------------------------

/// DLL の関数リストに関数を追加する (重複は無視)
fn add_function_to_dll(
    func_name: &str,
    dll_base: &str,
    dll_name_to_idx: &mut HashMap<String, usize>,
    dll_data: &mut Vec<DllImportData>,
) {
    let idx = if let Some(&i) = dll_name_to_idx.get(dll_base) {
        i
    } else {
        let i = dll_data.len();
        dll_data.push(DllImportData {
            name: dll_base.to_string(),
            functions: Vec::new(),
            ilt_rva: 0,
            iat_rva: 0,
            name_rva: 0,
        });
        dll_name_to_idx.insert(dll_base.to_string(), i);
        i
    };
    if !dll_data[idx].functions.contains(&func_name.to_string()) {
        dll_data[idx].functions.push(func_name.to_string());
    }
}

/// 文字列を 8 バイトのセクション名配列に変換する
fn str_to_name(s: &str) -> [u8; 8] {
    let mut name = [0u8; 8];
    for (i, &b) in s.as_bytes().iter().take(8).enumerate() {
        name[i] = b;
    }
    name
}

/// バイト列の `offset` から 4 バイトに `value` をリトルエンディアンで書き込む
fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// 現在のレイアウトにセクションを追加する場合の次の仮想アドレスを求める
fn next_virtual_address(layout: &SectionLayout, opts: &LinkerOptions) -> u32 {
    layout
        .pe_sections
        .iter()
        .map(|s| {
            let aligned =
                align_up(s.header.virtual_size, opts.section_alignment).max(opts.section_alignment);
            s.header.virtual_address + aligned
        })
        .max()
        .unwrap_or(opts.section_alignment)
}

/// 現在のレイアウトにセクションを追加する場合の次のファイルオフセットを求める
fn next_raw_address(layout: &SectionLayout) -> u32 {
    layout
        .pe_sections
        .iter()
        .filter(|s| s.header.pointer_to_raw_data > 0)
        .map(|s| s.header.pointer_to_raw_data + s.header.size_of_raw_data)
        .max()
        .unwrap_or(0)
}
