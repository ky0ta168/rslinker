//! Step 8: リロケーションの適用
//!
//! 各 .obj のリロケーションエントリを走査し、マージ済みセクションデータ内の
//! プレースホルダーを実際の PE 仮想アドレスで書き換える。
//!
//! 参照先の種別:
//!   ① 内部シンボル      — 同 .obj 内の別セクション (+=)
//!   ② External シンボル  — 別 .obj のシンボル (+=)
//!   ③ DLL __imp__ シンボル — IAT エントリを直接参照 (=)
//!   ④ DLL JMP スタブ    — .dlljmp セクション経由 (=)
//!
//! ① ② は初期値にアドレスを加算 (+=)、③ ④ はアドレスで上書き (=)。
//! コンパイラが DLL 参照に対してはオペランドを 0 で出力するため、
//! += と = は同じ結果になるが、C++ 版に合わせて使い分けている。
//!
//! リロケーション種別:
//!   Dir32va  : 32bit 絶対仮想アドレス (target_rva + image_base)
//!   Dir32rva : 32bit 相対仮想アドレス (target_rva)
//!   Rel32    : CALL/JMP 用 32bit 相対変位 (target_rva - (operand_rva + 4))
//!              例: operand_rva=0x1005, target_rva=0x2000 → 0x2000-0x1009=0xFF7
//!
//! C++ 版: main.cpp Step 8 (約 710〜803 行)

use crate::coff::object_file::{ObjectFile, RelocType};
use crate::coff::symbol::{StandardSymbol, storage_class};
use crate::error::{LinkerError, Result};

use super::import::ImportResult;
use super::options::LinkerOptions;
use super::section::SectionLayout;
use super::symbol::SymbolTable;

/// Step 8: 全 .obj のリロケーションエントリを適用する。
///
/// `layout.pe_sections` のセクションデータを直接書き換える。
pub fn apply_relocations(
    obj_files: &[ObjectFile],
    layout: &mut SectionLayout,
    symbol_table: &SymbolTable,
    import_result: &ImportResult,
    opts: &LinkerOptions,
) -> Result<()> {
    for (obj_index, obj) in obj_files.iter().enumerate() {
        for obj_sec in &obj.sections {
            let section_name = obj_sec.header.name_str().to_string();

            // この obj セクションが配置された PE セクションの位置を取得
            let Some(changed_pos) = layout.obj_section_map.get(&(obj_index, section_name)) else {
                continue;
            };
            let changed_section_index = changed_pos.section_index;
            let changed_offset = changed_pos.offset;
            let changed_section_va =
                layout.pe_sections[changed_section_index].header.virtual_address;

            for reloc in &obj_sec.relocations {
                if reloc.reloc_type == RelocType::Absolute {
                    continue;
                }

                let Some(sym) = obj.symbol_by_raw_index(reloc.symbol_index as usize) else {
                    continue;
                };

                // 書き換え対象の 4 バイト位置 (セクションデータ内オフセット)
                let data_offset = (changed_offset + reloc.virtual_address) as usize;
                // 書き換え対象オペランド自身の RVA
                let operand_rva = changed_section_va + changed_offset + reloc.virtual_address;

                // 参照先 RVA と書き込み方式 (addend=true なら +=、false なら =) を決定
                let (addressed_rva, use_addend) =
                    resolve_symbol(obj_index, obj, sym, layout, symbol_table, import_result)?;

                let current =
                    read_i32(&layout.pe_sections[changed_section_index].data, data_offset);

                let new_val: i32 = match reloc.reloc_type {
                    RelocType::Dir32va => {
                        // 絶対仮想アドレス: RVA + image_base
                        let va = (addressed_rva + opts.image_base) as i32;
                        if use_addend { current + va } else { va }
                    }
                    RelocType::Dir32rva => {
                        // 相対仮想アドレス: RVA のみ
                        let rva = addressed_rva as i32;
                        if use_addend { current + rva } else { rva }
                    }
                    RelocType::Rel32 => {
                        // CALL/JMP 用相対変位: target_rva - (operand_rva + 4)
                        // 例: CALL 命令 (E8 xx xx xx xx) のオペランドは次命令からの距離
                        let rel = addressed_rva as i32 - operand_rva as i32 - 4;
                        if use_addend { current + rel } else { rel }
                    }
                    RelocType::Absolute => unreachable!(),
                    RelocType::Other(t) => {
                        return Err(LinkerError::InvalidFormat(format!(
                            "unsupported relocation type: {t:#06x}"
                        )));
                    }
                };

                write_i32(
                    &mut layout.pe_sections[changed_section_index].data,
                    data_offset,
                    new_val,
                );
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ヘルパー
// ---------------------------------------------------------------------------

/// シンボルを解決して `(addressed_rva, use_addend)` を返す。
///
/// - `use_addend=true`  → `+=` で書き込む (内部シンボル・別 obj シンボル)
/// - `use_addend=false` → `=`  で書き込む (DLL __imp__ シンボル・JMP スタブ)
fn resolve_symbol(
    obj_index: usize,
    obj: &ObjectFile,
    sym: &StandardSymbol,
    layout: &SectionLayout,
    symbol_table: &SymbolTable,
    import_result: &ImportResult,
) -> Result<(u32, bool)> {
    let sym_name = sym.resolve_name(&obj.string_table).to_string();

    if sym.storage_class != storage_class::EXTERNAL {
        // ① 内部シンボル: シンボル名 = 参照先セクション名 (例: ".text", ".data")
        let Some(pos) = layout.obj_section_map.get(&(obj_index, sym_name.clone())) else {
            return Err(LinkerError::UndefinedSymbol(sym_name));
        };
        let rva = layout.pe_sections[pos.section_index].header.virtual_address + pos.offset;
        Ok((rva, true))
    } else if let Some(pos) = symbol_table.get(&sym_name) {
        // ② 別 .obj のシンボル
        let rva = layout.pe_sections[pos.section_index].header.virtual_address + pos.offset;
        Ok((rva, true))
    } else if let Some(&iat_rva) = import_result.imp_symbol_to_iat_rva.get(&sym_name) {
        // ③ DLL __imp__ シンボル: IAT エントリの RVA を直接使う
        Ok((iat_rva, false))
    } else if let Some(&jmp_rva) = import_result.symbol_to_jmp_va.get(&sym_name) {
        // ④ .dlljmp スタブ経由
        Ok((jmp_rva, false))
    } else {
        Err(LinkerError::UndefinedSymbol(sym_name))
    }
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn write_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
