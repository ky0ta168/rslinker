//! DLL シンボル検索
//!
//! Windows API (LoadLibraryA / GetProcAddress) を使って
//! シンボルが存在する DLL を探す。
//! C++ 版: main.cpp の tryFindDll 関数 (約 290〜343 行)

use std::ffi::CString;

use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::core::PCSTR;

// ---------------------------------------------------------------------------
// ロード済み DLL
// ---------------------------------------------------------------------------

/// ロード済み DLL のハンドルとメタデータ
pub struct LoadedDll {
    /// フルパス (例: "C:\\Windows\\System32\\kernel32.dll")
    pub path: String,
    /// ベース名 (例: "kernel32.dll")
    pub name: String,
    handle: HMODULE,
}

/// DLL をロードする。失敗した場合は None を返す。
pub fn load_dll(path: &str) -> Option<LoadedDll> {
    let cstr = CString::new(path).ok()?;
    let handle = unsafe { LoadLibraryA(PCSTR(cstr.as_ptr() as *const u8)).ok()? };
    let name = std::path::Path::new(path)
        .file_name()?
        .to_string_lossy()
        .into_owned();
    Some(LoadedDll {
        path: path.to_string(),
        name,
        handle,
    })
}

fn has_proc(dll: &LoadedDll, name: &str) -> bool {
    let Ok(cstr) = CString::new(name) else {
        return false;
    };
    unsafe { GetProcAddress(dll.handle, PCSTR(cstr.as_ptr() as *const u8)).is_some() }
}

// ---------------------------------------------------------------------------
// シンボル検索
// ---------------------------------------------------------------------------

/// `function_name` に対応するシンボルを持つ DLL を検索する。
///
/// 戻り値: `Some((real_name, dll_base_name))` — DLL 内での実際のエクスポート名と DLL ベース名。
///
/// 検索順:
/// 1. `function_name` そのまま
/// 2. `@N` などの非英数字サフィックスを除去した名前
/// 3. 先頭の `_` を順に除去した名前 (stdcall/cdecl マングリング対応)
pub fn try_find_dll<'a>(function_name: &str, dlls: &'a [LoadedDll]) -> Option<(String, &'a str)> {
    // 1. 完全一致
    for dll in dlls {
        if has_proc(dll, function_name) {
            return Some((function_name.to_string(), &dll.name));
        }
    }

    // 英数字とアンダースコアのみに切り詰める (例: _ExitProcess@4 → _ExitProcess)
    let stripped: String = function_name
        .chars()
        .take_while(|&c| c.is_alphanumeric() || c == '_')
        .collect();

    // 先頭の '_' を 1 つずつ剥がした候補リストを構築 (例: _printf → printf)
    let mut candidates: Vec<&str> = vec![stripped.as_str()];
    let mut i = 0;
    while stripped.as_bytes().get(i).copied() == Some(b'_') {
        i += 1;
        candidates.push(&stripped[i..]);
    }

    // 2. 候補名で検索
    for alt in &candidates {
        if *alt == function_name {
            continue; // 既にチェック済み
        }
        for dll in dlls {
            if has_proc(dll, alt) {
                return Some((alt.to_string(), &dll.name));
            }
        }
    }

    None
}
