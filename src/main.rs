#![allow(dead_code)]

mod binary_io;
mod coff;
mod error;
mod types;

use coff::object_file::ObjectFile;

fn main() {
    // --- Stage 2 動作確認: COFF オブジェクトファイルのダンプ ---
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: rslinker <obj_file>");
        std::process::exit(1);
    });
    let obj = ObjectFile::from_file(&path).expect("failed to read obj");

    // FileHeader
    println!("=== FileHeader ===");
    println!("  machine            : {:#06x}", obj.file_header.machine);
    println!(
        "  number_of_sections : {}",
        obj.file_header.number_of_sections
    );
    println!(
        "  number_of_symbols  : {}",
        obj.file_header.number_of_symbols
    );

    // Sections
    println!("\n=== Sections ({}) ===", obj.sections.len());
    for sec in &obj.sections {
        println!(
            "  [{:8}]  va={:#010x}  raw_size={:#08x}  relocs={}",
            sec.header.name_str(),
            sec.header.virtual_address,
            sec.header.size_of_raw_data,
            sec.header.number_of_relocations,
        );
        for r in &sec.relocations {
            println!(
                "    reloc  offset={:#010x}  sym_idx={}  type={:?}",
                r.virtual_address, r.symbol_index, r.reloc_type
            );
        }
    }

    // Symbols
    println!("\n=== Symbols ({}) ===", obj.symbols.len());
    for entry in &obj.symbols {
        let s = &entry.symbol;
        println!(
            "  sc={:3}  sec={:5}  val={:#010x}  {}",
            s.storage_class,
            s.section_number,
            s.value,
            s.resolve_name(&obj.string_table)
        );
    }

    // String table
    println!("\n=== String Table ({}) ===", obj.string_table.len());
    let mut entries: Vec<_> = obj.string_table.iter().collect();
    entries.sort_by_key(|&(k, _)| k);
    for (offset, name) in entries {
        println!("  [{offset:4}] {name}");
    }
}
