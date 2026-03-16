//! COFF FileHeader — 20 バイト固定
//!
//! C++ 版: `include/FileHeader.hpp`

use std::fs::File;
use std::io::BufReader;

use crate::binary_io::ReadExt;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

pub mod machine {
    pub const I386: u16 = 0x014c;
}

impl FileHeader {
    pub const SIZE: u32 = 20;

    pub fn read(r: &mut BufReader<File>) -> Result<Self> {
        Ok(Self {
            machine: r.read_u16_le()?,
            number_of_sections: r.read_u16_le()?,
            time_date_stamp: r.read_u32_le()?,
            pointer_to_symbol_table: r.read_u32_le()?,
            number_of_symbols: r.read_u32_le()?,
            size_of_optional_header: r.read_u16_le()?,
            characteristics: r.read_u16_le()?,
        })
    }
}
