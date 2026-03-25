#![allow(dead_code)]

mod binary_io;
mod coff;
mod error;
mod linker;
mod pe;
mod types;

use std::time::{SystemTime, UNIX_EPOCH};

use coff::object_file::ObjectFile;
use error::LinkerError;
use linker::dll::load_dll;
use linker::import::build_imports;
use linker::options::LinkerOptions;
use linker::relocation::apply_relocations;
use linker::section::merge_and_layout;
use linker::symbol::build_symbol_table;
use pe::dos_header::DosHeader;
use pe::optional_header::{DataDirectory, OptionalHeader32, dd, subsystem};
use pe::pe_file::PeFile;
use pe::pe_header::PeHeader;
use coff::file_header::{FileHeader, machine};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: rslinker [-dll DLL] [-out FILE] <obj_file>...");
        std::process::exit(1);
    }

    // コマンドライン引数のパース
    let mut opts = LinkerOptions::default();
    let mut obj_paths: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-dll" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -dll requires a path argument");
                    std::process::exit(1);
                }
                opts.dll_paths.push(args[i].clone());
            }
            "-out" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: -out requires a path argument");
                    std::process::exit(1);
                }
                opts.output_file = args[i].clone();
            }
            path => {
                obj_paths.push(path.to_string());
            }
        }
        i += 1;
    }

    if obj_paths.is_empty() {
        eprintln!("error: no obj files specified");
        std::process::exit(1);
    }

    let obj_files: Vec<ObjectFile> = obj_paths
        .iter()
        .map(|p| ObjectFile::from_file(p).expect("failed to read obj"))
        .collect();

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

    // Stage 6a: リロケーションの適用
    apply_relocations(&obj_files, &mut layout, &symbol_table, &imports, &opts)
        .expect("failed to apply relocations");

    // Stage 6b: エントリポイントの設定
    let entry_pos = symbol_table
        .get(&opts.entry_point)
        .ok_or_else(|| LinkerError::EntryPointNotFound(opts.entry_point.clone()))
        .expect("entry point not found");
    let address_of_entry_point =
        layout.pe_sections[entry_pos.section_index].header.virtual_address + entry_pos.offset;

    // Stage 6c: SizeOfImage の計算
    // 各セクションを section_alignment ブロック単位で何ブロック使うかを合算し、
    // ヘッダ領域分を加える。
    // 例: raw_size=0x200, section_alignment=0x1000 → 1ブロック → 0x1000
    let size_of_image: u32 = layout
        .pe_sections
        .iter()
        .map(|s| (s.header.size_of_raw_data / opts.section_alignment + 1) * opts.section_alignment)
        .sum::<u32>()
        + (layout.size_of_headers / opts.section_alignment + 1) * opts.section_alignment;

    // Stage 6d: FileHeader の構築
    let file_header = FileHeader {
        machine: machine::I386,
        number_of_sections: layout.pe_sections.len() as u16,
        time_date_stamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0),
        pointer_to_symbol_table: 0,
        number_of_symbols: 0,
        size_of_optional_header: OptionalHeader32::SIZE as u16,
        // RELOCS_STRIPPED(0x0001) | EXECUTABLE(0x0002) | 32BIT(0x0100) | DEBUG_STRIPPED(0x0200)
        characteristics: 0x0001 | 0x0002 | 0x0100 | 0x0200,
    };

    // Stage 6e: OptionalHeader の構築
    let mut data_directories = [DataDirectory::default(); 16];
    data_directories[dd::IMPORT] = imports.import_dir;
    data_directories[dd::IAT] = imports.iat_dir;

    let optional_header = OptionalHeader32 {
        magic_number: 0x010b, // PE32
        major_linker_version: 1,
        minor_linker_version: 0,
        size_of_code: layout.size_of_code,
        size_of_initialized_data: layout.size_of_initialized_data,
        size_of_uninitialized_data: layout.size_of_uninitialized_data,
        address_of_entry_point,
        base_of_code: layout.pe_sections[0].header.virtual_address,
        base_of_data: layout.base_of_data,
        image_base: opts.image_base,
        section_alignment: opts.section_alignment,
        file_alignment: opts.file_alignment,
        major_operating_system_version: 4,
        minor_operating_system_version: 0,
        major_image_version: 1,
        minor_image_version: 0,
        major_subsystem_version: 4,
        minor_subsystem_version: 0,
        win32_version_value: 0,
        size_of_image,
        size_of_headers: layout.size_of_headers,
        check_sum: 0,
        subsystem: subsystem::WINDOWS_CUI,
        dll_characteristics: 0,
        size_of_stack_reserve: opts.stack_reserve,
        size_of_stack_commit: opts.stack_commit,
        size_of_heap_reserve: opts.heap_reserve,
        size_of_heap_commit: opts.heap_commit,
        loader_flags: 0,
        number_of_rva_and_sizes: 16,
        data_directories,
    };

    // Stage 6f: PeFile の組み立て＆書き出し
    let pe_file = PeFile {
        dos_header: DosHeader::default(),
        pe_header: PeHeader {
            signature: 0x0000_4550, // "PE\0\0"
            file_header,
            optional_header,
        },
        sections: layout.pe_sections,
    };

    pe_file
        .write_to_file(&opts.output_file)
        .expect("failed to write PE file");

    println!("wrote {}", opts.output_file);
}
