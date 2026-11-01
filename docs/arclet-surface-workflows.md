# Arclet: 曲線・曲面の操作を決める根拠

調査日: 2026-11-01。対象は独立した Rust CAD アプリ Arclet。既存の clean-room Rust コアを再利用し、Melt3D の画面に機能を足す方針には戻さない。

Rhino の公開マニュアルは操作の目的と制約を確認するために読んだ。製品のコード、アイコン、説明文、作例データは使用していない。レビューは個人の使い勝手の意見として扱う。GPL / LGPL / AGPL の実装と、商用製品の実行・自動操作は調査対象にしていない。

## 先に使える形にする操作

| 優先 | 利用者がしたいこと | 公開資料から分かること | Arclet の判断と現在の範囲 |
| --- | --- | --- | --- |
| 1 | 少ない制御点で曲線を直す | [PointsOn](https://docs.mcneel.com/rhino/8/help/en-us/commands/pointson.htm) は制御点の表示と編集を分けている。結合済み曲面の点を無条件に動かすと継ぎ目が壊れるため、対象の取り扱いに制約がある。 | 既存スケッチの制御点編集を使う。新しい `NurbsCurve` コアは任意の有効なクランプ済みノット列と正の重みを保持し、点の編集を検証する。任意のノット・有理重みを編集して保存する専用画面や、曲面全体の自由な制御網編集は未実装。 |
| 2 | 点数を減らし、形の変化を見比べる | [Rebuild](https://docs.mcneel.com/rhino/8/help/en-us/commands/rebuild.htm) は次数・点数・プレビュー・変化量を扱う。端の接線を保持する選択肢や、別曲線の構造にそろえる選択肢もある。 | 最初は端点固定、次数と点数の指定、元の曲線を残したプレビューを提供する。新しいコアは最小二乗法で近似し、両方向の標本偏差を返す。接線固定、閉曲線の周期再構築、マスター曲線への構造合わせは未実装。 |
| 3 | つながりの悪い場所を数値で知る | [GCon](https://docs.mcneel.com/rhino/8/help/en-us/commands/gcon.htm) は曲線端同士を検査する。[Curvature / CurvatureGraph](https://docs.mcneel.com/rhino/8/help/en-us/commands/curvature.htm) は曲率の大きさ・変化を観察するための別の表示を提供する。 | 曲線端の距離、接線角度、曲率ベクトル差、弧長に対する曲率ベクトル変化の差を別々に表示し、許容値を指定できるようにする。サーフェスの継ぎ目検査と曲線端検査を同じ結果として表示しない。 |
| 4 | 面を合わせる、隙間に面を張る | Rhino 8 の [MatchSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/matchsrf.htm) の記載は位置・接線・曲率。[BlendSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/blendsrf.htm) は各端の連続性に G3/G4 も用意する。両者の能力を同一視しない。 | 既存 `surface.match`、`surface.blend`、`surface.loft` を見えるメニューから呼ぶ。既存エンジンは G3 を入力できるが、一般の `inspect.continuity` が G3 を証明するわけではない。形状作成と独立した計測を必ず分ける。 |
| 5 | 曲線や曲面の凸凹を減らす | [Fair](https://docs.mcneel.com/rhino/8/help/en-us/commands/fair.htm) は形の変更許容値と端部の保持を扱う。曲率表示と併用する。 | 今回の画面は曲面の Fair を対象とし、強さ、回数、移動許容値、境界の接線保持を指定できる。既存のエネルギー最小化を使うが、変更前後のエネルギー・移動量を比較表示する画面は未実装。「最小化したので Class A」とは表示しない。 |
| 6 | 複数の曲線をまとめて扱い、必要なら分離する | [Join](https://docs.mcneel.com/rhino/8/help/en-us/commands/join.htm) は異なる次数の曲線をまとめても次数や制御点を統一しない。[Explode](https://docs.mcneel.com/rhino/8/help/en-us/commands/explode.htm) はポリカーブを構成曲線に戻す。 | 新しい `CurveChain` は順序と方向を明示した曲線の集合。結合、曲線の反転、連続性の測定を別操作にする。自動的に隙間を埋めたり滑らかにしたことにはしない。 |
| 7 | 操作後の面のうねりを見つける | [Zebra](https://docs.mcneel.com/rhino/8/help/en-us/commands/zebra.htm) は縞を使った視覚検査。解析メッシュの密度も結果の見え方に影響する。 | 既存の Zebra、曲率表示、反射表示を独立したボタンにする。縞の見た目だけで寸法誤差や連続性の合格判定を出さない。 |

[FitCrv](https://docs.mcneel.com/rhino/8/help/en-us/commands/fitcrv.htm) は、点数を指定する Rebuild と異なり、許容誤差を中心に近似し、曲率に応じたノット配置を使う。Arclet の最初の Rebuild は点数を指定する操作とし、許容誤差を保証する自動近似という名前では公開しない。

## 既存コアと今回の追加

既存部分の比較基準は `feature/node-graph-gcode` のコミット `a309790ac8e265b918da09474a02b069609ab3b5`。表の「ある」はコード・登録仕様を確認した意味で、すべての形状で成功したという意味ではない。

| 能力 | 確認した場所 | 実装の範囲と残る差 |
| --- | --- | --- |
| B-spline 基底・微分・ノット挿入 | [bspline.rs](../crates/geom/src/bspline.rs)、[spline_space.rs](../crates/geom/src/spline_space.rs) | 再利用できる。対応次数は 1–7。新しい曲線にも同じ基底評価を使う。 |
| 有理テンソル積曲面 | [NurbsSurface](../crates/geom/src/bspline.rs) | 重み付き制御網・点評価・ノット挿入がある。これだけで面の編集 UI や B-rep 継ぎ目保持が完成するわけではない。 |
| 曲線作成・既存スケッチの点編集 | [sketch_create.rs](../crates/engine/src/cmd/sketch_create.rs)、[sketch.rs](../crates/engine/src/cmd/sketch.rs) | Fit Point / Control Point Spline、挿入、反転、点のドラッグがある。 |
| 既存スケッチの保存形式 | [CurveKind](../crates/sketch/src/model.rs) | スプラインの点 ID・次数・制御点モード・周期フラグは保存するが、任意ノット列と有理重みの欄はない。新しい `NurbsCurve` をここに無損失で格納できるとは宣言しない。 |
| 曲面のブレンド・ロフト・マッチ | [surface.rs](../crates/engine/src/cmd/surface.rs)、[surface_match.rs](../crates/engine/src/cmd/surface_match.rs)、[bridge.rs](../crates/kernel/src/bridge.rs)、[matchsurf.rs](../crates/kernel/src/matchsurf.rs) | G1/G2/G3 を指定する処理がある。参照面からの微分取得や継ぎ目の判定には標本・数値差分も使う。入力指定と厳密保証を同じ意味にしない。 |
| 境界ネットワーク | [surface_boundary.rs](../crates/engine/src/cmd/surface_boundary.rs)、[boundary_network.rs](../crates/kernel/src/boundary_network.rs) | スケッチからのブリッジと、面のエッジを参照するネットワークでは範囲が違う。後者は G3 を明示的に拒否し、G2 まで扱う。 |
| 曲面のうねりを抑える処理 | [fair.rs](../crates/kernel/src/fair.rs) | 曲線・曲面のパラメータ微分に基づく二次エネルギー、境界拘束、移動許容値がある。弧長に関して不変なすべての平滑性指標や、製品仕上げの認証ではない。 |
| 継ぎ目の通常の検査 | [continuity.rs](../crates/kernel/src/continuity.rs)、[surface_analysis.rs](../crates/engine/src/cmd/surface_analysis.rs) | エッジ上の標本点で隙間・法線角・継ぎ目を横切る方向の法曲率差を測る。G3 の量は返さない。局所的に評価不能な標本がある場合のカバレッジ表示も今後の改善点。 |
| 曲率と反射の表示 | [sketch_inspect.rs](../crates/engine/src/cmd/sketch_inspect.rs)、[surface_analysis.rs](../crates/engine/src/cmd/surface_analysis.rs) | `inspect.curvature_comb`、`inspect.curvature_map`、`inspect.zebra`、`inspect.environment_map` がある。 |
| 今回追加した曲線基盤 | [nurbs_curve.rs](../crates/geom/src/nurbs_curve.rs) | `NurbsCurve`、3 階までの解析微分、弧長微分、`CurveChain`、端点固定 Rebuild、両方向の標本偏差。保存形式と一般曲面の G3 検査は別の仕事。 |
| 今回追加したスケッチ接続 | [sketch_rebuild.rs](../crates/engine/src/cmd/sketch_rebuild.rs)、[入出力スキーマ](../crates/engine/src/schema/sketch_rebuild.rs) | `sketch.spline.rebuild` は非周期の Fit / CV Spline を読み、新しい CV Spline を作る。元の曲線・点・拘束は残す。出力は既存文書で保存できる等間隔ノット・重み 1 の表現。`inspect.curve_continuity` は有限線分と非周期スプラインの端を読み、形状を変えずに方向を指定して測る。 |

### 計測で区別すること

曲線の正則な端では、位置をそろえた後、単位接線 `dC/ds`、曲率ベクトル `d²C/ds²`、その弧長微分 `d³C/ds³` を比較する。ここで `s` は曲線の長さ。元のパラメータに関する `C'` の長さが違うだけで G1 不連続にはしない。3 次元では曲率の大きさだけが同じでも曲率の向きが違う場合があるため、ベクトルで比較する。

新しい検査値は順に mm、degree、1/mm、1/mm²。上位の判定には下位の許容値をすべて満たす必要がある。接線がゼロの点や数値範囲外の結果は評価不能とする。これは曲線端の局所検査であり、サーフェス全面の G3・自己交差の不在・Class A 品質の証明には使わない。式と解析例は [新しい曲線モジュールのテスト](../crates/geom/src/nurbs_curve.rs) にある。

比較基準の `kernel::seam_continuity` は次数を選ぶ条件に隙間を含めていなかった。今回この判定も修正し、位置の許容値を外れた場合は連続性を `None` として返す。明示的な位置許容値を指定する `seam_continuity_with_tolerance` を追加し、簡易 API の既定値は 1e-6 mm とした。離れた平行平面が、法線・曲率差がゼロという理由で G3 と判定されないことを回帰試験で確認する。評価不能な法線や曲率もゼロ扱いにせず、エラーを返す。

Rebuild は入力の正規化パラメータで標本を取り、両端を固定した最小二乗問題を QR 分解で解く。出力は重み 1、等間隔の内部ノットを持つ曲線。戻り値の `parameter_rms_mm` は対応パラメータ間の RMS で、幾何学的な最近点距離ではない。

`SampledDeviation` は「元の標本点 → 再構築した折れ線」と逆方向の最大距離を別々に返す。標本の間の細かい形状は見落とし得るため、Hausdorff 距離の認証値や絶対的な上限とは呼ばない。UI は `Estimated deviation` と両方向の推定偏差を表示し、標本間の細部を見落とし得る旨を伝える。今回の UI では近似用 129 点・検査用 257 点が固定値で、標本数の表示・変更欄は未実装。確定前に元の曲線と重ねて見比べる。

## PolySplines と Curve chain を混同しない

この追加の `CurveChain` は複数の曲線を扱うデータ型であり、メッシュから曲面を作る技術ではない。製品・研究の PolySplines と同等だとは表示しない。

[Plasticity の公式 PolySplines マニュアル](https://doc.plasticity.xyz/solid/polyspline) は、メッシュからトリムされていない滑らかな B-spline パッチを作る操作と説明し、G2 連続性を掲げる。四角形中心の入力を推奨し、三角形・多角形も対象とする。現在のページには結果の制御点編集、パッチの統合、境界を正確に補間する選択肢もある。これは製品側の仕様説明であり、Arclet で同じ品質を検証したという意味ではない。

一次研究 [Polyhedral Splines for Analysis, 2023](https://arxiv.org/abs/2304.12393) は、通常の格子だけでなく、星形・多角形・極・T 字接続を含む制御網から区分多項式の曲面を構成する。これは一列に曲線をつなぐデータ構造とは対象が異なる。製品の特定バージョンがこの論文のどのアルゴリズムを使うかは、この論文だけでは確定しない。

再利用候補は既存 [T-spline モジュール](../crates/geom/src/tspline/mod.rs)、[Bézier 抽出](../crates/geom/src/tspline/extract.rs)、[spline_space](../crates/geom/src/spline_space.rs)。現在の T-spline は長方形のインデックス領域と奇数次数を扱い、任意トポロジーの extraordinary vertex を備えた入力メッシュ全般の変換器ではない。局所的な基底、ノットの細分、Bézier パッチの評価は部品として再利用できるが、多価頂点付近の構成、境界・稜線、パッチ間の連続性、B-rep への結合は別途設計と検証が必要。

現段階ではこの研究課題を Rebuild の完成条件に混ぜない。まず実際に編集できる有理曲線と定量検査を完成させる。

## 使い勝手のレビューから採る判断

[Creative Bloq の Plasticity レビュー（2025-08-07）](https://www.creativebloq.com/3d/3d-software/forget-what-you-know-about-cad-plasticity-has-made-3d-modelling-fun-again) は、フィレット・ブーリアンなどの日常作業と検索パレットを評価する一方、選択に応じて変わる UI とショートカットへの依存は習得の障害にもなったと述べている。この使い勝手の意見を採り、Arclet では名前付きの入口を残し、キーボード操作を追加の近道にする。レビューを現在の機能一覧や数値精度の根拠にはしない。

[Rhino フォーラムの利用者報告（2024-02-22）](https://discourse.mcneel.com/t/rhino-after-8-years-long/176370) は、押し出し・回転・ロフト・配列・ブーリアンの往復と、散在する操作を探す負担を挙げる。これも一人の体験であり性能比較ではない。Arclet は `Curve`、`Surface`、`Solid`、`Inspect` の見える入口と、現在使える操作の理由を優先する。NURBS 編集のために重いマテリアルやレンダリングを必須にしない。

採用順は、制御点編集 → Rebuild の比較プレビュー → 曲線端の数値検査 → 既存の面作成・面解析への導線。コマンド数を増やす前に、この往復を保存・取り消し・中止まで含めて完結させる。

## 最小の UI 接続

以下は接続の設計方針であり、全項目が完成したという一覧ではない。今回接続したのは、既存スケッチの Rebuild と曲線端検査、既存面の Match / Fair のパネル。任意のノット・有理重みを直接編集して無損失保存する画面は後続の課題とする。

1. 曲線の次数、ノット、点、重みを専用のデータとして保持し、読み込みと編集のたびに `NurbsCurve::new` / `with_control_point` で検証する。デシリアライズだけで検証を飛ばさない。
2. 有限個の表示標本と制御多角形を描く。制御点の選択と座標編集は同じ曲線データを操作する。カメラ操作は既存のナビゲーション処理を通す。
3. スケッチでは `sketch.spline.rebuild` を中止可能なジョブで実行し、プレビュー中は元の曲線を保持する。確定時だけ取り消し履歴に一つの変更を追加する。ブラウザでも長い処理を UI フレーム内で一括実行しない。出力の `construction` は標準で `true` とし、重なった輪郭がプロファイル選択に混ざることを防ぐ。
4. 曲線を二つ選んだら `inspect.curve_continuity` を呼び、向きを明示する。順序のある複数曲線はコアの `CurveChain::joins` で一括検査できる。反転は測定の設定または `reversed` で明示的に行う。
5. 既存スケッチへの変換で重みやノットが失われる場合は拒否するか、明示的な近似として偏差を表示する。保存と再読込の無損失試験を通すまでは、一般 NURBS の CAD 文書保存が完成したとは報告しない。

新規コアの上限は曲線 1 本 1,024 制御点、鎖 256 本・合計 16,384 制御点、Rebuild 64 制御点・2,049 近似用標本・1,025 検査標本。加えて基底評価・QR・距離計算の合計仕事量を制限する。既存の Fit Spline は三次 Bézier 区間を正確に並べた NURBS に変換して測る。変換後が 1,024 制御点を超える場合、周期スプライン、対応範囲外の次数は明示的に拒否する。これらは軽い初期実装の上限であり、商用製品の上限を模したものではない。
