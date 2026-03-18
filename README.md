# rslinker

Windows 32-bit PE リンカーの Rust 実装。

## 概要

COFF オブジェクトファイル (`.obj`) を読み込み、Windows 32-bit PE 実行ファイル (`.exe`) を生成するリンカー。

対象アーキテクチャ: x86 (i386)

## 使い方

```
cargo run -- <obj_file>...
```

## モジュール構成

```
src/
├── main.rs          エントリポイント
├── error.rs         共通エラー型
├── binary_io.rs     バイナリ読み書きユーティリティ (ReadExt / WriteExt)
├── types.rs         共通型定義
└── coff/
    ├── file_header.rs    COFF FileHeader (20 bytes)
    ├── section_header.rs COFF SectionHeader (40 bytes)
    ├── symbol.rs         COFF シンボルテーブルエントリ
    └── object_file.rs    COFF オブジェクトファイル全体
```

## 実装状況

| Stage | 内容 |
|-------|------|
| 1 | 基盤: `error.rs` / `binary_io.rs` / `types.rs` ✅ |
| 2 | COFF パーサ: `coff/` ✅ |
| 3 | PE 構造体+ライタ: `pe/` ✅ |
| 4 | リンカ前半: セクションマージ・レイアウト ✅ |
| 5 | リンカ後半: DLL 検索・シンボル解決 |
| 6 | 仕上げ: リロケーション適用・PE 出力 |
