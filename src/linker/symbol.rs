//! Step 5: グローバルシンボルテーブルの構築
//!
//! 全 .obj の External シンボルを「シンボル名 → PE内アドレス」に変換して登録する。
//! C++ 版: main.cpp Step 5 (約 455〜479 行)

use std::collections::HashMap;

use crate::coff::object_file::ObjectFile;
use crate::coff::symbol::storage_class;
use crate::error::{LinkerError, Result};

use super::section::{PeSectionPosition, SectionLayout};

/// シンボル名 → PE セクション内位置
pub type SymbolTable = HashMap<String, PeSectionPosition>;

/// 全 .obj ファイルの External シンボルから SymbolTable を構築する。
///
/// - storage_class == EXTERNAL かつ section_number > 0 のシンボルが対象
/// - 重複シンボルはエラー
pub fn build_symbol_table(obj_files: &[ObjectFile], layout: &SectionLayout) -> Result<SymbolTable> {
    let mut table = SymbolTable::new();

    for (obj_index, obj) in obj_files.iter().enumerate() {
        for entry in &obj.symbols {
            let sym = &entry.symbol;

            // External かつ定義済み (section_number > 0) のみ対象
            if sym.storage_class != storage_class::EXTERNAL || sym.section_number == 0 {
                continue;
            }

            let sec_idx = (sym.section_number as usize).wrapping_sub(1);
            if sec_idx >= obj.sections.len() {
                continue;
            }

            let section_name = obj.sections[sec_idx].header.name_str().to_string();

            let Some(pos) = layout.obj_section_map.get(&(obj_index, section_name)) else {
                continue;
            };

            let name = sym.resolve_name(&obj.string_table).to_string();
            if name.is_empty() {
                continue;
            }

            if table.contains_key(&name) {
                return Err(LinkerError::DuplicateSymbol(name));
            }

            table.insert(
                name,
                PeSectionPosition {
                    section_index: pos.section_index,
                    offset: pos.offset + sym.value,
                },
            );
        }
    }

    Ok(table)
}
