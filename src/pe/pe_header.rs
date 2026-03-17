//! PE Header = "PE\0\0" シグネチャ + FileHeader + OptionalHeader32
//!
//! C++ 版: `include/PeHeader.hpp`

use std::fs::File;
use std::io::BufWriter;

use crate::binary_io::WriteExt;
use crate::coff::file_header::FileHeader;
use crate::error::Result;

use super::optional_header::OptionalHeader32;

pub struct PeHeader {
    /// "PE\0\0" = 0x00004550
    pub signature: u32,
    pub file_header: FileHeader,
    pub optional_header: OptionalHeader32,
}

impl PeHeader {
    /// バイト列上のサイズ: signature(4) + FileHeader(20) + OptionalHeader32(0xe0)
    pub const SIZE: u32 = 4 + FileHeader::SIZE + OptionalHeader32::SIZE; // 0xf8

    pub fn write(&self, w: &mut BufWriter<File>) -> Result<()> {
        w.write_u32_le(self.signature)?;
        self.file_header.write(w)?;
        self.optional_header.write(w)?;
        Ok(())
    }
}
