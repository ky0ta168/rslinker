//! リンカオプション
//!
//! C++ 版: `ProgramOptions` (main.cpp)

use crate::pe::optional_header::subsystem;

pub struct LinkerOptions {
    /// メモリ上のセクションアライメント (ページサイズ以上、2のべき乗)
    pub section_alignment: u32,
    /// ファイル上のセクションアライメント (512〜65536、2のべき乗)
    pub file_alignment: u32,
    /// イメージベースアドレス
    pub image_base: u32,
    /// エントリポイントシンボル名
    pub entry_point: String,
    /// 出力ファイル名
    pub output_file: String,
    /// サブシステム
    pub subsystem: u16,
    pub stack_reserve: u32,
    pub stack_commit: u32,
    pub heap_reserve: u32,
    pub heap_commit: u32,
    /// DLL 検索パス (例: "C:\\Windows\\System32\\kernel32.dll")
    pub dll_paths: Vec<String>,
}

impl Default for LinkerOptions {
    fn default() -> Self {
        Self {
            section_alignment: 0x1000,
            file_alignment: 0x200,
            image_base: 0x400000,
            entry_point: "_main".to_string(),
            output_file: "a.exe".to_string(),
            subsystem: subsystem::WINDOWS_CUI,
            stack_reserve: 0x200000,
            stack_commit: 0x1000,
            heap_reserve: 0x100000,
            heap_commit: 0x1000,
            dll_paths: vec![
                r"C:\Windows\System32\kernel32.dll".to_string(),
                r"C:\Windows\System32\msvcrt.dll".to_string(),
                r"C:\Windows\System32\user32.dll".to_string(),
            ],
        }
    }
}
