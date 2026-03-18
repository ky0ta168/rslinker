#![allow(dead_code)]

mod binary_io;
mod coff;
mod error;
mod linker;
mod pe;
mod types;

use coff::object_file::ObjectFile;
use linker::options::LinkerOptions;
use linker::section::merge_and_layout;

fn main() {
    // --- Stage 4 動作確認: セクションマージ＆レイアウト ---
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
    let layout = merge_and_layout(&obj_files, &opts);

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
}
