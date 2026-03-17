//! DOS Header — 64 バイト固定 (e_lfanew = 0x40 固定)
//!
//! C++ 版: `include/DosHeader.hpp`

use std::fs::File;
use std::io::BufWriter;

use crate::binary_io::WriteExt;
use crate::error::Result;

pub struct DosHeader {
    pub magic_number: u16,                  // e_magic  : 0x5a4d ("MZ")
    pub last_page_size: u16,                // e_cblp
    pub number_of_pages: u16,               // e_cp
    pub number_of_relocation_entries: u16,  // e_crlc
    pub header_size: u16,                   // e_cparhdr
    pub min_extra_memory: u16,              // e_minalloc
    pub max_extra_memory: u16,              // e_maxalloc
    pub stack_segment: u16,                 // e_ss
    pub stack_pointer: u16,                 // e_sp
    pub checksum: u16,                      // e_csum
    pub instruction_pointer: u16,           // e_ip
    pub code_segment: u16,                  // e_cs
    pub relocation_table_offset: u16,       // e_lfarlc
    pub overlay_number: u16,                // e_ovno
    pub reserved1: [u8; 8],                 // e_res
    pub oem_id: u16,                        // e_oemid
    pub oem_info: u16,                      // e_oeminfo
    pub reserved2: [u8; 20],                // e_res2
    pub pe_header_offset: u32,              // e_lfanew
}

impl Default for DosHeader {
    fn default() -> Self {
        Self {
            magic_number: 0x5a4d,
            last_page_size: 0x90,
            number_of_pages: 0x03,
            number_of_relocation_entries: 0,
            header_size: 0x4,
            min_extra_memory: 0,
            max_extra_memory: 0xffff,
            stack_segment: 0,
            stack_pointer: 0xb8,
            checksum: 0,
            instruction_pointer: 0,
            code_segment: 0,
            relocation_table_offset: 0x40,
            overlay_number: 0,
            reserved1: [0; 8],
            oem_id: 0,
            oem_info: 0,
            reserved2: [0; 20],
            pe_header_offset: 0x40,
        }
    }
}

impl DosHeader {
    /// バイト列上のサイズ (e_lfanew 固定値と一致)
    pub const SIZE: u32 = 0x40;

    pub fn write(&self, w: &mut BufWriter<File>) -> Result<()> {
        w.write_u16_le(self.magic_number)?;
        w.write_u16_le(self.last_page_size)?;
        w.write_u16_le(self.number_of_pages)?;
        w.write_u16_le(self.number_of_relocation_entries)?;
        w.write_u16_le(self.header_size)?;
        w.write_u16_le(self.min_extra_memory)?;
        w.write_u16_le(self.max_extra_memory)?;
        w.write_u16_le(self.stack_segment)?;
        w.write_u16_le(self.stack_pointer)?;
        w.write_u16_le(self.checksum)?;
        w.write_u16_le(self.instruction_pointer)?;
        w.write_u16_le(self.code_segment)?;
        w.write_u16_le(self.relocation_table_offset)?;
        w.write_u16_le(self.overlay_number)?;
        w.write_bytes(&self.reserved1)?;
        w.write_u16_le(self.oem_id)?;
        w.write_u16_le(self.oem_info)?;
        w.write_bytes(&self.reserved2)?;
        w.write_u32_le(self.pe_header_offset)?;
        Ok(())
    }
}
