# Arclet CAD

Arclet は、曲線と曲面を数値で確かめながら作る、独立した Rust 製 CAD アプリです。
`Model` と `Nodes` を切り替え、名前付きツール、コマンド入力、ノードによる形状作成を使います。
Melt3D とは別の実行ファイル・画面・設定を持ち、同じリポジトリの Rust 製 CAD コアを再利用します。

現在は開発版です。Rhino、Plasticity、xNURBS の互換実装や、Class A 品質を保証する製品としては公開していません。
今回の重点は、使える画面から **Spline → Rebuild → 連続性の検査 → 面の作成・調整** を往復できることです。

## 起動する

Rust 1.95 以降が必要です。以下はリポジトリのルートで実行します。

```sh
cargo run --release -p arclet
```

サンプルを表示する場合と、ノードから始める場合:

```sh
cargo run --release -p arclet -- --sample
cargo run --release -p arclet -- --nodes
```

曲線の Rebuild と、サンプルの Zebra 表示をすぐ試す場合:

```sh
cargo run --release -p arclet -- --curve-sample
cargo run --release -p arclet -- --sample --zebra
```

`--curve-sample` は実際の制御点スプラインを作り、その曲線を選択した Rebuild パネルを開きます。
`--sample --zebra` はリング形状に Zebra 表示を適用します。

保存した CAD 文書は画面の `File > Open` で開けます。起動時にパスを渡すこともできます。

```sh
cargo run --release -p arclet -- /path/to/model.sc
```

