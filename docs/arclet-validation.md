# Arclet の確認記録

確認日: 2026-11-01。比較元は `feature/node-graph-gcode` の
`a309790ac8e265b918da09474a02b069609ab3b5`。この記録は開発版の確認範囲を示す。

## 確認した環境

- Linux x86_64、Rust 1.99.1、`ci` ビルドプロファイル。実画面は Xvfb と Mesa lavapipe の Vulkan ソフトウェア描画。
- ブラウザ向けは `wasm32-unknown-unknown` を対象にした clippy の静的なコード検査。実ブラウザの起動確認とは区別する。
- 計算用テストは小さな解析例、敵対的入力、保存と取り消し、中止を含む。

## 結果

| 確認 | 結果 | 範囲 |
| --- | --- | --- |
| 変更した Rust ファイルの rustfmt | 61 ファイル成功 | vendor の Rust 5 ファイルは上流原本と一致。既存ファイルの対象外の書式は変更しない |
| `cargo xtask assets` | 成功 | 素材の出典 |
| `cargo xtask icons` | 成功 | 図柄規則、参照、17 個のツールテスト。localhost を許可した環境で実行 |
| `cargo xtask licences --check` | 成功 | 配布条件。生成済み一覧の更新は既存の生成ワークフローに任せる |
| `cargo xtask layers` | 成功 | 21 クレート、依存層の違反なし |
| `cargo xtask ui-containers` | 成功 | 外枠は共有 UI 部品を使う |
| native all-targets clippy、警告をエラーにする設定 | 成功 | Arclet / UI / Engine / Kernel / Geom / Graph / Slice の 7 パッケージ |
| wasm all-targets clippy、警告をエラーにする設定 | 成功 | Arclet、依存する UI・エンジン・幾何処理を含む |
| 新しい NURBS 曲線のテスト | 11 個成功 | 有理円と 3 階微分、パラメータ変更に依存しない端点判定、Rebuild、偏差、境界値、中止 |
| スライサー・グラフのテスト | 46 個成功 | Slice 21 個、Graph 25 個。スキン、層別充填率、JSON、上限、中止、ノードごとの失敗を含む |
| デスクトップの対象 UI テスト | 39 個成功 | Node UI 11 個、node workspace 8 個、curve quality 4 個、surface quality 6 個、host paths 9 個、Finish 1 個 |
| 曲面カーネルの関連テスト | 20 個成功 | Fair 13 個、Match 7 個。中止・再試行、境界保持、G0 の位置条件を含む |
| エンジンの新しい曲線コマンド・スキーマ | 8 個成功 | Rebuild・Undo・保存後の再読込、中止時の不変性、方向付き連続性、厳密な JSON 入力・出力契約 |
| Arclet アプリの native テスト | 20 個成功 | 実際のフレームでの Model / Nodes 切り替え・Settings、Rebuild サンプル、コマンド別名、保存先・ファイル上限 |
| 独立アプリの起動と実画面 | 4 例成功 | Model / Rebuild / Nodes / Zebra。1440×900 の実画面を保存し、すべて終了コード 0。配布バイナリで最終再確認 |

対象の自動テストは合計 **144 個成功**。これは変更に関係する範囲の件数であり、全テストを実行したという意味ではない。

対象 UI テストの実行結果は、配布資料の `review/ui-test-results.json` に記録した。
Surface UI の途中の失敗はテスト用形状・条件を修正し、全 6 個の成功を確認した。
中止、結果を採用しないこと、再試行を確認する条件は維持し、この修正で本体の処理は変更していない。
上記 39 個には、Arclet アプリ自体のテストや実画面の確認は含めない。

実起動で名前付きフォントの初期化漏れを発見し、本番とアプリのフレームテストを同じ初期化へ統一して修正した。
ノード画面の固定サイズと未描画の背景も修正し、1440×900 → 1920×1080 → 960×720 の
リサイズ回帰テストと、配布バイナリの再撮影で確認した。
起動・終了時間の記録は描画確認のためのもので、GPU 性能のベンチマークではない。

native と wasm の検査は、それぞれ新しい target ディレクトリから実施した。古い検査出力の再利用とは区別する。

wasm の厳しい検査:

```sh
cargo clippy -p arclet --all-targets --target wasm32-unknown-unknown \
  --profile ci --locked --offline -- -D warnings
```

## リポジトリ全体の既存の失敗

`cargo xtask ci` は実行したが、最初の `cargo fmt --all -- --check` で止まる。
比較元のままでも、`apps/meltcad/src/bench.rs` と `apps/meltcad/src/precise_services.rs` の
書式が検査に一致しない。今回これらのファイルは変更していない。

`cargo xtask lists` も、比較元と今回の両方で既存の `de.json`、`en.json`、`ja.json`、`zh.json`
の並び順検査に失敗する。新しい文字列は `assets/locales/pending/` に追加した。
利用者の指示に従い、既存カタログの並べ替えは行わない。

したがって「リポジトリ全体の CI が通った」とは報告しない。

## 未確認・未対応

- Windows / macOS の実機での起動、Intel 内蔵 GPU の性能。
- 実ブラウザの起動・描画・アップロード・ダウンロード。wasm の検査成功とは区別する。
- ブラウザの新しい重い曲線・曲面ツールと Print ノードは利用を拒否する。
  既存 CAD コマンドすべての非同期化も完了していない。
- Bambu A1 / A1 mini の実機印刷、専用開始・終了処理、AMS 用 `.gcode.3mf`。
- 一般曲面全体の G3 証明、Class A 品質の認証、xNURBS の互換性、任意メッシュの PolySplines 変換。

曲線端の測定とサーフェスの継ぎ目の標本測定を混同しない。
Rebuild の偏差は標本からの推定で、全域の最大誤差を保証する値ではない。
