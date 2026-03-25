# rslinker

Windows 32-bit PE リンカーの Rust 実装。

## 概要

COFF オブジェクトファイル (`.obj`) を読み込み、Windows 32-bit PE 実行ファイル (`.exe`) を生成するリンカー。

対象アーキテクチャ: x86 (i386)

## 使い方

```
cargo run -- [options] <obj_file>...
```

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `-out FILE` | 出力ファイル名 | `a.exe` |
| `-dll PATH` | リンクする DLL のパス (複数指定可) | kernel32 / msvcrt / user32 |

### 例

```bash
# 基本的な使い方
cargo run -- foo.obj bar.obj -out hello.exe

# カスタム DLL をリンクする
cargo run -- hello_dll.obj -dll mylib.dll -out hello_dll.exe
```

## サンプルのビルド

```bash
# 全サンプルをビルド
make

# 個別にビルド
make examples/hello
make examples/hello_dll

# 生成物を削除
make clean
```

| サンプル | 内容 |
|---------|------|
| `examples/hello/` | MessageBoxA を使う GUI アプリ |
| `examples/hello_dll/` | カスタム DLL (`mylib.dll`) を呼び出すアプリ |

## モジュール構成

```
src/
├── main.rs              エントリポイント・コマンドライン引数パース
├── error.rs             共通エラー型
├── binary_io.rs         バイナリ読み書きユーティリティ (ReadExt / WriteExt)
├── types.rs             共通型定義
├── coff/
│   ├── file_header.rs       COFF FileHeader (20 bytes)
│   ├── section_header.rs    COFF SectionHeader (40 bytes)
│   ├── symbol.rs            COFF シンボルテーブルエントリ
│   └── object_file.rs       COFF オブジェクトファイル全体・リロケーションエントリ
├── pe/
│   ├── dos_header.rs        DOS Header (64 bytes)
│   ├── pe_header.rs         PE Header (signature + FileHeader + OptionalHeader)
│   ├── optional_header.rs   OptionalHeader32 / DataDirectory
│   └── pe_file.rs           PE ファイル全体・書き出し
└── linker/
    ├── options.rs       リンカオプション
    ├── section.rs       セクションマージ・レイアウト計算
    ├── symbol.rs        グローバルシンボルテーブル構築
    ├── dll.rs           DLL エクスポートテーブル読み込み・シンボル検索
    ├── import.rs        .dlljmp / .idata セクション生成
    └── relocation.rs    リロケーション適用
```

## 実装状況

| Stage | 内容 | 状態 |
|-------|------|------|
| 1 | 基盤: `error.rs` / `binary_io.rs` / `types.rs` | ✅ |
| 2 | COFF パーサ: `coff/` | ✅ |
| 3 | PE 構造体+ライタ: `pe/` | ✅ |
| 4 | リンカ前半: セクションマージ・レイアウト | ✅ |
| 5 | リンカ後半: DLL 検索・シンボル解決 | ✅ |
| 6 | 仕上げ: リロケーション適用・PE 出力 | ✅ |
