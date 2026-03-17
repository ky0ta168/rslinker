#![allow(dead_code)]

mod binary_io;
mod coff;
mod error;
mod pe;
mod types;

use pe::dos_header::DosHeader;
use pe::optional_header::OptionalHeader32;
use pe::pe_header::PeHeader;

fn main() {
    // --- Stage 3 動作確認: PE 構造体のサイズ確認 ---
    println!("DosHeader::SIZE        = {:#x} (expect 0x40)",  DosHeader::SIZE);
    println!("OptionalHeader32::SIZE = {:#x} (expect 0xe0)", OptionalHeader32::SIZE);
    println!("PeHeader::SIZE         = {:#x} (expect 0xf8)", PeHeader::SIZE);
}
