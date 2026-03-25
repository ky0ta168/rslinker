# rslinker プロジェクト

C++ で書かれた Windows 32bit PE リンカー (`reference/spell/`) を Rust で書き直す学習プロジェクト。

## 実装ルール

- **ステップを1つ実装したら必ず止まり、ユーザーに確認してから次へ進む**
- 一度に複数のステップを実装しない
- **各ステップ完了時に `main.rs` に動作確認コードを追加する**
  - 例: パース機能を実装したらパース結果をダンプする
  - 次のステップに進む際は前のステップの確認コードを削除してよい

## ユーザーレベル

- プログラミングは書けるが、アライメントや PE 構造などの低レイヤー知識は初めて
- コメントや説明は「プログラム中級者・低レイヤー初心者」向けにする
  - 「バイト単位で何ブロック確保するか」のような具体例付きの説明が有効

## 実装方針

- バイナリ読み込みは `read_u16_le()` 等を使った**手動読み込み**で統一する
  - `zerocopy` / `bytemuck` 等は使わない (理解優先)

## 実装ステップ

| Stage | 内容 | 状態 |
|-------|------|------|
| 1 | 基盤: `types.rs` / `error.rs` / `binary_io.rs` | ✅ 完了 |
| 2 | COFF パーサ: `coff/file_header.rs` / `section_header.rs` / `symbol.rs` / `object_file.rs` | ✅ 完了 |
| 3 | PE 構造体+ライタ: `pe/dos_header.rs` / `optional_header.rs` / `pe_header.rs` / `pe_file.rs` | ✅ 完了 |
| 4 | リンカ前半: `linker/options.rs` / `linker/section.rs` (セクションマージ・レイアウト) | ✅ 完了 |
| 5 | リンカ後半: `linker/import.rs` / `linker/dll.rs` / `linker/symbol.rs` (DLL 検索・シンボル解決) | ✅ 完了 |
| 6 | 仕上げ: `linker/relocation.rs` / エントリポイント設定 / PE 出力 / `main.rs` 完成 | ✅ 完了 |

## テスト用オブジェクトファイルの作成

動作確認には 32-bit x86 の `.obj` が必要。ソースは `examples/` に置き、以下でコンパイルする。
`.obj` はコンパイル生成物なので `.gitignore` に追加してコミットしない。

```bash
export PATH="/c/msys64/mingw32/bin:$PATH"
cd examples
i686-w64-mingw32-gcc -c foo.c -o foo.obj
i686-w64-mingw32-gcc -c bar.c -o bar.obj
```

## 参照コード

`reference/spell/` に C++ 版の実装がある。
- `src/main.cpp` — リンクパイプライン全体 (10 ステップ)
- `include/*.hpp` — 各種ヘッダ定義
