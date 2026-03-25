//! COFF オブジェクトファイル全体の読み込み
//!
//! C++ 版: `include/ObjectFile.hpp`

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use crate::binary_io::ReadExt;
use crate::error::Result;

use super::{
    file_header::FileHeader,
    section_header::SectionHeader,
    symbol::{SymbolEntry, read_symbol_entry},
};

// ---------------------------------------------------------------------------
// リロケーションエントリ
// ---------------------------------------------------------------------------

/// Intel 386 向けリロケーション種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocType {
    Absolute, // 0x00: 無視
    Dir32va,  // 0x06: 32-bit 絶対仮想アドレス (RVA + imageBase)
    Dir32rva, // 0x07: 32-bit 相対仮想アドレス (RVA のみ)
    Rel32,    // 0x14: CALL/JMP 向け 32-bit 相対変位
    Other(u16),
}

impl From<u16> for RelocType {
    fn from(v: u16) -> Self {
        match v {
            0x00 => Self::Absolute,
            0x06 => Self::Dir32va,
            0x07 => Self::Dir32rva,
            0x14 => Self::Rel32,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RelocationEntry {
    /// セクション先頭からのオフセット (書き換え対象の位置)
    pub virtual_address: u32,
    /// シンボルテーブルの 0-based インデックス
    pub symbol_index: u32,
    pub reloc_type: RelocType,
}

impl RelocationEntry {
    pub fn read(r: &mut BufReader<File>) -> Result<Self> {
        Ok(Self {
            virtual_address: r.read_u32_le()?,
            symbol_index: r.read_u32_le()?,
            reloc_type: RelocType::from(r.read_u16_le()?),
        })
    }
}

// ---------------------------------------------------------------------------
// オブジェクトセクション
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ObjectSection {
    pub header: SectionHeader,
    pub data: Vec<u8>,
    pub relocations: Vec<RelocationEntry>,
}

// ---------------------------------------------------------------------------
// オブジェクトファイル
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ObjectFile {
    pub file_header: FileHeader,
    pub sections: Vec<ObjectSection>,
    pub symbols: Vec<SymbolEntry>,
    /// キー: 文字列テーブル内のバイトオフセット (4 始まり)
    pub string_table: HashMap<u32, String>,
}

impl ObjectFile {
    /// COFF raw シンボルテーブルインデックス (Aux を含む絶対インデックス) から
    /// 対応する StandardSymbol を返す。
    /// reloc.symbol_index はこの raw インデックスを指すため、このメソッドで解決する。
    pub fn symbol_by_raw_index(
        &self,
        raw_index: usize,
    ) -> Option<&crate::coff::symbol::StandardSymbol> {
        let mut cur = 0usize;
        for entry in &self.symbols {
            if cur == raw_index {
                return Some(&entry.symbol);
            }
            cur += 1 + entry.aux.len(); // standard(1) + aux の数
        }
        None
    }

    /// ファイルパスから COFF オブジェクトファイルを読み込む
    pub fn from_file(path: &str) -> Result<Self> {
        let f = File::open(path)?;
        let mut r = BufReader::new(f);
        Self::read(&mut r)
    }

    fn read(r: &mut BufReader<File>) -> Result<Self> {
        // 1. FileHeader
        let file_header = FileHeader::read(r)?;

        // 2. SectionHeader 群 (FileHeader の直後に連続する)
        let n = file_header.number_of_sections as usize;
        let mut sections: Vec<ObjectSection> = (0..n)
            .map(|_| {
                let h = SectionHeader::read(r)?;
                Ok(ObjectSection {
                    header: h,
                    data: Vec::new(),
                    relocations: Vec::new(),
                })
            })
            .collect::<Result<_>>()?;

        // 3. 各セクションのデータとリロケーションテーブル
        for sec in &mut sections {
            // セクションデータ
            r.set_position(sec.header.pointer_to_raw_data as u64)?;
            sec.data = r.read_bytes(sec.header.size_of_raw_data as usize)?;

            // リロケーション
            if sec.header.number_of_relocations > 0 {
                r.set_position(sec.header.pointer_to_relocations as u64)?;
                for _ in 0..sec.header.number_of_relocations {
                    sec.relocations.push(RelocationEntry::read(r)?);
                }
            }
        }

        // 4. シンボルテーブル
        r.set_position(file_header.pointer_to_symbol_table as u64)?;
        let total = file_header.number_of_symbols;
        let mut symbols = Vec::new();
        let mut i: u32 = 0;
        // iをAuxシンボルの個数分進める必要があるためwhileを採用
        while i < total {
            symbols.push(read_symbol_entry(r, &mut i)?);
            i += 1;
        }

        // 5. 文字列テーブル (シンボルテーブルの直後)
        let string_table = read_string_table(r)?;

        Ok(ObjectFile {
            file_header,
            sections,
            symbols,
            string_table,
        })
    }
}

fn read_string_table(r: &mut BufReader<File>) -> Result<HashMap<u32, String>> {
    // 先頭 4 バイトはテーブル全体のサイズ (自身を含む)
    let size = r.read_u32_le()?;

    let mut table = HashMap::new();
    let mut offset: u32 = 4; // サイズフィールドの直後から

    while offset < size {
        let mut s = String::new();
        // NULL終端文字列を抽出
        loop {
            let b = r.read_u8()?;
            if b == 0 {
                break;
            }
            s.push(b as char);
        }
        let len = s.len() as u32;
        if !s.is_empty() {
            table.insert(offset, s);
        }
        offset += len + 1;
    }
    Ok(table)
}