実行ファイルは `arclet`、ウィンドウ名は `Arclet CAD`、アプリ ID は `arclet-cad` です。
設定も Arclet 専用で、Melt3D の設定を書き換えません。
初期状態は英語表示と、中ボタンで回転するカメラ操作です。`Settings` から変更できます。
デスクトップの自動保存・復旧候補、クラッシュ記録の一覧、プラグイン一覧も Arclet 専用の保存先を使います。
Windows は `%LOCALAPPDATA%/Arclet`、macOS は `~/Library/Application Support/Arclet`、
Linux は `$XDG_DATA_HOME/arclet`（未指定時は `~/.local/share/arclet`）の各サブフォルダーです。
旧アプリの復旧データを自動で読み込んだり、移したりする処理はありません。
Linux の表示ライブラリなど、共有コアのビルドに必要な環境は
[CONTRIBUTING.md](../../CONTRIBUTING.md#get-it-running-about-10-minutes-on-a-fast-machine) を参照してください。
Linux のウィンドウは X11 / XWayland を使います。Arclet 単体のビルドでは、
別の配布条件を持つ Plasma のプロトコル資料を含む依存を避けるため、ネイティブ Wayland を有効にしていません。
Windows / macOS のウィンドウ方式には影響しません。

### ブラウザ版

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd apps/arclet
trunk serve --release
```

表示された localhost の URL を開きます。WebGPU を優先し、URL の末尾に `?webgl` を付けると
WebGL2 を明示的に選べます。配布時は Trunk の出力を HTTPS で配信します。
アプリの処理は Rust / wasm、HTML はキャンバスと起動画面です。

**ブラウザ向けは `cargo clippy --all-targets --target wasm32-unknown-unknown -- -D warnings` のコード検査に合格しています。
実ブラウザでの起動、ファイル操作、WebGPU / WebGL2 の描画は未確認です。** ブラウザ版の Rebuild、曲線端の連続性検査、
Match、Fair、ノードの Print 処理は、バックグラウンド処理の実装まで明示的に使用を拒否します。
既存の CAD コマンドすべてをブラウザのワーカーへ移した段階でもありません。
曲線・曲面の編集を試す場合は、デスクトップ版から始めてください。

## 最初にノードで形を作る

1. 上部の `Nodes` を選びます。スケッチや CAD ツールが開いている場合は、先に編集を終えてください。
2. 最初のグラフは Circle、Number、Extrude の例です。必要なら `Graph > Load example` で戻せます。
3. カードを選ぶと右側に入力が出ます。Circle の大きさ、Number の値をスライダーまたは数値欄で変えます。
4. `Run` で計算します。`Live` を有効にすると変更後に再計算します。実行中は `Stop` で中止できます。
5. Extrude を選び、出力の `Add to model` を押すと、結果の三角形メッシュを CAD 文書へ追加します。
6. ノード側の `Save` でグラフを JSON 保存し、Model 側の `Save` で CAD 文書を保存します。

`Add` でノードを選び、左クリックで配置します。出力ソケット、入力ソケットの順にクリックすると接続できます。
右クリックで配置・接続操作を終えます。カードの見出しをドラッグすると移動でき、`Undo` / `Redo` で戻せます。
入力の `Keep tree` / `Flatten` / `Graft` はデータのまとまり方を変更します。
一つのノードで失敗しても、無関係なノードの結果は残ります。

`Add to model` はその時点のメッシュを追加する操作です。後からグラフを変更しても、追加済みの CAD 形状が
自動で置き換わる連携にはなっていません。現在の Extrude は凸または星形の閉じた輪郭に適した三角形生成です。
Merge Solids は三角形の集合をまとめる処理で、重なりを解消するブーリアン和ではありません。

詳しい操作と上限は [Node editor](../../docs/node-editor.md) にあります。

## 曲線を整える

### Spline と Rebuild

`Model` で `Spline` を選び、平面を選択してスケッチを作ります。制御点を置いて曲線を作り、描画を終えると
点を直接動かせます。通過点を指定したい場合は `Curve > Fit spline` を選びます。

デスクトップ版では、スケッチの編集中に `Curve > Rebuild` を開きます。

1. `Source curve` に元のスプラインを選びます。
2. `Degree` と `Control points` を指定します。次数は 1–7、点数は次数より多く、最大 64 点です。
3. プレビューで形を見比べ、両方向の `Estimated deviation` と `Parameter-fit RMS` を確認します。
4. 結果を確定します。元の曲線、元の点、その拘束は残り、新しい編集可能な CV Spline ができます。

新しい曲線は初期状態で作図用の `Construction` になります。同じ位置の二つの曲線が、押し出し用の閉じた輪郭に
混ざることを避けるためです。輪郭として使う場合は、どちらの曲線を使うか確認して切り替えてください。

端点は固定しますが、端の接線や曲率は固定しません。出力は等間隔の内部ノットと重み 1 を持つ曲線です。
周期スプラインなどの対応外入力は拒否します。偏差は標本と折れ線による推定値で、細部をすべて含む
最大誤差の保証ではありません。

### 曲線端の G0–G3 を調べる

スケッチの編集中に `Curve > Curve continuity` を開き、`Before curve` と `After curve` を選びます。
Before の終点と After の始点を比較します。別の端を調べるには、それぞれの `Reverse` を使います。
`Tolerances` で判定の許容値を変更できます。

| 表示する量 | 単位 | 調べること |
| --- | --- | --- |
| Position gap | mm | 端点の位置が一致するか |
| Tangent angle | degree | 進む向きがそろうか |
| Curvature jump | 1/mm | 曲率ベクトルがそろうか |
| Curvature change | 1/mm² | 長さに対する曲率ベクトルの変化がそろうか |

上位の判定には下位の条件も必要です。離れた二本を、向きが同じという理由で G1 と表示しません。
端の接線がゼロになるなど、評価できない入力はエラーにします。この検査は形を変更しません。

## 面を作り、見比べる

| 入口 | 現在使う処理 | 範囲 |
| --- | --- | --- |
| `Surface > Loft / Sweep / Patch / Boundary / Blend` | 既存の曲面作成コマンド | 対応する入力を各ツールのパネルで指定する |
| `Surface > Match` | 既存面の境界を参照面に合わせる | 新しい選択パネルで対象・参照を分け、G1 / G2 / G3 を指定する。デスクトップ限定 |
| `Surface > Fair` | 曲面の制御点を調整する平滑化 | 強さ、繰り返し回数、移動許容値、境界保持を指定する。デスクトップ限定 |
| `Analyze > Zebra / Reflection / Curvature map / Curvature comb` | 縞、反射、曲率の表示 | 見た目や局所的な変化を確認する |
| `Analyze > Surface continuity` | 継ぎ目上の標本検査 | 隙間、法線角、法曲率差による G0–G2 の検査 |

Match に G3 を指定できることと、一般の曲面全体について G3 を独立に証明できることは別です。
曲線端の検査、曲面の継ぎ目の検査、縞の見え方をそれぞれ確認してください。
解析と実装の詳しい範囲は [曲線・曲面の操作を決める根拠](../../docs/arclet-surface-workflows.md) に記載しています。

## 画面と保存

頻出ツールは上部の `Curve` / `Surface` / `Solid` / `Transform` / `Analyze` にまとめています。
`Select` で面・エッジ・点などの選択対象を切り替え、`View > Fit` で形を画面に収めます。
`Isometric` / `Top` / `Front` / `Right` で見る方向を切り替えます。
`Isolate` は選んだ形以外を隠し、`Show all` はすべてを再表示します。Show all は、隔離前の個々の表示状態を復元する操作ではありません。

下部の `Command` には `Line`、`Loft`、`Match`、`Rebuild`、`Zebra` などを入力できます。
候補を選ぶか `Run` を押します。`Ctrl+K` は入力欄へ移る近道です。数式やスクリプトの実行欄ではありません。
同名の操作を区別する場合は `surface.loft` / `solid.loft` のような完全なコマンド ID を指定します。
`All tools` から共有コアの検索画面も開けます。

カメラは中ボタンドラッグで回転、Shift と中ボタンで平行移動、ホイールで拡大縮小します。
左クリックで配置・選択・編集し、ビュー内の右クリックまたは `Finish` で作業を確定します。
パネルの値や対象が不足しているツールは確定せず、パネルに残ります。パネルの確定・取消ボタンも使えます。
読み取り専用の検査パネルでは Finish は Close と同じです。削除の確認は専用の確認ボタンで答えます。
スケッチ全体を閉じる操作は `Finish sketch` です。

| 保存するもの | 保存場所の選び方 | 内容 |
| --- | --- | --- |
| CAD 文書 | Model の `Save` / `File > Save as…` | 共有コアの `.sc` 文書、形状と履歴 |
| ノードグラフ | Nodes の `Save` | `.json`、ノード・値・接続。計算済みの出力を自動で埋め込む形式ではない |
| 外部向けの形状 | Model の `File > Export STL… / Export 3MF… / Export STEP…` | それぞれの形式で表現できる形状 |

STL / 3MF は三角形メッシュを書き出します。STEP は CAD の境界表現を持つボディが対象で、
ノードから追加したメッシュをそのまま STEP に書き出すことはできません。
STEP の書き出しを、メッシュから滑らかな NURBS 面への変換としては使えません。

ノードグラフは CAD 文書とは別に保存します。名前の横の `*` は現在の文書またはグラフが未保存であることを示します。
グラフの復元用スナップショットは Arclet の設定にも保持しますが、
持ち運びや確実な保管には JSON を保存してください。グラフを開く操作や例の読み込みも `Undo` で戻せます。
JSON の上限は 4 MiB、グラフは最大 2,000 ノード、ブラウザでの評価は最大 256 ノードです。

デスクトップでは `Settings` の `Autosave every` に従って CAD 文書の復旧用コピーを作り、
次回起動時に Arclet の復旧候補を表示します。`Reopen designs` は前回開いていた保存済み文書を再び開く設定です。
起動引数で文書やサンプルを指定した場合は、その指定を優先します。

ブラウザの `Save` はコピーのダウンロードです。開いた元ファイルを上書きせず、CAD 文書をブラウザ内の永続フォルダにも保存しません。
ブラウザ内で CAD 文書の自動保存を継続し、次回起動時に復旧する機能は未対応です。
未保存時にはブラウザへ終了確認を要求しますが、この確認自体がデータを保存するものではありません。
タブを閉じたり再読み込みしたりする前に、必要な CAD 文書とグラフをダウンロードしてください。
アップロードは 1 ファイル 64 MiB、保持中のファイルは最大 32 個・合計 128 MiB です。
容量に達した場合は作業を保存してからページを再読み込みします。

## 印刷について

デスクトップのノードには Slice / G-code の入口があります。スライサーのコアには上下のスキンと
層ごとの充填率の指定を追加しています。詳細は [Solid skins and per-layer infill](../../docs/slice-skins-density.md) を参照してください。

**Bambu A1 / A1 mini での印刷は未確認です。** 現在の出力は汎用の開始・終了処理を使う G-code です。
専用プロファイル、AMS 用のフィラメント情報、`.gcode.3mf` の梱包は未実装です。
`Export 3MF` は形状の書き出しであり、印刷ジョブの `.gcode.3mf` を作る操作ではありません。
サポート、ブリッジ方向、シーム、ワイプ／コースティングも未実装です。
重なった立体の偶奇規則や鋭い角のオフセットの制約も残っています。

## まだ保証していないこと

- xNURBS と同等の汎用制約サーフェス生成、Class A の認証、一般曲面の G3 検査。
- Plasticity PolySplines と同等の、任意メッシュから滑らかな曲面への変換。今回の `CurveChain` は曲線の集合です。
- 任意のノットと有理重みを持つ NURBS を、現行のスケッチ保存形式で無損失に直接編集・保存すること。
- すべての形状でのブーリアン・フィレット・曲面作成の成功、商用 CAD のファイル互換性。
- ブラウザの実操作、Intel 内蔵 GPU の速度、Windows / macOS でのこの新しい実行ファイルの動作、実機印刷。

## 開発時の確認

この開発版で実際に行った検査と既存の失敗は [確認記録](../../docs/arclet-validation.md) に分けて記載します。

通常の確認コマンドは次のとおりです。実行していない項目を、この README の機能表だけで合格扱いにはしません。

```sh
cargo test --profile ci -p solidcraft-geom -p solidcraft-graph -p solidcraft-slice
cargo test --profile ci -p solidcraft-engine -p solidcraft-ui-egui -p arclet
cargo clippy -p arclet -p solidcraft-graph -p solidcraft-slice --all-targets -- -D warnings
cargo check -p arclet --target wasm32-unknown-unknown
cargo fmt --all --check
cargo xtask ci
```

この作業の開始時点では、全体の fmt チェックに既存の `apps/meltcad/src/bench.rs` と
`apps/meltcad/src/precise_services.rs` の差分がありました。依頼と無関係な整形は変更せず、
全体の CI が通ったという報告とは分けて扱います。

## 設計とライセンス

採用した方針は [Arclet の設計判断](../../docs/arclet-decisions.md)、操作調査は
[Plasticity の操作調査](../../docs/arclet-usability-research.md) にあります。
[Serpentine3D](https://github.com/chisomobanzi/Serpentine3D) は公開されたコマンド名と挙動の参考です。
コードを写さず、OpenCASCADE には依存しません。Rhino / Plasticity の公開マニュアルとレビューからも、
操作の考え方だけを採用しています。

本アプリのソースはリポジトリと同じ **MIT OR Apache-2.0** です。
[LICENSE-MIT](../../LICENSE-MIT)、[LICENSE-APACHE](../../LICENSE-APACHE)、[NOTICE](../../NOTICE) を参照してください。
配布時は依存物の条件と著作権表示も維持します:
[THIRD-PARTY-LICENSES.txt](../../THIRD-PARTY-LICENSES.txt)、[ATTRIBUTION.md](../../ATTRIBUTION.md)。
既存の許諾された Rust コアとアイコンを再利用し、商用製品のコード・アイコン・文言は取り込みません。
GPL / LGPL / AGPL の実装を読み込んだり、本体へ持ち込んだりしない方針を継続します。
