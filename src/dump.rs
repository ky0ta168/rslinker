//! デバッグ用ダンプ関数
//!
//! リンカの各処理ステップで生成されるデータ構造を人間が読める形式で表示する。

use crate::linker::dll::LoadedDll;
use crate::linker::import::ImportResult;
use crate::linker::section::SectionLayout;
use crate::linker::symbol::SymbolTable;

/// Stage 4: SectionLayout の内容を表示する
///
/// - ヘッダ/コード/データのサイズ
/// - PE セクション一覧 (仮想アドレス・ファイルオフセット・サイズ)
/// - obj→PE 位置マッピング一覧
pub fn dump_layout(layout: &SectionLayout) {
    println!("=== SectionLayout ===");
    println!("  size_of_headers       : 0x{:08X}", layout.size_of_headers);
    println!("  size_of_code          : 0x{:08X}", layout.size_of_code);
    println!(
        "  size_of_initialized   : 0x{:08X}",
        layout.size_of_initialized_data
    );
    println!(
        "  size_of_uninitialized : 0x{:08X}",
        layout.size_of_uninitialized_data
    );
    println!("  base_of_code          : 0x{:08X}", layout.base_of_code);
    println!("  base_of_data          : 0x{:08X}", layout.base_of_data);
    println!();

    println!("  pe_sections ({}):", layout.pe_sections.len());
    for (i, sec) in layout.pe_sections.iter().enumerate() {
        let h = &sec.header;
        println!(
            "    [{i}] {:<8}  VA=0x{:08X}  VirtualSize=0x{:04X}  \
             RawSize=0x{:04X}  RawOffset=0x{:08X}  data={}B",
            h.name_str(),
            h.virtual_address,
            h.virtual_size,
            h.size_of_raw_data,
            h.pointer_to_raw_data,
            sec.data.len(),
        );
    }
    println!();

    println!(
        "  obj_section_map ({} entries):",
        layout.obj_section_map.len()
    );
    let mut entries: Vec<_> = layout.obj_section_map.iter().collect();
    entries.sort_by_key(|((obj_idx, name), _)| (*obj_idx, name.clone()));
    for ((obj_idx, sec_name), pos) in &entries {
        println!(
            "    obj[{obj_idx}] {sec_name:<8}  -> pe_sections[{}] offset=0x{:08X}",
            pos.section_index, pos.offset,
        );
    }
    println!("=====================");
    println!();
}

/// Stage 5a: SymbolTable の内容を表示する
///
/// 各シンボルが PE のどのセクション・オフセットに解決されたかを示す。
pub fn dump_symbol_table(table: &SymbolTable) {
    println!("=== SymbolTable ({} symbols) ===", table.len());
    let mut entries: Vec<_> = table.iter().collect();
    entries.sort_by_key(|(name, _)| name.as_str());
    for (name, pos) in &entries {
        println!(
            "  {:<30}  -> pe_sections[{}] offset=0x{:08X}",
            name, pos.section_index, pos.offset,
        );
    }
    println!("================================");
    println!();
}

/// Stage 5b: ロード済み DLL 一覧を表示する
///
/// 各 DLL のパス・エクスポート関数数を表示する。
/// エクスポート関数名は先頭 10 件のみ表示し、多すぎる場合は省略する。
pub fn dump_loaded_dlls(dlls: &[LoadedDll]) {
    println!("=== LoadedDlls ({}) ===", dlls.len());
    for dll in dlls {
        let exports = dll.exports_sorted();
        println!("  {} ({})", dll.name, dll.path);
        println!("    exports: {}", dll.export_count());
        let show_count = exports.len().min(10);
        for name in &exports[..show_count] {
            println!("      {name}");
        }
        if exports.len() > 10 {
            println!("      ... ({} more)", exports.len() - 10);
        }
    }
    println!("======================");
    println!();
}

/// Stage 5c: ImportResult の内容を表示する
///
/// - インポートする DLL と関数名・IAT アドレス
/// - __imp__ シンボル → IAT RVA マッピング
/// - JMP スタブ → VA マッピング
/// - Import/IAT DataDirectory の位置とサイズ
pub fn dump_imports(imports: &ImportResult) {
    println!("=== ImportResult ===");

    println!("  dlls ({}):", imports.dlls.len());
    for dll in &imports.dlls {
        println!("    {}  ({} functions)", dll.name, dll.imports.len());
        for entry in &dll.imports {
            println!(
                "      {:<30}  IAT RVA=0x{:08X}",
                entry.function_name, entry.iat_rva,
            );
        }
    }
    println!();

    println!(
        "  imp_symbol_to_iat_rva ({} entries):",
        imports.imp_symbol_to_iat_rva.len()
    );
    let mut imp_entries: Vec<_> = imports.imp_symbol_to_iat_rva.iter().collect();
    imp_entries.sort_by_key(|(k, _)| k.as_str());
    for (sym, rva) in &imp_entries {
        println!("    {sym:<35}  -> IAT RVA=0x{rva:08X}");
    }
    println!();

    println!(
        "  symbol_to_jmp_va ({} entries):",
        imports.symbol_to_jmp_va.len()
    );
    let mut jmp_entries: Vec<_> = imports.symbol_to_jmp_va.iter().collect();
    jmp_entries.sort_by_key(|(k, _)| k.as_str());
    for (sym, va) in &jmp_entries {
        println!("    {sym:<35}  -> .dlljmp VA=0x{va:08X}");
    }
    println!();

    println!(
        "  import_dir : VA=0x{:08X}  size=0x{:X}",
        imports.import_dir.virtual_address, imports.import_dir.size,
    );
    println!(
        "  iat_dir    : VA=0x{:08X}  size=0x{:X}",
        imports.iat_dir.virtual_address, imports.iat_dir.size,
    );
    println!("====================");
    println!();
}
