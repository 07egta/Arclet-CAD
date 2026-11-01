# Arclet: Plasticity の操作調査と採用する方針

確認日: 2026-11-01。公開マニュアルと利用者レビューから、操作の考え方だけを調べた。
製品コード、アイコン、説明文は取り込まない。Arclet は独立したアプリとして実装する。
以下の「確認」は文書での確認であり、Plasticity を操作して測定した結果ではない。

## 判断

少ない常設ツール、選択に応じた操作、直接編集、数値入力を組み合わせる。
頻出機能は短い英語名のボタンから到達できるようにし、コマンド検索も常に使えるようにする。
操作の学習をショートカットの暗記に依存させない。

公式 [Resources](https://www.plasticity.xyz/resources) が案内するマニュアルを一次資料とした。
レビューは、発見しやすさや操作の印象についての個人の経験として扱った。
既存機能との対応はこの作業ブランチのソースを確認したもので、製品と同等の機能や操作品質を保証しない。

## 採用の優先順位

| 優先 | 公開仕様から確認した操作 | Arclet の方針 | このブランチの対応と残り |
| --- | --- | --- | --- |
| P0 | 点、エッジ／曲線、面、ボディという選択モードをボタンで切り替え、複数モードも組み合わせられる。[Selection Mode](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/selection-mode) | 選択対象を見える状態にする。選べない理由を隠さず、Bodies / Faces / Edges / Points へ名前で到達できるようにする。 | Arclet の Select メニューから既存 `toolbar::selection_filter` を利用する。選択件数と現在の対象をまとめて見せる改善は次段階。 |
| P0 | コマンド名の検索、候補の絞り込み、お気に入り登録がある。コマンドバーには選択種類に応じた提案もある。[Command Palette](https://doc.plasticity.xyz/plasticity-essentials/plasticity-interface/command-palette)、[Command Bar](https://plasticity-document-revisions.vercel.app/plasticity-essentials/plasticity-interface/command-bar) | 名前つきの主要ツール、常設コマンド欄、All tools を入口にする。選択に応じた提案は追加の近道として扱い、探している機能そのものを隠さない。 | `apps/arclet/src/app.rs` のメニューとコマンド欄、`commands.rs` の名前／エイリアス候補を利用。お気に入りは後続の改善。 |
| P0 | 移動ギズモに加え、XYZ 数値、基準点、向き、軸／平面制限を指定でき、ボタンまたは右クリックで確定する。[Move](https://doc.plasticity.xyz/common/move) | ドラッグによる概形と数値による仕上げを同じ操作で行う。現在の軸と数値を見せ、左クリックで編集、右クリックで確定する。 | Transform メニューは `solid.move`、`solid.scale`、`solid.mirror` に接続。既存フォームとギズモを用いる。選択を変えたときの値の保持と取り消しは、実際の操作試験で確認する。 |
| P0 | Boolean は対象と道具を分けて選び、加算・差分・共通部分を切り替えられる。道具を残す設定がある。[Boolean](https://doc.plasticity.xyz/solid/boolean) | Combine の対象と道具を役割別に表示する。結果のプレビューを見て決められ、Keep Tools の状態を明示する。 | `solid.combine` を Solids メニューに公開。カーネルの対応範囲と、各入力のエラーは既存エンジンに従う。マニュアルにある高度な sheet/solid 混合規則を実装済みとは扱わない。 |
| P1 | エッジを選択して丸め／面取りを調整できる。数値、接線方向のエッジ追加、可変位置などの設定がある。[Fillet Shell](https://doc.plasticity.xyz/solid/fillet-shell) | Fillet と Chamfer を名前で選び、半径／距離、対象エッジ、連続エッジの範囲を見せる。正負だけで処理が切り替わる隠れた規則を初期操作にはしない。 | `solid.fillet` と `solid.chamfer` を公開。選択の追加、プレビュー、失敗後に値を直す操作を既存フォームで確認する。可変丸めの品質を製品同等とは主張しない。 |
| P1 | Focus は選択物を画面に合わせ、Isolate は他の物を一時的に隠す。隔離状態の表示と解除操作がある。[Focus and Isolate](https://doc.plasticity.xyz/plasticity-essentials/focus-and-isolate-objects) | Fit と選択への Focus を区別する。Isolate と再表示を近くに置き、隔離中であることを明示する。 | View に Fit、Isolate、Show all を追加。既存の `ui.isolate` と `browser.visibility` を使う。Show all は全体を表示する操作で、隔離前の表示状態や階層的な隔離履歴の復元は未実装。 |
| P1 | 26.2 では Boolean / Hollow / Thicken が失敗した場所を表示する。[26.2 Release Notes](https://plasticity-document-revisions.vercel.app/release-notes/whats-new-2026.2) | 失敗時に入力と選択を保ち、短い原因と修正できる数値を表示する。エンジンが場所を返せる場合だけ、その部分を強調する。 | ノードはノード単位のエラーを表示する。CAD の幾何位置に結びつく診断は後続課題。場所が不明なエラーに推測の赤表示を付けない。 |
| P2 | PolySplines はメッシュから滑らかな曲面を作る機能であり、複数の曲線を連ねる操作ではない。現行マニュアルでは PolySpline の CV 編集も説明される。[PolySplines](https://doc.plasticity.xyz/solid/polyspline) | 通常の曲線連結は CurveChain と呼ぶ。メッシュから曲面への変換、NURBS の直接編集、曲面の連続性評価は別の機能として仕様と試験を持つ。 | CurveChain は独自の曲線操作。PolySplines 相当の変換は未実装。既存の surface 制作・解析機能を入口として整える。 |

## レビューから得た操作上の課題

[Creative Bloq のレビュー（2025-08-07）](https://www.creativebloq.com/3d/3d-software/forget-what-you-know-about-cad-plasticity-has-made-3d-modelling-fun-again)
では、画面の簡潔さと検索を評価する一方、選択で使える機能が変わることやホットキーへの依存が、
そのレビュアーにとって導入時の障害になったと述べている。
Arclet ではこの意見を、常設の名前つき入口と検索候補を残す理由にする。

[利用者レビュー（2026-09-04）](https://monohoshi.blog/plasticity/)
では、面やエッジを直接動かす作業、お気に入りに登録したコマンドの呼び出し、
コマンド実行中の設定の確認が紹介されている。
Arclet では「大まかな形を作る → 直接編集する → 数値と解析で仕上げる」を短い練習作業にする。
レビューの使いやすさや軽さの感想を、実測した性能として引用しない。

## バージョンと機能名の注意

26.2 のリリース日は公式文書で 2026-10-19 と確認した。同文書では、PolySpline の CV 編集と、
Studio 向け Constraint System が追加された。拘束は当該文書上、ファイルへの保存に未対応とされる。
そのため、2025 年や 2026 年 9 月のレビューの「拘束がない」という記述を、現在の全機能の説明には使わない。
出典: [26.2 Release Notes](https://plasticity-document-revisions.vercel.app/release-notes/whats-new-2026.2)。

PolySplines の公開仕様は、メッシュから G2 連続の滑らかな B-spline 曲面パッチを生成するというもの。
主に四角形のメッシュが推奨され、境界やパッチ結合の選択肢もある。
これは CurveChain の仕様ではない。製品固有の内部アルゴリズムを推測して移植せず、
将来取り組む際は独自の仕様、許諾を確認した論文、再現可能な幾何テストから設計する。
出典: [PolySplines](https://doc.plasticity.xyz/solid/polyspline)。

## Arclet で確認する短い作業

1. 何も覚えていない状態で、Box を作り、Move / Rotate を数値で変更する。
2. 二つの立体で Combine の対象と道具を選び、加算・差分・共通部分と Keep Tools を確かめる。
3. エッジを選んで Fillet / Chamfer をかけ、不可能な大きさを入力し、値を戻して続行する。
4. 物が多い状態で選択フィルタ、Focus、Isolate、Restore を使い、選択と表示を取り戻す。
5. 保存し、閉じ、再度開いて形と数値を確認する。ブラウザーは保存コピーのダウンロードを確認する。

これらは受け入れ試験の計画であり、完了報告ではない。Intel 内蔵 GPU での応答性、
WebGPU / WebGL の実行、Bambu A1 / A1 mini での印刷はそれぞれ別途確認する。
ビルド成功のみを操作確認や実機印刷の成功としない。
