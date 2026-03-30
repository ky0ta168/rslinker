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
/// ## 何をするか
///
/// 複数の .obj ファイルに散らばっている「関数や変数の定義場所」を集めて、
/// 「シンボル名 → PE ファイル内のアドレス(セクション番号 + オフセット)」という
/// 辞書 (SymbolTable) を作る。
///
/// これがないとリロケーション処理で「`printf` ってどこにある?」が分からず、
/// 参照先アドレスを埋め込めない。
///
/// ## 対象シンボルの絞り込み
///
/// COFF シンボルテーブルには様々な種類のシンボルが入っているが、今回使うのは:
/// - `storage_class == EXTERNAL` — 他の .obj からも参照できる「公開された」シンボル
///   (関数定義、グローバル変数など)。スタティック関数やローカル変数は STATIC なので除外。
/// - `section_number > 0` — どこかのセクションに「実体がある」シンボル。
///   0 は未定義 (extern 宣言のみ)、-1 は絶対値シンボル (定数) なので除外。
///
/// ## PE 内アドレスの計算
///
/// COFF シンボルの `value` はそのシンボルが属するセクションの**先頭からのバイトオフセット**。
///
/// ```text
/// .obj の .text セクション
/// ┌─────────────────────────────┐
/// │ 0x00: ... (他の関数コード)    │
/// │ 0x10: add() の先頭           │← sym.value = 0x10
/// │  ...                        │
/// └─────────────────────────────┘
///         ↓ PE にマージ後
/// PE の .text セクション (先頭オフセット = pos.offset)
/// ┌──────────────────────────────────────┐
/// │ pos.offset + 0x10: add() の先頭       │
/// └──────────────────────────────────────┘
/// ```
///
/// よって PE 内での最終オフセットは `pos.offset + sym.value` になる。
/// `pos.section_index` はどの PE セクション (.text/.data など) かを示すインデックス。
///
/// ## エラー
///
/// 同名シンボルが複数 .obj に定義されていたら `DuplicateSymbol` エラーを返す
/// (C/C++ の「多重定義エラー」に相当)。
pub fn build_symbol_table(obj_files: &[ObjectFile], layout: &SectionLayout) -> Result<SymbolTable> {
    let mut table = SymbolTable::new();

    for (obj_index, obj) in obj_files.iter().enumerate() {
        for entry in &obj.symbols {
            let sym = &entry.symbol;

            // External かつ定義済み (section_number > 0) のみ対象
            if sym.storage_class != storage_class::EXTERNAL || sym.section_number == 0 {
                continue;
            }

            // section_number は 1-origin なので 0-origin に変換する
            let sec_idx = (sym.section_number as usize).wrapping_sub(1);
            if sec_idx >= obj.sections.len() {
                continue;
            }

            // このシンボルが属するセクション名 (".text", ".data" など) を取得する
            let section_name = obj.sections[sec_idx].header.name_str().to_string();

            // SectionLayout から「この .obj のこのセクションが PE のどこに配置されたか」を引く
            let Some(pos) = layout.obj_section_map.get(&(obj_index, section_name)) else {
                continue;
            };

            let name = sym.resolve_name(&obj.string_table).to_string();
            if name.is_empty() {
                continue;
            }

            // 同じ名前のシンボルが既に登録されていたら多重定義エラー
            if table.contains_key(&name) {
                return Err(LinkerError::DuplicateSymbol(name));
            }

            // PE 内でのアドレスを確定して登録する
            // offset = セクション先頭までのオフセット + セクション内でのシンボル位置
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
