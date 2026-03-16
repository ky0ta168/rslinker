//! COFF SectionHeader — 40 バイト固定
//!
//! C++ 版: `include/SectionHeader.hpp`

use std::fs::File;
use std::io::BufReader;

use crate::binary_io::ReadExt;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct SectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_line_numbers: u32,
    pub number_of_relocations: u16,
    pub number_of_line_numbers: u16,
    pub characteristics: u32,
}

/// characteristics のビットフラグ
pub mod ch {
    pub const CONTAINS_CODE: u32 = 0x0000_0020;
    pub const CONTAINS_INITIALIZED_DATA: u32 = 0x0000_0040;
    pub const CONTAINS_UNINITIALIZED_DATA: u32 = 0x0000_0080;
    pub const LINK_INFO: u32 = 0x0000_0200;
    pub const LINK_REMOVE: u32 = 0x0000_0800;
    pub const CAN_EXECUTE: u32 = 0x2000_0000;
    pub const CAN_READ: u32 = 0x4000_0000;
    pub const CAN_WRITE: u32 = 0x8000_0000;
}

impl SectionHeader {
    pub const SIZE: u32 = 40;

    pub fn read(r: &mut BufReader<File>) -> Result<Self> {
        Ok(Self {
            name: r.read_array::<8>()?,
            virtual_size: r.read_u32_le()?,
            virtual_address: r.read_u32_le()?,
            size_of_raw_data: r.read_u32_le()?,
            pointer_to_raw_data: r.read_u32_le()?,
            pointer_to_relocations: r.read_u32_le()?,
            pointer_to_line_numbers: r.read_u32_le()?,
            number_of_relocations: r.read_u16_le()?,
            number_of_line_numbers: r.read_u16_le()?,
            characteristics: r.read_u32_le()?,
        })
    }

    /// セクション名を文字列として返す (末尾の NUL を除去)
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(8);
        std::str::from_utf8(&self.name[..end]).unwrap_or("")
    }
}
