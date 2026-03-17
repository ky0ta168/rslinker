//! PE ファイル全体の書き出し
//!
//! C++ 版: `include/PeFile.hpp`

use std::fs::File;
use std::io::{BufWriter, SeekFrom, Seek};

use crate::binary_io::WriteExt;

use crate::coff::section_header::SectionHeader;
use crate::error::Result;

use super::dos_header::DosHeader;
use super::pe_header::PeHeader;

pub struct PeSection {
    pub header: SectionHeader,
    pub data: Vec<u8>,
}

pub struct PeFile {
    pub dos_header: DosHeader,
    pub pe_header: PeHeader,
    pub sections: Vec<PeSection>,
}

impl PeFile {
    /// PE ファイルをパスに書き出す。
    pub fn write_to_file(&self, path: &str) -> Result<()> {
        let last = self.sections.last().expect("sections must not be empty");
        let file_size = last.header.pointer_to_raw_data + last.header.size_of_raw_data;

        // ファイルを作成してゼロ埋め
        let f = File::create(path)?;
        f.set_len(file_size as u64)?;
        let mut w = BufWriter::new(f);

        // ヘッダ群を先頭から順番に書く
        self.dos_header.write(&mut w)?;
        self.pe_header.write(&mut w)?;
        for sec in &self.sections {
            sec.header.write(&mut w)?;
        }

        // 各セクションのデータをオフセット位置に書く
        for sec in &self.sections {
            w.seek(SeekFrom::Start(sec.header.pointer_to_raw_data as u64))?;
            w.write_bytes(&sec.data)?;
        }

        Ok(())
    }
}
