# Arclet CAD

曲線と曲面を数値で確かめながら作る、独立した純 Rust 製の NURBS CAD です（開発版）。
An independent, pure-Rust NURBS CAD where curves and surfaces are checked numerically, not by eye.
Development build.

![status](https://img.shields.io/badge/status-development-orange)
![licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)

## できること / What it does

- **スケッチと履歴**: 拘束ソルバーつきスケッチ、フィーチャの履歴、B-rep カーネル。
- **ノード**: `Model` と `Nodes` を切り替え、ノードで形状を組み立てられます。
- **ライノ風の画面**: メニューバー（ファイル・編集・表示・曲線・サーフェス・ソリッド・
  メッシュ・変換・解析・レンダー・ツール・ヘルプ）、コマンド行（短縮名・候補・履歴・
  Enter で直前を繰り返す）、標準ツールバーと左ツールバー、Properties / Groups / Display
  パネル、ステータスバー、明るいクラシック調テーマ。
- **解析**: Zebra、曲率マップ、曲率コーム、曲線端／面の連続性、断面、アイソカーブ、
  干渉、クリアランス、最小半径、ドラフト、重心、ボディ比較、アクセシビリティ。
- **入出力**: 読み込み STEP・IGES・3DM（デスクトップのみ）・OBJ・glTF/GLB・STL・3MF・
  PLY/XYZ/PTS・DXF・SVG。書き出し STEP・IGES・OBJ・STL・3MF・DXF（現在のビュー）・
  URDF・設計 JSON。3DM は読み込みのみです。

## 起動 / Run

Rust 1.95 以降が必要です。リポジトリのルートで:

```sh
cargo run --release -p arclet
```

サンプル、ノード画面、曲線の Rebuild をすぐ試す場合:

```sh
cargo run --release -p arclet -- --sample
cargo run --release -p arclet -- --nodes
cargo run --release -p arclet -- --curve-sample
```

保存した設計は `File > Open`、または起動時の引数で開けます:

```sh
cargo run --release -p arclet -- path/to/model.sc
```

アプリの詳しい説明（保存場所、ブラウザ版、既知の制限）は
[apps/arclet/README.md](apps/arclet/README.md) にあります。

## いまの状態 / Status

開発版です。次のことは**約束しません**。

- Rhino、Plasticity、xNURBS など他社製品との互換性
- Class A 品質、G3 曲面の一般的な保証
- 3DM・GH/GHX・RhinoCommon・Python・.rhp/.gha プラグインの対応
- すべてのコマンドの計算結果の正しさ（到達できることと、正しいことは別です）

実際にできることの一覧は [COMPATIBILITY_MATRIX.md](COMPATIBILITY_MATRIX.md)、
ライノ級機能までの距離と調査は
[docs/rhino-coverage-research.md](docs/rhino-coverage-research.md)、
実行した検証と未検証の範囲は [TEST_RESULTS.md](TEST_RESULTS.md) にあります。

## ライセンス / Licence

[MIT](LICENSE-MIT) または [Apache-2.0](LICENSE-APACHE) のどちらかを選べます。
著作権表示は [NOTICE](NOTICE)、画像・アイコンの出典は [ATTRIBUTION.md](ATTRIBUTION.md)、
依存ライブラリは [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt) にあります。
**無保証**です（作品の消失や破損について作者は責任を負いません）。大切なデータは
こまめに別の場所にも保存してください。

## 独立していること / Independence and trademarks

- このリポジトリは、オープンソースの CAD **SolidCraft**
  （<https://github.com/tangency-labs/solidcraft>、MIT OR Apache-2.0）のソースを取り込み、
  その上に作った**別のプログラム**です。SolidCraft の作者とも、他の CAD ベンダーとも
  関係がなく、公式でも承認を受けたものでもありません。
- 商用 CAD や 3D ソフトのコード・アイコン・画像・文章・テンプレートは使っていません。
  アイコンはコードで描くか、独自に作ったものです。
- 他社の製品名（Rhino、FreeCAD、STEP など）は、読み書きできるファイル形式や比較対象を
  示すためだけに書いてあり、それぞれの持ち主の商標です。
- This project is independent of, and not affiliated with or endorsed by, any CAD vendor or
  the SolidCraft authors. All product names are trademarks of their owners and are used only
  to name file formats or the compared product.

*This is not legal advice.*

## このリポジトリの他のアプリ / Other apps in this tree

同じ Rust の CAD コアを使う別のアプリも入っています。

- [apps/meltcad](apps/meltcad/README.md) — Melt 3D（形がとけてくっつくモデリング）
- [apps/solidcraft](apps/solidcraft) — SolidCraft のデスクトップアプリ
- [apps/solidcraft-cli](apps/solidcraft-cli) — コマンドライン
- [apps/solidcraft-web](apps/solidcraft-web) — ブラウザ版
