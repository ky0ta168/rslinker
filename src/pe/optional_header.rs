//! PE Optional Header (32-bit) — 224 バイト固定 (0xe0)
//!
//! C++ 版: `include/OptionalHeader32.hpp`, `include/DataDirectory.hpp`

use std::fs::File;
use std::io::BufWriter;

use crate::binary_io::WriteExt;
use crate::error::Result;

// ---------------------------------------------------------------------------
// DataDirectory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct DataDirectory {
    pub virtual_address: u32,
    pub size: u32,
}

impl DataDirectory {
    pub const SIZE: u32 = 8;

    pub fn write(&self, w: &mut BufWriter<File>) -> Result<()> {
        w.write_u32_le(self.virtual_address)?;
        w.write_u32_le(self.size)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// DataDirectory インデックス定数
// ---------------------------------------------------------------------------

pub mod dd {
    pub const EXPORT: usize = 0;
    pub const IMPORT: usize = 1;
    pub const RESOURCE: usize = 2;
    pub const BASE_RELOCATION: usize = 5;
    pub const IAT: usize = 12;
}

// ---------------------------------------------------------------------------
// OptionalHeader32
// ---------------------------------------------------------------------------

pub struct OptionalHeader32 {
    // Standard Fields
    pub magic_number: u16,                   // 0x10b = PE32
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,

    // Windows-Specific Fields
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,           // 予約。0 固定
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub check_sum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,                  // 予約。0 固定
    pub number_of_rva_and_sizes: u32,       // 常に 16

    pub data_directories: [DataDirectory; 16],
}

/// subsystem 定数
pub mod subsystem {
    pub const WINDOWS_GUI: u16 = 2;
    pub const WINDOWS_CUI: u16 = 3;
}

impl OptionalHeader32 {
    /// バイト列上のサイズ
    pub const SIZE: u32 = 96 + 16 * DataDirectory::SIZE; // 0xe0

    pub fn write(&self, w: &mut BufWriter<File>) -> Result<()> {
        w.write_u16_le(self.magic_number)?;
        w.write_u8(self.major_linker_version)?;
        w.write_u8(self.minor_linker_version)?;
        w.write_u32_le(self.size_of_code)?;
        w.write_u32_le(self.size_of_initialized_data)?;
        w.write_u32_le(self.size_of_uninitialized_data)?;
        w.write_u32_le(self.address_of_entry_point)?;
        w.write_u32_le(self.base_of_code)?;
        w.write_u32_le(self.base_of_data)?;
        w.write_u32_le(self.image_base)?;
        w.write_u32_le(self.section_alignment)?;
        w.write_u32_le(self.file_alignment)?;
        w.write_u16_le(self.major_operating_system_version)?;
        w.write_u16_le(self.minor_operating_system_version)?;
        w.write_u16_le(self.major_image_version)?;
        w.write_u16_le(self.minor_image_version)?;
        w.write_u16_le(self.major_subsystem_version)?;
        w.write_u16_le(self.minor_subsystem_version)?;
        w.write_u32_le(self.win32_version_value)?;
        w.write_u32_le(self.size_of_image)?;
        w.write_u32_le(self.size_of_headers)?;
        w.write_u32_le(self.check_sum)?;
        w.write_u16_le(self.subsystem)?;
        w.write_u16_le(self.dll_characteristics)?;
        w.write_u32_le(self.size_of_stack_reserve)?;
        w.write_u32_le(self.size_of_stack_commit)?;
        w.write_u32_le(self.size_of_heap_reserve)?;
        w.write_u32_le(self.size_of_heap_commit)?;
        w.write_u32_le(self.loader_flags)?;
        w.write_u32_le(self.number_of_rva_and_sizes)?;
        for dd in &self.data_directories {
            dd.write(w)?;
        }
        Ok(())
    }
}
