# Serpentine3Dとのコマンド比較

項目11の仕様調査。確認日: **2026-11-01（日本時間）**。
比較対象は `feature/node-graph-gcode` の固定コミット `a309790ac8e265b918da09474a02b069609ab3b5`。
この文書は実装済み機能の合格表ではなく、次に作る機能を決めるための登録・仕様の対照表である。

## 結論と件数

独立した実行ファイルと独自のGUIを持つ、軽量なオープンCADとして開発する。
曲線、ロフト、スイープ、曲面ブレンド、計測などの既存のクリーンルームRustコアは、
ライセンスと層の境界を守って再利用する。新しいアプリの操作と画面は独立して設計する。
まず印刷の上下スキンと層別充填率、次に新しいアプリへノード画面をつなぐ。
計算処理を再利用し、独立したアプリから必要な部分だけを使うことで、重複と保守対象を減らす。
一般的な製図用紙、ノット編集、任意曲面の制御点編集は、別の実装・検証が必要になる。

| 数えたもの | 件数 | 確認方法 |
|---|---:|---|
| Serpentine3Dの文書にある主コマンド | 247 | [固定版のコマンド表](https://github.com/chisomobanzi/Serpentine3D/blob/eb02726eba44e55a958a1946fbc1823a51219efb/docs/reference/commands.md)の主名を1行ずつ数えた。別名は数えない |
| 旧SolidCraft一覧のヘッダー | 371 | [固定版のdocs/features.md](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/features.md)に書かれた数字。現在の登録数とは扱わない |
| 同じ旧一覧の実際のID行 | 373 | 重複なし。うち2行がplanned表記、371行が非planned表記 |
| 現行ソースのコマンド宣言 | 508 | [command_specs()](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/mod.rs)が参照する53個のCOMMANDS配列を静的に列挙。重複なし、うち1個はplanned |
| この調査の逆引き対象 | 511 | 現行508件と旧一覧373件の和集合。旧一覧にだけある3件も残した |

**508は実行して確認した件数ではない。** コンパイルしたCLIの`commands`出力は、この調査では未確認。
宣言があることと、必要な入力で動くこと、UIから使えること、正確な形状が出ることは分けて扱う。
ノードエンジンのコンポーネント数も、このコマンド数には足していない。

旧一覧には現行宣言の138件が載っていない。例えば`solid.cone`、`pointcloud.*`、標準ビュー、
`file.export_view_dxf`、スプライン点の追加がある。`surface.boundary`も、旧一覧のplannedから現行では通常宣言になっている。
逆に`solid.melt`、`construct.plane.from_view`、`construct.plane.from_view_origin`は旧一覧にはあるが、
この53配列では登録を確認できない。別のアプリやUIの経路で使えるかは未確認。
生成物の`docs/features.md`はこの作業で書き換えない。

## 読み方と限界

| 表記 | 意味 |
|---|---|
| ある | その目的に対応するコマンド宣言と説明がある。実行結果の保証ではない |
| 一部 | スケッチ限定、出力だけ、複数操作の組合せなど、対象や手順に差がある |
| ない | 調査したコマンド登録に直接の対応がない。下位ライブラリや別アプリに処理がないという意味ではない |
| 未確認 | 宣言や文書だけでは相当性を判断できない |

ある97件、一部87件、ない53件、未確認10件。これは**247個の主コマンド行の分類**であり、製品全体の完成率ではない。
1個のSerpentineコマンドが複数のSolidCraftコマンドに対応する場合、その逆の場合がある。
主名が同じでも、対象、許容誤差、幾何品質、Undo、保存互換性までは同じと判断していない。
全行についてアプリ操作、wasm実行、Intel内蔵GPU、Bambu A1/A1 miniでの印刷は、この調査では**未確認**。

「目的」は公開コマンド名と仕様説明から独自の言葉で短くまとめた。
Serpentineの説明が主名だけの行は、一般的なCAD操作の粒度で整理しており、詳細なオプションは未確認。
比較のために同じコマンド名を載せるが、これらをアプリのラベル・ヘルプ文として流用しない。

## 出典と調査方法

- Serpentine3D: [固定版コマンド仕様](https://github.com/chisomobanzi/Serpentine3D/blob/eb02726eba44e55a958a1946fbc1823a51219efb/docs/reference/commands.md)、[ライセンス](https://github.com/chisomobanzi/Serpentine3D/blob/eb02726eba44e55a958a1946fbc1823a51219efb/LICENSE)。参照コミットは`eb02726eba44e55a958a1946fbc1823a51219efb`。
- SolidCraft: [旧機能一覧](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/features.md)、[コマンドエンジンの説明](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/book/src/dev/commands.md)、[スケッチの説明](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/book/src/guide/sketching.md)、[ソリッドの説明](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/book/src/guide/features.md)、[UIの説明](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/book/src/guide/ui-tour.md)。
- 現行登録: [crates/engine/src/cmd/mod.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/mod.rs)から呼ばれる各`COMMANDS`配列のID・ラベル・入力説明。各IDのファイルへの固定リンクは[逆引き一覧](node-command-inventory.md)にある。
- 旧ヘッダーの扱い: [現在の一覧生成器](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/xtask/src/features.rs)は件数ヘッダーを出さない。旧文書の数字だけで現行機能を判定しない。

Serpentineは文書だけを参照し、実装・依存先のソースは読んでいない。コード、アイコン、ヘルプ文、テンプレートを取り込んでいない。
GPL/LGPL/AGPLのコードも参照していない。実装を追加するときは既存のRustコードと独自の計算・テストを使う。
参照されている`skills/solidcraft-contributing`等は固定コミット内に存在しないため、
[AGENTS.md](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/AGENTS.md)、[CONTRIBUTING.md](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/CONTRIBUTING.md)、上記の開発文書を確認した。

再確認するときは、まず両方の参照コミットを固定し直す。主コマンドと別名を分けて数え、
`command_specs()`が参照する配列だけを列挙する。テスト内のCommandSpec例や未登録の配列を加えない。
CLIを実行できる環境では`cargo run -p solidcraft-cli -- commands`のID集合と照合し、
差分があれば静的な一覧を実行済みと書き換えず、差の原因を調べる。

## 247コマンドの対照表

下の「登録・関連コマンド」は現行ソースのID。別名を含むSerpentine側の正本は上記の固定版コマンド仕様。
「一部」「未確認」の関連IDは、完全な代替を意味しない。

### ブーリアン（4件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `booleandifference` | 立体から別の立体を差し引く | ある | `solid.combine` | operation=cut。幾何的な頑健性は別途テストが必要。 |
| `booleanintersection` | 立体どうしの共通部分を残す | ある | `solid.combine` | operation=intersect。 |
| `booleanunion` | 重なる立体を一体化する | ある | `solid.combine` | operation=join。 |
| `curveboolean` | 平面曲線が区切る領域を選んで輪郭にする | ない | — | スケッチのプロファイル検出とは別に、選択領域を曲線として出すコマンドは未登録。 |

### カメラ（1件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `camera` | 視点や注視点を設定する | ある | `view.navigate`, `render.camera.set`, `render.camera.get` | モデリング用とレンダー用は別の設定。 |

### 動画・画面の記録（2件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `turntable` | モデルを一周見せる動画を作る | 一部 | `view.spin`, `motion.export_animation` | 連続旋回と機構アニメーション出力はある。旋回を動画化する一体の操作は未確認。 |
| `turntableui` | アプリの画面を含む旋回動画を作る | ない | — | UI込みの録画コマンドは未登録。 |

### 曲線（12件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `arc` | 円弧を作る | ある | `sketch.arc.three_point`, `sketch.arc.center_point`, `sketch.arc3d` | 平面スケッチと3Dスケッチに分かれる。指定方法の完全一致は未確認。 |
| `circle` | 円を作る | ある | `sketch.circle.center`, `sketch.circle.two_point`, `sketch.circle.three_point` | 平面スケッチ内で作成する。 |
| `closecrv` | 開いた曲線の端を閉じる | 一部 | `sketch.line`, `sketch.constraint.coincident`, `sketch.loose_ends.tie` | 線の追加や接続拘束で組み立てる。任意曲線を一度に閉じる専用コマンドではない。 |
| `curve` | 制御点から曲線を作る | 一部 | `sketch.spline.control_point`, `sketch.spline.control_point_5` | スケッチの制御点スプライン。任意3D NURBSの同じ入力方式とは限らない。 |
| `divide` | 曲線上に分割点を置く | 一部 | `construct.point.along_path` | 曲線上の点はある。等分点を一括生成する操作は未登録。 |
| `ellipse` | 楕円を作る | ある | `sketch.ellipse` | スケッチ平面内。 |
| `interpcrv` | 指定した点を通る曲線を作る | 一部 | `sketch.spline.fit_point`, `sketch.spline3d` | 平面の周期閉曲線と3D補間を別々に扱う。全オプション一致は未確認。 |
| `line` | 線分を作る | ある | `sketch.line`, `sketch.line3d`, `sketch.line.midpoint` | 平面・3D・中点基準に分かれる。 |
| `point` | 点を置く | ある | `sketch.point`, `sketch.point3d` | スケッチに所属する点。 |
| `polyline` | 連続した線分を作る | ある | `sketch.line`, `sketch.chain` | スケッチガイドに連続線の操作が記載される。独立した単一ポリライン型との一致は未確認。 |
| `rectangle` | 長方形を作る | ある | `sketch.rectangle.two_point`, `sketch.rectangle.three_point`, `sketch.rectangle.center` | スケッチ内の各指定方法。 |
| `tweencurves` | 2本の曲線の中間形を作る | ない | — | 中間曲線列を作る専用コマンドは未登録。 |

### 変形・曲率（8件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `bend` | 形を曲げる | ない | — | 板金の折曲げは別機能。任意形状の変形コマンドは未登録。 |
| `curvaturegraph` | 曲線の曲率を櫛状に表示する | ある | `inspect.curvature_comb` | 解析表示。表示の操作感は未確認。 |
| `draftanalysis` | 抜き方向に対する面の傾きを調べる | ある | `inspect.draft` | 解析条件と色分けは独自仕様。 |
| `extend` | 曲線の端を延ばす | ある | `sketch.extend` | スケッチの曲線に限定。混在曲線での対応範囲はガイド参照。 |
| `flow` | 曲線に沿って形を変形する | ない | — | 経路上の複製とは異なる連続変形は未登録。 |
| `matchcrv` | 曲線端の接続方向や曲率を合わせる | 一部 | `sketch.constraint.tangent`, `sketch.constraint.curvature` | 拘束として実現する。任意の既存曲線端を作り直す専用操作ではない。 |
| `taper` | 形に先細りを付ける | 一部 | `solid.draft`, `solid.extrude` | 面の抜き勾配と押出テーパー。任意オブジェクト全体のテーパー変形とは範囲が異なる。 |
| `twist` | 形を軸周りにねじる | 一部 | `solid.sweep`, `surface.sweep` | スイープ生成時のtwist指定。任意の完成形を後からねじる操作は未登録。 |

### 表示・ビュー（60件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `1view` | 表示を単一ビューにする | 未確認 | — | 単一ビューポートUIはあるが、分割表示から戻す相当コマンドは特定できない。 |
| `4view` | 4方向のビューを並べる | ない | — | 4分割ビューポート用コマンドは未登録。 |
| `ai` | 自然言語の支援パネルを開く | ない | — | MCP接続は別の仕組み。内蔵チャットパネル相当は未登録。 |
| `angle` | 方向どうしの角度を測る | ある | `inspect.measure` | 選択した要素間の角度。点を3回選ぶ操作の一致は未確認。 |
| `area` | 面積を測る | ある | `inspect.measure` | 面・ボディの計測。 |
| `back` | 背面を見る | ある | `view.back` | 標準ビューの登録あり。 |
| `bottom` | 底面を見る | ある | `view.bottom` | 標準ビューの登録あり。 |
| `clippingplane` | 平面でモデルの表示を切る | 一部 | `inspect.section` | 断面解析。独立した複数のクリッピング平面オブジェクトとは異なる。 |
| `cplane` | 作図の基準平面を変える | 一部 | `sketch.create`, `sketch.redefine`, `construct.plane.offset`, `construct.plane.three_points` | スケッチ平面と作業平面で扱う。全ツール共通の一つのCPlane操作は未確認。 |
| `curvature` | 曲線や面の曲がりを調べる | 一部 | `inspect.curvature_comb`, `inspect.minimum_radius`, `inspect.curvature_map` | 解析機能はある。指定点の曲率値を返す同等操作は未確認。 |
| `curvatureanalysis` | 曲面の曲率分布を表示する | ある | `inspect.curvature_map` | 解析機能の登録あり。 |
| `disableclippinginview` | 現在のビューだけ断面表示を止める | ない | — | ビュー別のクリッピング制御は未登録。 |
| `disableclippingplane` | 切断表示を一時停止する | 一部 | `inspect.section` | clearで断面解除。平面を保存したまま無効化する操作との一致は未確認。 |
| `distance` | 要素間の距離を測る | ある | `inspect.measure` | 最短距離などを計測できる宣言。 |
| `enableclippinginview` | 現在のビューだけ断面表示を有効にする | ない | — | ビュー別のクリッピング制御は未登録。 |
| `enableclippingplane` | 切断表示を有効にする | 一部 | `inspect.section` | 断面を設定し直す。保存済み平面の有効化と同一ではない。 |
| `floatviewport` | ビューを別ウィンドウに出す | ない | — | 独立したモデルビューポートを増設するコマンドは未登録。 |
| `front` | 正面を見る | ある | `view.front` | 標準ビューの登録あり。 |
| `ghosted` | 形を透過して表示する | 一部 | `appearance.assign` | オブジェクトのopacity設定。専用の表示モードではない。 |
| `grid` | 作図グリッドを表示・非表示にする | ある | `view.grid` | 表示切替の登録あり。 |
| `gridsnap` | グリッドに点を吸着させる | 未確認 | `sketch.snap` | 推論スナップの登録はあるが、グリッドへの吸着との対応は未確認。 |
| `gumball` | 移動・回転・拡縮のハンドルを使う | 一部 | `solid.move`, `sketch.transform` | 変換コマンドと各ツールのハンドルはある。常設ギズモの一括切替は未登録。 |
| `gumballalignment` | 変換ハンドルの座標基準を切り替える | ない | — | 常設ギズモの整列基準を選ぶ専用コマンドは未登録。 |
| `isocurves` | 曲面のパラメータ線を表示する | 一部 | `inspect.isocurve` | 解析として利用。全曲面に対する描画設定とは範囲が異なる。 |
| `isometric` | 等角方向を見る | ある | `view.isometric` | 標準ビューの登録あり。 |
| `left` | 左側面を見る | ある | `view.left` | 標準ビューの登録あり。 |
| `length` | 曲線やエッジの長さを測る | ある | `inspect.measure` | 要素の計測に含まれる。 |
| `maxviewport` | 選んだビューを最大化する | ない | — | 複数のモデルビューポートを切り替える相当コマンドは未登録。 |
| `namedview` | 名前を付けて視点を保存する | ある | `view.save`, `view.list`, `view.apply`, `view.rename`, `view.delete` | 保存・一覧・復元・改名・削除に分かれる。 |
| `newviewport` | 追加のモデルビューを開く | ない | — | レンダーのパネル配置とは別。モデルビューポート増設は未登録。 |
| `ortho` | 作図を基準軸方向に揃える | 一部 | `sketch.constraint.horizontal_vertical`, `sketch.snap` | 拘束と推論スナップ。全ツール共通の直交モード切替は未登録。 |
| `osnap` | 端点・中心などに吸着する | 一部 | `sketch.snap`, `solid.align` | スケッチ推論とAlignの頂点・中点・中心指定。統一された種類別切替は未確認。 |
| `perspective` | 透視投影に切り替える | ある | `view.perspective` | ビューポート設定の登録あり。 |
| `pictureframe` | 参照画像をモデル内に置く | ある | `canvas.insert`, `canvas.edit`, `canvas.calibrate` | 画像キャンバスの作成・編集・寸法合わせ。 |
| `pointsoff` | 制御点の編集表示を閉じる | 一部 | `sketch.finish` | スケッチ終了は可能。任意曲線・曲面の制御点だけを隠す操作ではない。 |
| `pointson` | 曲線・曲面の制御点を編集する | 一部 | `sketch.edit`, `sketch.move_point`, `sketch.move3d` | スケッチ点編集はある。NURBS曲面の任意の制御点格子を直接動かすUIは未確認。 |
| `printcheck` | 印刷用の形状を検査する | 一部 | `inspect.measure` | 閉鎖性の情報はある。非多様体・薄肉・造形可能性をまとめた専用検査は未確認。 |
| `radius` | 曲線の曲率半径を調べる | 一部 | `inspect.measure`, `inspect.minimum_radius` | 円の半径と最小半径解析。任意の曲線上の指定点で測る同等操作は未確認。 |
| `redoview` | 戻した視点変更をやり直す | ない | — | カメラだけの履歴Redoは未登録。 |
| `rendered` | 材質と照明で形を表示する | 一部 | `render.start`, `render.scene`, `appearance.assign` | レンダー機能はある。対話ビューポートの同じ表示モードではない。 |
| `right` | 右側面を見る | ある | `view.right` | 標準ビューの登録あり。 |
| `selclippingplane` | クリッピング平面をまとめて選ぶ | ない | — | 独立したクリッピング平面の選択コマンドは未登録。 |
| `shaded` | 面を塗って表示する | ある | `view.visual_style` | style=shaded。 |
| `snap` | 作図時の吸着を切り替える | 一部 | `sketch.snap` | スケッチの推論スナップ。全ツール共通の仕様とは限らない。 |
| `spacemouse` | 3Dマウスの状態や入力を確認する | 未確認 | — | デバイス対応の有無をコマンド登録だけから断定できない。専用診断コマンドは特定できない。 |
| `surfaceedges` | 面の輪郭線表示を切り替える | 一部 | `view.visual_style` | shadedとshaded_with_edgesを切り替える。対象限定の同じ動作は未確認。 |
| `technical` | 隠線を区別した技術図の表示にする | 一部 | `view.visual_style`, `file.export_view_dxf` | 隠線付き描画とDXF投影。専用の技術図表示モードとの一致は未確認。 |
| `tolerance` | 文書全体のモデリング許容差を設定する | 未確認 | — | 各コマンドのtoleranceとは別。文書全体の単一設定は特定できない。 |
| `top` | 上面を見る | ある | `view.top` | 標準ビューの登録あり。 |
| `undoview` | 直前の視点に戻す | ない | — | カメラだけのUndo履歴は未登録。 |
| `units` | 文書の単位を変える | ある | `document.units` | 単位変更の登録あり。モデルを再スケールする選択肢の一致は未確認。 |
| `viewcapturetoclipboard` | 表示画像をクリップボードへ送る | 一部 | `render.result.copy` | レンダー結果のコピー。現在のモデリングビューポートの撮影ではない。 |
| `viewcapturetofile` | 表示画像をファイルに保存する | 一部 | `render.export` | レンダー結果の出力。モデリング画面の撮影との一致は未確認。 |
| `volume` | 立体の体積を測る | ある | `inspect.measure` | 閉じて向きが整合するボディを対象とする。 |
| `wireframe` | ワイヤ表示に切り替える | ある | `view.visual_style` | style=wireframe。 |
| `zebra` | 縞の反射で面の連続性を見る | ある | `inspect.zebra` | 解析機能の登録あり。 |
| `zoom` | 表示を拡大・縮小する | ある | `view.tool`, `view.navigate`, `view.fit` | ズームとフィット。全ての対話オプション一致は未確認。 |
| `zoomextents` | 表示中の形が収まるようにする | ある | `view.fit` | 選択状態によって対象が変わるため、全体フィット時は選択条件に注意。 |
| `zoomselected` | 選択した形が収まるようにする | ある | `view.fit` | 選択対象へのフィットが宣言されている。 |
| `zoomwindow` | 指定した矩形範囲へ拡大する | 未確認 | `view.tool` | ズームツールはあるが、2点で窓を指定する方式は未確認。 |

### 製図・用紙（28件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `annotedit` | 注記の内容やスタイルを変える | 一部 | `sketch.edit_text` | スケッチ文字の編集。製図用注記オブジェクトとは異なる。 |
| `detail` | 製図用紙にモデルの詳細ビューを置く | ない | — | 用紙上の詳細ビューは未登録。 |
| `detailborder` | 詳細ビューの枠を設定する | ない | — | 製図用紙の枠設定は未登録。 |
| `detaildelete` | 詳細ビューを削除する | ない | — | 用紙上の詳細ビューは未登録。 |
| `detaillock` | 詳細ビューの視点を固定する | ない | — | 製図の詳細ビュー固定は未登録。 |
| `detailmode` | 詳細ビューの表示方式を変える | ない | — | 用紙内の表示方式指定は未登録。 |
| `detailscale` | 詳細ビューの図面縮尺を設定する | ない | — | 製図用の縮尺設定は未登録。 |
| `detailsection` | 詳細ビューを断面図にする | ない | — | モデリングの断面解析とは別。図面内の断面ビューは未登録。 |
| `dim` | 寸法を記入する | 一部 | `sketch.dimension` | スケッチ寸法。用紙上の独立した寸法注記ではない。 |
| `dimangle` | 角度寸法を記入する | 一部 | `sketch.dimension` | スケッチの角度拘束・参照寸法。 |
| `dimdiameter` | 直径寸法を記入する | 一部 | `sketch.dimension`, `sketch.toggle_diameter` | スケッチの円寸法。図面注記としての挙動は未確認。 |
| `dimradius` | 半径寸法を記入する | 一部 | `sketch.dimension`, `sketch.toggle_diameter` | スケッチの円寸法。図面注記としての挙動は未確認。 |
| `dimstyle` | 寸法・注記の共通スタイルを設定する | ない | — | 製図用の名前付きスタイルは未登録。 |
| `dot` | 点に短いラベルを付ける | ない | — | 3D注記ドットは未登録。 |
| `exportdxf` | 2D線画をDXFで出力する | ある | `sketch.export_dxf`, `sheet.export_dxf`, `file.export_view_dxf` | スケッチ・板金・現在のビュー投影。用紙や注記の保存には対応差がある。 |
| `exportpdf` | 図面をPDFで出力する | ない | — | 製図PDF出力コマンドは未登録。 |
| `exportsvg` | 図面をSVGで出力する | ない | — | SVG読み込みはあるが図面のSVG書き出しは未登録。 |
| `hatch` | 領域にハッチを入れる | 一部 | `inspect.section` | 断面解析のハッチ表示のみ。任意の曲線領域を図面ハッチにする操作ではない。 |
| `hatchpatterns` | ハッチ模様を追加・削除する | ない | — | PATパターンの管理は未登録。 |
| `layout` | 図面の用紙を作る | ない | — | view.layoutはUI配置の変更であり製図用紙ではない。 |
| `leader` | 引出線付き注記を作る | ない | — | 製図注記用コマンドは未登録。 |
| `make2d` | 3Dから投影線画を作る | 一部 | `file.export_view_dxf` | 隠線を扱うDXF投影がある。編集可能なモデル内曲線としての生成は未確認。 |
| `printsheets` | 複数の図面用紙を印刷する | ない | — | 用紙の一括印刷は未登録。 |
| `revision` | 図面に改訂履歴の行を加える | ない | — | document.revisionは文書の改訂情報であり図面の改訂表ではない。 |
| `scalebar` | 図面に縮尺バーを置く | ない | — | 製図用の縮尺バーは未登録。 |
| `sheetindex` | 用紙の一覧を図面に載せる | ない | — | 図面用紙の目次は未登録。 |
| `text` | 文字注記を置く | 一部 | `sketch.text` | スケッチ文字。製図注記としての同じ振る舞いは未確認。 |
| `titleblock` | 図面に表題欄を置く | ない | — | 製図の表題欄は未登録。 |

### 編集（28件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `boundingbox` | 対象を囲む箱を作る | ある | `solid.bounding_solid` | 対象ボディを囲むソリッド。 |
| `chamfer` | 曲線の角を直線で落とす | ある | `sketch.chamfer.equal_distance`, `sketch.chamfer.distance_angle`, `sketch.chamfer.two_distance` | スケッチ内の面取り。 |
| `delete` | 選択した要素を削除する | ある | `selection.delete`, `sketch.delete` | 対象種別ごとに削除の規則がある。 |
| `dir` | 曲線や面の方向を調べる | 一部 | `model.edges`, `model.faces` | 形状情報の取得はある。画面に向きの矢印を出す専用操作は未確認。 |
| `explode` | 複合形状を構成要素に分ける | 一部 | `surface.unstitch`, `browser.ungroup` | 面の分離とグループ解除。複合曲線・ブロックを一つの操作で分解する方式ではない。 |
| `fillet` | 角を丸める | ある | `sketch.fillet`, `solid.fillet` | スケッチ曲線とソリッドのエッジに分かれる。 |
| `filletcorners` | ポリラインの角を一括で丸める | 一部 | `sketch.fillet` | 個別の角のフィレットはある。一括処理の同等操作は未確認。 |
| `flip` | 曲線の方向や面の表裏を逆にする | ある | `sketch.spline.reverse`, `surface.reverse_normal` | スプラインの向きと曲面の法線。全種類の曲線への対応は未確認。 |
| `hide` | 対象を非表示にする | ある | `browser.visibility` | ブラウザ項目の可視性を変更する。 |
| `insertknot` | 形を保ったままNURBSのノットを増やす | ない | — | sketch.spline.insert_pointは補間点の追加であり、形状不変のノット挿入ではない。 |
| `join` | 端や境界で形状を接続する | 一部 | `surface.stitch`, `sketch.constraint.coincident`, `sketch.loose_ends.tie` | 曲面縫合と曲線の接続拘束。任意曲線を複合曲線へ結合する操作は未確認。 |
| `layer` | 対象を層で整理する | 一部 | `browser.group`, `browser.move`, `browser.visibility` | ブラウザグループはある。色・線種・印刷設定を継承するレイヤーとは異なる。 |
| `material` | 見た目の材質を割り当てる | ある | `appearance.assign`, `appearance.library` | 物性のmaterial.assignとは区別する。 |
| `offset` | 曲線から一定距離の曲線を作る | ある | `sketch.offset` | 混在輪郭や特異点には対応限界がある。ガイド参照。 |
| `plugins` | 拡張機能の一覧を調べる | ない | — | 実行時プラグイン一覧は未登録。 |
| `rebuild` | 曲線を指定した次数・点数で作り直す | ない | — | 曲線の再構築専用コマンドは未登録。既存フィーチャの再計算とは別。 |
| `recordhistory` | 元形状への変更を派生形状へ反映する | 一部 | `timeline.compute_all`, `timeline.edit` | 既存のパラメトリック履歴を使う。履歴記録のオン・オフ方式ではない。 |
| `redo` | 取り消した編集をやり直す | ある | `edit.redo` | 編集履歴のRedo。 |
| `removecontrolpoint` | 保持している制御点を削除する | 未確認 | `sketch.delete` | スケッチ要素の削除はあるが、曲線を保つ制御点削除の条件は未確認。 |
| `removeknot` | 許容誤差を見ながらノットを除く | ない | — | NURBSノット削除は未登録。 |
| `rename` | 対象の名前を変える | ある | `body.rename`, `component.rename`, `timeline.rename` | 対象種別ごとのコマンド。 |
| `selall` | 全ての対象を選択する | 一部 | `select.set` | ID一覧を列挙すれば設定できる。全選択専用の登録はない。 |
| `selnone` | 選択を解除する | ある | `select.clear` | 選択解除の登録あり。 |
| `show` | 非表示の対象を表示する | ある | `browser.visibility` | 項目ごとの可視性を変更する。 |
| `smooth` | 曲線の制御点をならす | ない | — | surface.fairは曲面用であり、曲線のSmooth相当は未登録。 |
| `split` | 曲線や立体を切り分ける | 一部 | `sketch.break`, `solid.split_body`, `solid.split_face` | 対象ごとに分かれる。任意曲面を含む同一操作ではない。 |
| `trim` | 境界で余分な部分を切り落とす | ある | `sketch.trim`, `surface.trim` | スケッチと曲面に分かれる。 |
| `undo` | 直前の編集を取り消す | ある | `edit.undo` | 編集履歴のUndo。 |

### ファイル（6件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `export` | 形状を別形式に書き出す | ある | `file.export`, `file.save_mesh` | 対応形式・精度・属性保存は形式別に確認が必要。 |
| `import` | 外部の形状を取り込む | ある | `file.insert_step`, `file.insert_mesh`, `sketch.insert_dxf`, `sketch.insert_svg`, `pointcloud.insert` | 各形式別に入口が分かれる。Serpentineと同じ形式一式という意味ではない。 |
| `new` | 新しい文書を作る | ある | `file.new` | 文書作成の登録あり。 |
| `open` | 保存した文書を開く | ある | `doc.open` | 文書読み込みの登録あり。 |
| `save` | 文書を保存する | ある | `file.save`, `file.save_as` | 保存と名前を付けた保存。 |
| `setdefaultapp` | 専用拡張子の標準アプリに設定する | ない | — | OSの関連付けを変更する同等コマンドは未登録。 |

### ヘルプ（1件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `help` | 使えるコマンドや入力を調べる | ある | `engine.commands`, `engine.describe` | 一覧と入力・出力の説明。専用UIの同じ操作ではない。 |

### 整理・属性（26件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `audit` | 形状の整合性を調べる | 一部 | `document.inspect`, `inspect.measure` | 構造と閉鎖性の情報はある。全オブジェクトの妥当性検査一括実行は未確認。 |
| `block` | 再利用できる定義と配置を作る | 一部 | `component.from_bodies`, `occurrence.copy` | コンポーネントとインスタンス。任意の曲線や注記を含むブロックとは範囲が異なる。 |
| `blocklist` | 再利用する定義の一覧を出す | 一部 | `component.list` | コンポーネント一覧。 |
| `breptomesh` | 境界表現の形をメッシュにする | 一部 | `file.save_mesh` | メッシュ出力はある。文書内のボディをメッシュへ置換する操作とは異なる。 |
| `bringforward` | 重なった表示を一段前に出す | 一部 | `browser.order` | ブラウザ順の変更はあるが、2Dの描画順と同じとは限らない。 |
| `bringtofront` | 重なった表示を最前面に出す | 一部 | `browser.order` | ブラウザ順と画面上の重なり順は別。描画順操作は未確認。 |
| `changelayer` | 所属する層を変える | 一部 | `browser.move`, `component.move_bodies`, `component.move_sketches` | グループ・コンポーネントへの移動。レイヤー属性の継承は対象外。 |
| `count` | 種類や所属ごとの個数を数える | 一部 | `document.inspect`, `assembly.bom`, `component.list` | 文書情報と部品表。任意のオブジェクト種別・レイヤー別集計とは範囲が異なる。 |
| `getusertext` | 任意の属性を読む | 一部 | `document.metadata`, `component.metadata` | 文書・部品の属性。全オブジェクトに同じキー値を付ける方式ではない。 |
| `group` | 対象をまとめて扱う | 一部 | `browser.group` | ブラウザの整理用グループ。選択・変換のグループ挙動は未確認。 |
| `insert` | 再利用する定義を配置する | 一部 | `component.insert_design`, `occurrence.copy` | 外部設計やコンポーネントのインスタンス。 |
| `linetype` | 線の破線パターンを設定する | ない | — | 任意オブジェクトの線種・ByLayer設定は未登録。 |
| `lock` | 対象を編集できない状態にする | ない | — | occurrence.groundは機構の固定。一般オブジェクトの編集ロックとは異なる。 |
| `lockother` | 選択以外の対象をロックする | ない | — | 一般オブジェクトの選択保護操作は未登録。 |
| `matchprops` | 外観などの属性を別の対象へ合わせる | 一部 | `appearance.assign`, `appearance.replace` | 見た目の割当はある。層・線幅・ハッチ等をまとめて移す操作は未登録。 |
| `meshtobrep` | メッシュを編集できる境界表現にする | 一部 | `mesh.convert` | 一定断面の機械形状を再構築する範囲。任意メッシュの全三角形を面にする方式ではない。 |
| `pointcloud` | 点群を読み込み、状態を調べる | 一部 | `pointcloud.insert`, `pointcloud.list`, `pointcloud.edit` | 読み込み時の間引きはある。既存点群を後から再サンプリングする同等操作は未確認。 |
| `printwidth` | 図面に出す線の太さを設定する | ない | — | オブジェクトごとの印刷線幅は未登録。 |
| `purge` | 未使用の定義や空の層を取り除く | ない | — | 一括整理コマンドは未登録。 |
| `sectionhatch` | 断面の塗りを設定する | 一部 | `inspect.section` | ハッチの有無とボディ色。オブジェクトごとの断面スタイルとは異なる。 |
| `sendbackward` | 重なった表示を一段後ろに送る | 一部 | `browser.order` | ブラウザ順は変更できるが、2D描画順との一致は未確認。 |
| `sendtoback` | 重なった表示を最後面へ送る | 一部 | `browser.order` | ブラウザ順と画面上の描画順は区別する。 |
| `setusertext` | 任意の属性を書き込む | 一部 | `document.metadata.set`, `component.metadata.set` | 文書・部品単位。全オブジェクト共通の任意属性ではない。 |
| `ungroup` | グループを解く | 一部 | `browser.ungroup` | ブラウザグループの解除。 |
| `unlockall` | 対象の編集ロックをまとめて解除する | ない | — | component.unlockは剛体結合の解除であり、この機能ではない。 |
| `what` | 対象の種類や形状情報を調べる | ある | `document.inspect`, `model.faces`, `model.edges`, `component.properties` | 問い合わせが対象種別ごとに分かれる。 |

### 選択（16件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `invert` | 選択と非選択を反転する | ない | — | 専用の反転選択コマンドは未登録。 |
| `isolate` | 選択対象以外を隠す | 一部 | `browser.visibility` | 個別可視性から組み立てられる。元の表示状態を保持する専用Isolateは未確認。 |
| `selcrv` | 曲線をまとめて選ぶ | 一部 | `select.set` | 対象を列挙すれば設定できる。曲線型を検索して全選択するコマンドは未登録。 |
| `seldup` | 同じ場所の重複を選ぶ | 一部 | `component.copies` | 同じコンポーネントの検出はある。任意形状の重複選択ではない。 |
| `selfilter` | 選択できる対象種別を絞る | 一部 | `select.filter` | body・face・componentの粒度設定。全種類のフィルターとは異なる。 |
| `selfiltertoggle` | フィルターを一時的に切り替える | 未確認 | `select.filter` | 粒度設定はあるが、元の条件を保持した一時停止は未確認。 |
| `selhatch` | ハッチをまとめて選ぶ | ない | — | 独立したハッチオブジェクトの選択は未登録。 |
| `sellast` | 最後に作成した対象を選ぶ | ない | — | 最後の作成物を検索する専用コマンドは未登録。 |
| `sellayer` | 指定した層の全対象を選ぶ | ない | — | レイヤー単位の選択コマンドは未登録。 |
| `selname` | 名前で対象を探して選ぶ | 一部 | `document.inspect`, `select.set` | 文書の列挙と選択を組み合わせる。部分一致選択の専用操作は未登録。 |
| `selpointcloud` | 点群をまとめて選ぶ | 未確認 | `pointcloud.list` | 点群一覧はある。ビュー上での一括選択対応は未確認。 |
| `selprev` | 前回の選択を復元する | ない | — | 選択履歴の復元コマンドは未登録。 |
| `selpt` | 点をまとめて選ぶ | 一部 | `select.set` | sketch_pointを列挙して選択できる。型で一括検索する操作は未登録。 |
| `selsolid` | ソリッドをまとめて選ぶ | 一部 | `select.set` | bodyの列挙選択。閉じた立体だけを判定して選ぶ同等操作は未確認。 |
| `selsrf` | 曲面をまとめて選ぶ | 一部 | `select.set` | faceやbodyを列挙して選択。曲面種別の一括検索は未登録。 |
| `unisolate` | 一時非表示を解除して戻す | 未確認 | `browser.visibility` | 可視性の変更は可能。隔離前の状態を復元する仕組みは未確認。 |

### 立体の編集（9件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `booleansplit` | 切断に使った立体も含めて全片を残す | 一部 | `solid.split_body`, `solid.combine` | 分割とブーリアンはある。任意ソリッドの相互分割を一度に行う方式は未確認。 |
| `cap` | 平らな開口部にふたをする | 一部 | `surface.patch`, `surface.stitch` | 面の生成と縫合で組み立てる。全開口を自動で閉じる専用コマンドではない。 |
| `chamferedge` | 立体のエッジを面取りする | ある | `solid.chamfer` | 距離・角度などの指定を持つ。 |
| `contour` | 等間隔の平面で断面輪郭を取る | 一部 | `inspect.section`, `sketch.intersect` | 単一断面や交差の取得はある。輪郭群の一括生成は未登録。 |
| `filletedge` | 立体のエッジを丸める | ある | `solid.fillet` | 変動半径を含む登録。あらゆる形状での成功を意味しない。 |
| `intersect` | 形状どうしの交差線を取る | ある | `sketch.intersect`, `sketch.intersection_curve` | スケッチへの投影・3D交線に分かれる。 |
| `mergeallcoplanarfaces` | 同じ平面上の隣接面をまとめる | ない | — | 既存ボディの面を一括整理する専用コマンドは未登録。 |
| `pushpull` | 面を押したり引いたりする | ある | `solid.press_pull` | 選択した面の編集。 |
| `section` | 平面で切った輪郭を作る | 一部 | `inspect.section`, `sketch.intersect` | 表示用断面とスケッチ交差。単一の切断線入力による同じ操作は未確認。 |

### 基本立体（5件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `box` | 直方体を作る | ある | `solid.box` | 基本形状の登録あり。 |
| `cone` | 円錐・円錐台を作る | ある | `solid.cone` | 現行登録に存在。旧docs/features.mdでは未掲載。 |
| `cylinder` | 円柱を作る | ある | `solid.cylinder` | 基本形状の登録あり。 |
| `sphere` | 球を作る | ある | `solid.sphere` | 基本形状の登録あり。 |
| `torus` | 円環を作る | ある | `solid.torus` | 基本形状の登録あり。 |

### 曲面（25件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `blendcrv` | 2本の曲線を滑らかな曲線でつなぐ | ある | `sketch.blend_curve` | スケッチ内のブレンド。 |
| `blendsrf` | 2つの面境界を滑らかにつなぐ | ある | `surface.blend` | 連続性の指定あり。入力形状別の成功範囲は未確認。 |
| `dupborder` | 曲面の境界を曲線として取り出す | 一部 | `sketch.project`, `sketch.include_3d` | 投影・3D取り込みで参照する。独立した境界曲線へ一度に複製する操作は未確認。 |
| `dupedge` | 選んだエッジを曲線にする | 一部 | `sketch.project`, `sketch.include_3d`, `sketch.break_link` | 参照を取り込み、必要ならリンク解除。独立曲線の同じ操作とは限らない。 |
| `dupfaceborder` | 選んだ面の外周を曲線にする | 一部 | `sketch.project`, `sketch.include_3d` | 面やエッジをスケッチへ取り込む。外周だけを抽出する同等操作は未確認。 |
| `edgesrf` | 境界曲線から面を張る | ある | `surface.boundary`, `surface.patch` | 境界・ネットワーク面の登録。旧カタログのplanned表記から進んでいる。 |
| `extendsrf` | 面を境界の外へ延ばす | ある | `surface.extend` | 曲面編集の登録あり。 |
| `extractisocurve` | 曲面の等パラメータ曲線を取り出す | ある | `sketch.isoparametric_curve` | スケッチに取り込む形。 |
| `extractsrf` | ボディから面を分離する | ある | `surface.unstitch` | 面を選んで個別の曲面ボディにする。 |
| `extrude` | 曲線や領域を押し出す | ある | `solid.extrude`, `surface.extrude` | ソリッドと曲面に分かれる。 |
| `helix` | らせんを作る | 一部 | `solid.coil` | コイルの生成。独立したらせん曲線の作成とは異なる。 |
| `loft` | 複数の断面をつなぐ | ある | `solid.loft`, `surface.loft` | ソリッドと曲面のロフト。 |
| `offsetsrf` | 曲面から一定距離の面を作る | ある | `surface.offset` | 曲面オフセットの登録あり。 |
| `patch` | 境界や曲線網から面を補う | ある | `surface.patch`, `surface.boundary` | パッチとネットワーク面に分かれる。 |
| `pipe` | 曲線に沿う管を作る | ある | `solid.pipe` | パスに沿う管の登録あり。 |
| `planarsrf` | 平面の閉じた輪郭に面を張る | ある | `surface.patch` | スケッチのプロファイルから平面パッチ。 |
| `project` | 曲線を平面・曲面へ投影する | ある | `sketch.project`, `sketch.project_to_surface` | 対象に応じたスケッチコマンド。 |
| `pull` | 曲線を近くの曲面に沿わせる | 未確認 | `sketch.project_to_surface` | 投影と最短距離による引き寄せは別。Pull相当の方式は未確認。 |
| `revolve` | 断面を回転させて形を作る | ある | `solid.revolve`, `surface.revolve` | ソリッドと曲面に分かれる。 |
| `shell` | 立体を中空にする | ある | `solid.shell` | 厚さと開口面を指定する登録。 |
| `sweep1` | 1本の経路に沿って断面を運ぶ | ある | `solid.sweep`, `surface.sweep` | ソリッド・曲面のスイープ。 |
| `sweep2` | 2本のレールに沿って断面を運ぶ | 一部 | `solid.sweep`, `surface.sweep` | rails指定はある。任意の2レール操作と全条件が一致するかは未確認。 |
| `textobject` | 文字の輪郭をモデル形状にする | ある | `sketch.text`, `sketch.edit_text` | 編集可能なスケッチ文字と押出に使える輪郭。 |
| `unrollsrf` | 展開できる面を平面に広げる | 一部 | `sheet.flat_pattern`, `sheet.unfold` | 板金の展開。任意の可展面を扱う一般展開ではない。 |
| `untrim` | 切り取る前の面の領域を復元する | ある | `surface.untrim` | 穴・外周の復元条件を持つ。 |

### 配置・変換（16件）

| Serpentine主名 | 目的 | 判定 | 登録・関連コマンド | 範囲・差分 |
|---|---|---|---|---|
| `array` | 格子状に複製する | ある | `solid.pattern.rectangular`, `sketch.pattern.rectangular` | ボディ・フィーチャとスケッチに分かれる。 |
| `arraypath` | 経路に沿って複製する | ある | `solid.pattern.path` | パスパターンの登録あり。 |
| `arraypolar` | 中心の周囲に複製する | ある | `solid.pattern.circular`, `sketch.pattern.circular` | 円形パターンの登録あり。 |
| `copy` | 対象を複製して配置する | ある | `solid.move`, `sketch.move`, `occurrence.copy` | ボディ・スケッチ・インスタンスごとの方法。 |
| `mirror` | 基準面や軸で反転したコピーを作る | ある | `solid.mirror`, `sketch.mirror` | 対象ごとのミラー。 |
| `move` | 対象を移動する | ある | `solid.move`, `sketch.move`, `occurrence.move` | 各オブジェクト種別に分かれる。 |
| `orient` | 基準点を使って位置と向きを合わせる | 一部 | `solid.align`, `solid.move` | 点・面・平面によるAlign。2点対応から変換を作る同じ入力手順ではない。 |
| `orient3pt` | 3点対応で位置と姿勢を合わせる | 一部 | `solid.align`, `occurrence.move` | 平面整列や行列による配置。3点の対を順に選ぶ専用操作は未登録。 |
| `projecttocplane` | 作図平面へ形を平らにする | 一部 | `sketch.project` | スケッチへの投影。任意の既存ボディ全体の平坦化とは異なる。 |
| `rotate` | 基準軸の周りに回転する | ある | `solid.move`, `sketch.transform` | axis・angleまたは平面内angleを使う。 |
| `rotate3d` | 空間内の任意軸の周りに回転する | ある | `solid.move`, `occurrence.move` | 軸・角度または配置行列。軸の2点入力と同一ではない。 |
| `scale` | 基準点から一様に拡縮する | ある | `solid.scale`, `sketch.scale` | ボディとスケッチのスケール。 |
| `scale1d` | 1方向だけ伸縮する | 一部 | `solid.scale` | factorsで軸別指定。任意の基準方向を選ぶ操作は未確認。 |
| `scale2d` | 平面内の2方向だけ伸縮する | 一部 | `solid.scale` | 軸別factorsで表現。任意CPlaneを基準にした同じ操作は未確認。 |
| `scalenu` | 軸ごとに異なる率で拡縮する | ある | `solid.scale` | factors=[x,y,z]。 |
| `setpt` | 点の一部の座標を揃える | 一部 | `sketch.move_point`, `sketch.move3d` | スケッチ点の位置変更。任意面・エッジ・曲線部分を同時に座標合わせする操作は未登録。 |

## 実装の優先順位に使う判断

1. **印刷のスキンと充填率を先に扱う。** 形状を作る入口は既に多いが、閉じた上下の面がない出力は実用印刷の完了とは言えない。輪郭の穴、段差、薄い部品、層数上限、NaNを含む入力を解析値で検証する。
2. **独立したアプリのノード画面を、再利用するRustコアにつなぐ。** ノード配置、接続、数値変更、ノード単位の失敗表示、中止、JSON往復を新しいGUIで確認する。必要な計算だけを参照し、アプリとしての操作と実行ファイルは独立させる。コマンド数が増えても、ノード画面が使えた証明にはならない。
3. **曲線・曲面は既存能力から必要な操作を接続する。** スプライン補間点の追加とNURBSノット挿入、曲面フェアリングと曲線Smooth、レール付きスイープと任意2レールの一致は混同しない。制御点編集を追加する際は対象と保持する不変条件を先に定義する。
4. **製図とファイル形式は別の作業単位にする。** ビューDXF出力があっても、用紙、詳細ビュー、表題欄、注記、PDF出力が揃ったことにはならない。.3dmの読み書きもこのコマンド調査では合格扱いにしない。

## 後続の受け入れ確認例

この文書はテスト実装ではない。次の確認を、各機能を変更するPRの既存テスト群に加える。
期待値は解析値や独自に定義した不変条件から作る。

| 対象 | 確認する結果 |
|---|---|
| 円・押出・ブーリアン | 半径・高さ・体積が指定どおりになり、失敗時に元の文書が変わらない |
| 曲線の方向反転 | 形を保ち、端点の順序だけが変わり、保存とUndoでも保たれる |
| 制御点・ノット編集 | 許容誤差内の形状保持、次数・ノット列の妥当性、閉曲線の継ぎ目を確認する |
| 曲面の接続 | 位置・接線・曲率のどれを保証するかを指定し、境界全体を評価する |
| スナップ | 拡大率や作業平面を変えても同じ対象に吸着し、非表示の対象を拾わない |
| ノード画面 | 配置・接続・数値変更・保存往復・部分失敗・中止を実操作またはヘッドレスUIで確認する |
| 印刷 | 上下の面、穴、段差、移動と押出の区別を検証し、実機で確認するまでは「未確認」とする |

この調査の文書確認では、247主名の欠落・重複がないこと、全ての対応IDが現行の宣言に存在すること、
逆引きに旧373件と現行508件が全て入ること、plannedの差を区別していることを確認した。
