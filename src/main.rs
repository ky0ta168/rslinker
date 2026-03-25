#![allow(dead_code)]

mod binary_io;
mod coff;
mod error;
mod linker;
mod pe;
mod types;

use coff::object_file::ObjectFile;
use linker::dll::load_dll;
use linker::import::build_imports;
use linker::options::LinkerOptions;
use linker::section::merge_and_layout;
use linker::symbol::build_symbol_table;

fn main() {
    // --- Stage 5 動作確認: シンボル解決・DLL インポート ---
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: rslinker <obj_file>...");
        std::process::exit(1);
    }

    let obj_files: Vec<ObjectFile> = paths
        .iter()
        .map(|p| ObjectFile::from_file(p).expect("failed to read obj"))
        .collect();

    let opts = LinkerOptions::default();

    // Stage 4: セクションマージ＆レイアウト
    let mut layout = merge_and_layout(&obj_files, &opts);

    // Stage 5a: グローバルシンボルテーブルの構築
    let mut symbol_table =
        build_symbol_table(&obj_files, &layout).expect("failed to build symbol table");

    // Stage 5b: DLL のロード
    let loaded_dlls: Vec<_> = opts.dll_paths.iter().filter_map(|p| load_dll(p)).collect();

    // Stage 5c: DLL シンボル解決 + .dlljmp / .idata 生成
    let imports = build_imports(
        &obj_files,
        &mut layout,
        &mut symbol_table,
        &opts,
        &loaded_dlls,
    )
    .expect("failed to build imports");

    // -----------------------------------------------------------------------
    // 表示
    // -----------------------------------------------------------------------

    println!("=== Input Files ({}) ===", paths.len());
    for p in &paths {
        println!("  {}", p);
    }

    println!();
    println!(
        "SizeOfHeaders             = {:#010x}",
        layout.size_of_headers
    );
    println!("SizeOfCode                = {:#010x}", layout.size_of_code);
    println!(
        "SizeOfInitializedData     = {:#010x}",
        layout.size_of_initialized_data
    );
    println!(
        "SizeOfUninitializedData   = {:#010x}",
        layout.size_of_uninitialized_data
    );
    println!("BaseOfCode                = {:#010x}", layout.base_of_code);
    println!("BaseOfData                = {:#010x}", layout.base_of_data);

    println!("\n=== Merged Sections ({}) ===", layout.pe_sections.len());
    for sec in &layout.pe_sections {
        println!(
            "  [{:8}]  VA={:#010x}  FileOffset={:#010x}  RawSize={:#08x}",
            sec.header.name_str(),
            sec.header.virtual_address,
            sec.header.pointer_to_raw_data,
            sec.header.size_of_raw_data,
        );
    }

    println!("\n=== Symbols ({}) ===", symbol_table.len());
    let mut syms: Vec<(&String, u32)> = symbol_table
        .iter()
        .map(|(name, pos)| {
            let va = layout.pe_sections[pos.section_index].header.virtual_address + pos.offset;
            (name, va)
        })
        .collect();
    syms.sort_by_key(|(_, va)| *va);
    for (name, va) in &syms {
        println!("  {:#010x}  {}", va, name);
    }

    // AddressOfEntryPoint
    if let Some(pos) = symbol_table.get(&opts.entry_point) {
        let entry_va = layout.pe_sections[pos.section_index].header.virtual_address + pos.offset;
        println!(
            "\nAddressOfEntryPoint       = {:#010x}  ({})",
            entry_va, opts.entry_point
        );
    } else {
        println!(
            "\nAddressOfEntryPoint       = <not found: {}>",
            opts.entry_point
        );
    }

    // Imports
    if imports.dlls.is_empty() {
        println!("\n=== Imports (none) ===");
    } else {
        println!("\n=== Imports ({} DLL(s)) ===", imports.dlls.len());
        for dll in &imports.dlls {
            println!("  [{}]", dll.name);
            for entry in &dll.imports {
                println!(
                    "    IAT {:#010x}  {}",
                    opts.image_base + entry.iat_rva,
                    entry.function_name
                );
            }
        }
    }
}
