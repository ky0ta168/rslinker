//! COFF Symbol Table エントリ — 1 レコード 18 バイト固定
//!
//! C++ 版: `include/SymbolTableEntry.hpp`
//!
//! 構造:
//!   StandardSymbol (18 bytes) の直後に、
//!   numberOfAuxSymbols 個の AuxSymbol (各 18 bytes) が続く。
//!   AuxSymbol の解釈は StandardSymbol の storageClass と type で決まる。

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use crate::binary_io::ReadExt;
use crate::error::Result;

// ---------------------------------------------------------------------------
// 定数
// ---------------------------------------------------------------------------

pub mod storage_class {
    pub const EXTERNAL: u8 = 2;
    pub const STATIC: u8 = 3;
    pub const FUNCTION: u8 = 101;
    pub const FILE: u8 = 103;
}

pub mod sym_type {
    pub const IS_FUNCTION: u16 = 0x20;
}

pub mod special_section {
    pub const UNDEFINED: u16 = 0; // 外部シンボル (未定義)
    pub const ABSOLUTE: u16 = 0xFFFF; // 再配置不要な絶対値
    pub const DEBUG: u16 = 0xFFFE; // デバッグ情報
}

// ---------------------------------------------------------------------------
// Auxiliary Symbol の種別
// ---------------------------------------------------------------------------

/// 関数定義の補助シンボル
/// (storageClass=External, type=IsFunction, sectionNumber>0)
#[derive(Debug, Clone)]
pub struct AuxFunctionDef {
    pub tag_index: u32,
    pub total_size: u32,
    pub pointer_to_line_number: u32,
    pub pointer_to_next_function: u32,
    // 2 バイトパディング (読み捨て)
}

/// 関数開始/終了の補助シンボル (.bf / .ef)
/// (storageClass=Function)
#[derive(Debug, Clone)]
pub struct AuxFunctionBeginEnd {
    pub line_number: u16,
    pub pointer_to_next_function: u32,
}

/// Weak External の補助シンボル
/// (storageClass=External, sectionNumber=0, value=0)
#[derive(Debug, Clone)]
pub struct AuxWeakExternal {
    pub tag_index: u32,
    pub characteristics: u32,
}

/// ファイル名の補助シンボル
/// (storageClass=File)
#[derive(Debug, Clone)]
pub struct AuxFile {
    pub file_name: [u8; 18],
}

/// セクション定義の補助シンボル
/// (storageClass=Static)
#[derive(Debug, Clone)]
pub struct AuxSectionDef {
    pub length: u32,
    pub number_of_relocations: u16,
    pub number_of_line_numbers: u16,
    pub check_sum: u32,
    pub number: u16,
    pub selection: u8,
}

#[derive(Debug, Clone)]
pub enum AuxSymbol {
    FunctionDef(AuxFunctionDef),
    FunctionBeginEnd(AuxFunctionBeginEnd),
    WeakExternal(AuxWeakExternal),
    File(AuxFile),
    SectionDef(AuxSectionDef),
}

// ---------------------------------------------------------------------------
// Standard Symbol
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StandardSymbol {
    /// 名前 (8 バイト)
    /// 先頭 4 バイトが 0 なら、後半 4 バイトが文字列テーブルのオフセット
    pub name: [u8; 8],
    pub value: u32,
    pub section_number: u16,
    pub sym_type: u16,
    pub storage_class: u8,
    pub number_of_aux_symbols: u8,
}

impl StandardSymbol {
    /// シンボル名を解決する
    pub fn resolve_name<'a>(&'a self, string_table: &'a HashMap<u32, String>) -> &'a str {
        if self.name[..4] == [0, 0, 0, 0] {
            let offset = u32::from_le_bytes(self.name[4..8].try_into().unwrap());
            string_table.get(&offset).map(|s| s.as_str()).unwrap_or("")
        } else {
            let end = self.name.iter().position(|&b| b == 0).unwrap_or(8);
            std::str::from_utf8(&self.name[..end]).unwrap_or("")
        }
    }
}

// ---------------------------------------------------------------------------
// Symbol Table Entry (Standard + Auxiliary をまとめた単位)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub symbol: StandardSymbol,
    pub aux: Vec<AuxSymbol>,
}

// ---------------------------------------------------------------------------
// 読み込み
// ---------------------------------------------------------------------------

/// Standard Symbol 1 つとそれに続く Auxiliary Symbol を読み込む。
/// `index` は呼び出し元のループカウンタ。
/// Auxiliary Symbol の個数分だけ加算して返す (呼び出し元は +1 する)。
pub fn read_symbol_entry(r: &mut BufReader<File>, index: &mut u32) -> Result<SymbolEntry> {
    let sym = StandardSymbol {
        name: r.read_array::<8>()?,
        value: r.read_u32_le()?,
        section_number: r.read_u16_le()?,
        sym_type: r.read_u16_le()?,
        storage_class: r.read_u8()?,
        number_of_aux_symbols: r.read_u8()?,
    };

    *index += sym.number_of_aux_symbols as u32;

    let mut aux = Vec::with_capacity(sym.number_of_aux_symbols as usize);
    for _ in 0..sym.number_of_aux_symbols {
        aux.push(read_aux_symbol(r, &sym)?);
    }

    Ok(SymbolEntry { symbol: sym, aux })
}

fn read_aux_symbol(r: &mut BufReader<File>, sym: &StandardSymbol) -> Result<AuxSymbol> {
    use storage_class::*;
    use sym_type::*;

    if sym.storage_class == EXTERNAL && sym.sym_type == IS_FUNCTION && sym.section_number > 0 {
        // 関数定義
        let a = AuxFunctionDef {
            tag_index: r.read_u32_le()?,
            total_size: r.read_u32_le()?,
            pointer_to_line_number: r.read_u32_le()?,
            pointer_to_next_function: r.read_u32_le()?,
        };
        r.read_u16_le()?; // 2 バイトパディング
        Ok(AuxSymbol::FunctionDef(a))
    } else if sym.storage_class == FUNCTION {
        // .bf / .ef
        r.read_u32_le()?; // 4 バイトパディング
        let line_number = r.read_u16_le()?;
        r.read_bytes(6)?; // 6 バイトパディング
        let ptr = r.read_u32_le()?;
        r.read_u16_le()?; // 2 バイトパディング
        Ok(AuxSymbol::FunctionBeginEnd(AuxFunctionBeginEnd {
            line_number,
            pointer_to_next_function: ptr,
        }))
    } else if sym.storage_class == EXTERNAL
        && sym.section_number == special_section::UNDEFINED
        && sym.value == 0
    {
        // Weak External
        let a = AuxWeakExternal {
            tag_index: r.read_u32_le()?,
            characteristics: r.read_u32_le()?,
        };
        r.read_bytes(10)?; // 10 バイトパディング
        Ok(AuxSymbol::WeakExternal(a))
    } else if sym.storage_class == FILE {
        // ファイル名
        Ok(AuxSymbol::File(AuxFile {
            file_name: r.read_array::<18>()?,
        }))
    } else {
        // セクション定義 (storageClass=Static)
        let a = AuxSectionDef {
            length: r.read_u32_le()?,
            number_of_relocations: r.read_u16_le()?,
            number_of_line_numbers: r.read_u16_le()?,
            check_sum: r.read_u32_le()?,
            number: r.read_u16_le()?,
            selection: r.read_u8()?,
        };
        r.read_bytes(3)?; // 3 バイトパディング
        Ok(AuxSymbol::SectionDef(a))
    }
}
