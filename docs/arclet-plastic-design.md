# Arclet CAD — 樹脂製品設計フィーチャの検証（Mission 26）

## 対象と実装済み機能

Arcletの既存樹脂形状フィーチャを改善する。UI全体・B-repカーネル全体・汎用解析エンジンは変更しない。

| 設計対象 | 既存のエンジンコマンド | 対応 |
| --- | --- | --- |
| ネジ用ボス・リブ付きボス | `plastic.boss` | 穴・抜き勾配・根元フィレット・リブ配置の入力検証を強化 |
| 補強リブ／ウェブ | `solid.rib`、`solid.web` | 既存のスケッチ曲線ベース形状生成を再利用 |
| ケース合わせ面・溝 | `plastic.lip` | `type:lip` または `type:groove` を使用、既存のShellへ接続 |
| カンチレバー式スナップ | `plastic.snap_fit` | 既存のキャッチ／アーム、別ボディ生成を再利用 |
| 支持台・中空支持台 | `plastic.rest` | テーパーで壁または内側開口が消える条件を検査 |
| 抜き勾配 | `plastic.boss`、`plastic.rest`、`solid.draft` | 専用フィーチャの成立性を検証、汎用Draftは維持 |
| ケースの肉抜き | `solid.shell` | 既存実装を維持 |
| 材料・寸法ルール | `plastic.manage_rules`、`plastic.assign_rule`、`plastic.rules` | 不正限界値の拒否・名目警告 |
| 樹脂設計診断 | `plastic.audit` | 今回追加した名目寸法専用の読み取りコマンド |

**注意：`plastic.audit` はB-rep全域をサンプリングした実測肉厚検査ではない。**
入力した幾何寸法の解析値とルール値を比較するだけで、結果は
`geometry_thickness_sampled: false` および
`molding_feasibility_certified: false` を明記する。
実際の局所肉厚や成形可能性は保証しない。

## 今回修正した点

1. **ルール内のThickness参照**：設計全体に同名パラメータがある場合も、ルール内の大文字 `Thickness` は同ルール自身の肉厚に固定する。
2. **厚さルールの自己矛盾**：最小肉厚が0以下、または最大肉厚を超える定義を拒否する。名目寸法だけが推奨範囲外の場合は互換性を保って警告する。
3. **Bossの穴と勾配**：円筒穴がテーパーの根元・先端に収まるか評価し、穴深さがボス高さを超える定義を拒否する。
4. **Bossの無効入力**：負の根元フィレット、約57°以上の抜き勾配、小数のリブ本数、負のリブオフセットを拒否する。
5. **中空Restのテーパー**：正の抜き勾配では外周が縮み穴が広がるため、頂部の肉厚消失を拒否する。負の勾配では内側開口の消失と根元の薄肉化を別に検証する。

## 実設計シナリオと数値オラクル

次は自動テストが検証する**期待値**。CIが成功するまでは実行結果ではない。

| シナリオ | 数値・合格条件 | テスト |
| --- | --- | --- |
| ネジボス付きパネル | 60×60×3 mm、外径10、穴径4、高さ12：ボス追加体積252π mm³ | `boss_with_hole_fillet_and_ribs` |
| ボス直径変更 | 外径10→12：追加体積252π→384π mm³、穴周り名目肉厚3→4 mm | `plastic_audit_tracks_parametric_boss_wall_and_roundtrip` |
| ボスのリブ4本 | 既存ボスのリブの体積増分を数値で検査 | `boss_with_hole_fillet_and_ribs` |
| Shell後のLip | 80×50×30、シェル2 mm、Lip幅1・高さ2：追加体積496 mm³ | `lip_and_groove_on_a_shelled_box` |
| Shell後のGroove | 上記に溝幅1・深さ2：除去体積512 mm³ | `lip_and_groove_on_a_shelled_box` |
| スナップ嵌合 | アーム15×1.5×6＋キャッチ：追加体積144 mm³ | `snap_fit_arm_and_catch` |
| 抜き勾配付き中空Rest | 20×10、壁厚1、高さ6、勾配2°で成立。高さ16では壁が消失するため拒否。負の勾配でも開口消失を拒否 | `plastic_preflight_rejects_bad_boss_and_rest_without_mutation` |
| 上下ケース | ケース、Shell、複数スタンドオフ、コネクタ穴、スナップ、蓋、ねじ穴 | `examples/acceptance/sensor_enclosure.json` |
| 再計算・再読み込み | ボスの直径変更後の体積、JSON設計の保存相当往復後の再評価 | `plastic_audit_tracks_parametric_boss_wall_and_roundtrip` |
| 失敗時の原状維持 | 無効な穴深さ・フィレット・リブ数・中空開口で旧モデルを維持 | `plastic_preflight_rejects_bad_boss_and_rest_without_mutation` |

診断の呼び出し例：

~~~json
{"command":"plastic.audit","params":{"bodies":["Plate"]}}
~~~

主な返却値：`body_rules`、`feature_checks`、`nominal_mm`、
`min_thickness_mm`、`max_thickness_mm`、`warnings`。
Bossの計算上の半径方向肉厚は、外径、穴径、ボス高さ、
抜き勾配に基づく両端での薄い側の値。根元フィレット、
Boolean後の肉厚分布、局所的な厚肉溜まりは測定しない。
ボディの明示的な関連付けがないフィーチャは、ボディ指定の監査で
推測して誤報することを避ける。

## 回帰テスト

~~~sh
cargo test --profile ci --locked -p solidcraft-doc --lib plastic
cargo test --profile ci --locked -p solidcraft-engine --lib cmd::plastic::tests
cargo xtask ci
~~~

専用ワークフロー `.github/workflows/arclet-plastic.yml` は
Windows/Linuxの両方で上2つのテストを実行する。
全体チェックは既知の他領域の失敗と分けて判断する。

## 製造条件として未検証の事項

- 実材料の収縮率、吸湿、繊維配向、冷却・反り。
- 金型のパーティングライン、ランナー、ゲート、エジェクタ、スライド金型。
- 抜き方向の全周確認とアンダーカット解析、実型との合わせ代。
- ネジ締付トルク、ボス割れ、スナップ保持力・変形量・疲労、応力集中。
- 実B-repの局所肉厚サンプリング、最薄部・最厚部の統計、フィレット後の断面。
- 実機寸法公差、加工条件、試作・量産の金型検証。

これらは材料特性と金型条件を使った専用解析・実物評価が必要。
GPL/LGPL/AGPLコードは本ミッションで追加しない。
