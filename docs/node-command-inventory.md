# コマンド比較の逆引き一覧

[比較表と判定の意味](node-command-comparison.md)を先に読む。
基準コミット: `a309790ac8e265b918da09474a02b069609ab3b5`。

旧`docs/features.md`の**373 ID全て**と、現行の静的登録**508 ID全て**を含む**511 ID**の一覧。
「宣言あり」は`command_specs()`が参照する`COMMANDS`配列にあるという意味で、実行成功を保証しない。
「旧一覧のみ」は3件。「planned」は現行の宣言で未実装指定の1件。旧一覧側にもplanned表記を別に残す。
対応主名が「—」の行は、247行の対照表で直接割り当てなかったもの。Serpentineに関連する機能がないという断定ではない。

各行の出典リンクは同じ固定コミットを指す。アプリ操作・幾何品質・印刷は**未確認**。
ラベルはこのリポジトリ自身のコマンド名であり、外部製品の説明文は含めていない。

## appearance（5 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `appearance.assign` | Appearance | 宣言あり | 掲載 | `ghosted`, `rendered`, `material`, `matchprops` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L47) |
| `appearance.chain` | Appearance Chain | 宣言あり | 未掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L52) |
| `appearance.library` | Appearance Library | 宣言あり | 掲載 | `material` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L58) |
| `appearance.list` | List Appearances | 宣言あり | 掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L61) |
| `appearance.replace` | Replace Appearance | 宣言あり | 未掲載 | `matchprops` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L55) |

## assembly（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `assembly.bom` | Bill of Materials | 宣言あり | 未掲載 | `count` | [bom.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/bom.rs#L36) |

## body（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `body.rename` | Rename Body | 宣言あり | 掲載 | `rename` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L21) |

## browser（8 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `browser.collapse_all` | Collapse All | 宣言あり | 未掲載 | — | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L58) |
| `browser.expand_all` | Expand All | 宣言あり | 未掲載 | — | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L53) |
| `browser.group` | New Group | 宣言あり | 掲載 | `layer`, `group` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L25) |
| `browser.move` | Move Browser Items | 宣言あり | 掲載 | `layer`, `changelayer` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L30) |
| `browser.order` | Order Browser Items | 宣言あり | 掲載 | `bringforward`, `bringtofront`, `sendbackward`, `sendtoback` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L44) |
| `browser.rename_group` | Rename Group | 宣言あり | 掲載 | — | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L29) |
| `browser.ungroup` | Ungroup | 宣言あり | 掲載 | `explode`, `ungroup` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L28) |
| `browser.visibility` | Show/Hide | 宣言あり | 掲載 | `hide`, `layer`, `show`, `isolate`, `unisolate` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L45) |

## canvas（6 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `canvas.calibrate` | Calibrate Canvas | 宣言あり | 掲載 | `pictureframe` | [sketch_canvas.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_canvas.rs#L21) |
| `canvas.decal` | Decal | 宣言あり | 掲載 | — | [sketch_canvas.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_canvas.rs#L16) |
| `canvas.delete` | Delete Canvas | 宣言あり | 掲載 | — | [sketch_canvas.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_canvas.rs#L23) |
| `canvas.edit` | Edit Canvas | 宣言あり | 掲載 | `pictureframe` | [sketch_canvas.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_canvas.rs#L20) |
| `canvas.insert` | Canvas | 宣言あり | 掲載 | `pictureframe` | [sketch_canvas.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_canvas.rs#L12) |
| `canvas.to_sketch` | Canvas to Sketch | 宣言あり | 未掲載 | — | [canvas_sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/canvas_sketch.rs#L21) |

## component（27 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `component.activate` | Activate Component | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L50) |
| `component.break_link` | Break Link | 宣言あり | 掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L44) |
| `component.capture_position` | Capture Position | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L119) |
| `component.copies` | Find Identical Copies | 宣言あり | 未掲載 | `seldup` | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L52) |
| `component.copy_bodies` | Paste Bodies | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L96) |
| `component.create` | New Component | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L43) |
| `component.cycle_color` | Cycle Component Color | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L40) |
| `component.delete` | Delete Component | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L86) |
| `component.edit_in_place` | Edit in Place | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L51) |
| `component.extract` | Extract to File | 宣言あり | 未掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L47) |
| `component.finish_edit` | Finish Edit | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L64) |
| `component.from_bodies` | Create Components from Bodies | 宣言あり | 掲載 | `block` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L47) |
| `component.insert_design` | Insert Design | 宣言あり | 掲載 | `insert` | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L32) |
| `component.links` | List Links | 宣言あり | 掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L60) |
| `component.list` | List Components | 宣言あり | 掲載 | `blocklist`, `count` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L75) |
| `component.metadata` | Part Properties | 宣言あり | 未掲載 | `getusertext` | [bom.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/bom.rs#L25) |
| `component.metadata.set` | Change Part Properties | 宣言あり | 未掲載 | `setusertext` | [bom.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/bom.rs#L29) |
| `component.move_bodies` | Move Bodies to Component | 宣言あり | 掲載 | `changelayer` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L91) |
| `component.move_sketches` | Move Sketches to Component | 宣言あり | 掲載 | `changelayer` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L102) |
| `component.paste_new` | Paste New | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L105) |
| `component.properties` | Component Physical Properties | 宣言あり | 掲載 | `what` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L79) |
| `component.relink` | Relink Source | 宣言あり | 未掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L41) |
| `component.rename` | Rename Component | 宣言あり | 掲載 | `rename` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L85) |
| `component.replace_with_link` | Replace with Link | 宣言あり | 未掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L55) |
| `component.revert_position` | Revert Position | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L120) |
| `component.unlock` | Unlock Contents | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L87) |
| `component.update_links` | Update Links | 宣言あり | 掲載 | — | [component_link.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component_link.rs#L37) |

## config（8 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `config.activate` | Activate Configuration | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L25) |
| `config.column` | Configuration Column | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L21) |
| `config.configure` | Configure | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L13) |
| `config.preview` | Preview Configuration | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L26) |
| `config.row` | Configuration Row | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L22) |
| `config.rule` | Configuration Rule | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L27) |
| `config.rules` | Configuration Rules | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L30) |
| `config.table` | Display Configuration Table | 宣言あり | 掲載 | — | [config.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/config.rs#L16) |

## construct（21 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `construct.axis.edge` | Axis Through Edge | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L39) |
| `construct.axis.normal_to_face` | Axis Perpendicular To Face | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L27) |
| `construct.axis.through_cylinder` | Axis Through Cylinder/Cone/Torus | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L23) |
| `construct.axis.two_planes` | Axis Through Two Planes | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L31) |
| `construct.axis.two_points` | Axis Through Two Points | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L35) |
| `construct.plane.along_path` | Plane Along Path | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L83) |
| `construct.plane.at_angle` | Plane at Angle | 宣言あり | 掲載 | — | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L144) |
| `construct.plane.from_view` | Plane from View (Ctrl+Shift+Space) | 旧一覧のみ・未確認 | 掲載 | — | [旧一覧](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/features.md) |
| `construct.plane.from_view_origin` | Plane from View Through Origin (Ctrl+Space) | 旧一覧のみ・未確認 | 掲載 | — | [旧一覧](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/features.md) |
| `construct.plane.midplane` | Midplane | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L67) |
| `construct.plane.offset` | Offset Plane | 宣言あり | 掲載 | `cplane` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L140) |
| `construct.plane.perpendicular` | Perpendicular Plane | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L87) |
| `construct.plane.tangent` | Tangent Plane | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L79) |
| `construct.plane.three_points` | Plane Through Three Points | 宣言あり | 掲載 | `cplane` | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L75) |
| `construct.plane.two_edges` | Plane Through Two Edges | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L71) |
| `construct.point.along_path` | Point Along Path | 宣言あり | 掲載 | `divide` | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L63) |
| `construct.point.center` | Point At Center Of Circle/Sphere/Torus | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L55) |
| `construct.point.edge_plane` | Point At Edge And Plane | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L59) |
| `construct.point.three_planes` | Point Through Three Planes | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L51) |
| `construct.point.two_edges` | Point Through Two Edges | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L47) |
| `construct.point.vertex` | Point At Vertex | 宣言あり | 掲載 | — | [construct.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/construct.rs#L43) |

## contact（7 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `contact.check` | Check Contact | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L53) |
| `contact.create` | New Contact Set | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L45) |
| `contact.disable_all` | Disable Contact | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L44) |
| `contact.edit` | Edit Contact Set | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L49) |
| `contact.enable_all` | Enable All Contact | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L40) |
| `contact.enable_sets` | Enable Contact Sets | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L37) |
| `contact.list` | List Contact Sets | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L52) |

## diagnostics（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `diagnostics.crash_report` | Crash Report | 宣言あり | 掲載 | — | [diagnostics.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/diagnostics.rs#L16) |
| `diagnostics.crash_reports` | Crash Reports | 宣言あり | 掲載 | — | [diagnostics.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/diagnostics.rs#L11) |
| `diagnostics.crash_reports.clear` | Delete Crash Reports | 宣言あり | 掲載 | — | [diagnostics.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/diagnostics.rs#L21) |
| `diagnostics.crash_test` | Crash Test | 宣言あり | 掲載 | — | [diagnostics.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/diagnostics.rs#L25) |

## doc（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `doc.open` | Open | 宣言あり | 掲載 | `open` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L26) |
| `doc.recover` | Recover Design | 宣言あり | 掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L52) |
| `doc.recovery_discard` | Discard Recovered Design | 宣言あり | 掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L55) |
| `doc.recovery_list` | Recoverable Designs | 宣言あり | 掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L49) |

## document（6 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `document.inspect` | Inspect Design | 宣言あり | 掲載 | `audit`, `count`, `what`, `selname` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L28) |
| `document.metadata` | Design Properties | 宣言あり | 掲載 | `getusertext` | [metadata.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/metadata.rs#L16) |
| `document.metadata.remove` | Remove Design Property | 宣言あり | 掲載 | — | [metadata.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/metadata.rs#L25) |
| `document.metadata.set` | Change Design Properties | 宣言あり | 掲載 | `setusertext` | [metadata.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/metadata.rs#L20) |
| `document.revision` | Design Revision | 宣言あり | 未掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L29) |
| `document.units` | Change Design Units | 宣言あり | 掲載 | `units` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L32) |

## edit（5 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `edit.checkpoint` | Checkpoint | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L23) |
| `edit.redo` | Redo | 宣言あり | 掲載 | `redo` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L14) |
| `edit.repeat` | Repeat Last Command | 宣言あり | 未掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L15) |
| `edit.restore_checkpoint` | Restore Checkpoint | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L27) |
| `edit.undo` | Undo | 宣言あり | 掲載 | `undo` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L13) |

## engine（2 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `engine.commands` | List Commands | 宣言あり | 掲載 | `help` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L68) |
| `engine.describe` | Describe Command | 宣言あり | 未掲載 | `help` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L69) |

## explode（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `explode.create` | Exploded View | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L106) |
| `explode.delete` | Delete Exploded View | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L115) |
| `explode.list` | List Exploded Views | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L114) |
| `explode.show` | Show Exploded View | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L110) |

## expr（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `expr.evaluate` | Evaluate Expression | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L58) |

## face（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `face.delete` | Delete Face | 宣言あり | 掲載 | — | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L120) |

## file（11 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `file.export` | Export | 宣言あり | 掲載 | `export` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L56) |
| `file.export_view_dxf` | Export Current View as DXF | 宣言あり | 未掲載 | `technical`, `exportdxf`, `make2d` | [view_dxf.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_dxf.rs#L23) |
| `file.insert_mesh` | Insert Mesh | 宣言あり | 掲載 | `import` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L35) |
| `file.insert_step` | Insert STEP | 宣言あり | 掲載 | `import` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L31) |
| `file.new` | New Design | 宣言あり | 掲載 | `new` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L25) |
| `file.open_in_easyprint` | Open with EasyPrint | 宣言あり | 未掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L71) |
| `file.save` | Save | 宣言あり | 掲載 | `save` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L39) |
| `file.save_as` | Save As | 宣言あり | 掲載 | `save` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L44) |
| `file.save_mesh` | Save As Mesh | 宣言あり | 掲載 | `export`, `breptomesh` | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L60) |
| `file.send_to_slicer` | Send to Slicer | 宣言あり | 未掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L65) |
| `file.slicers` | Installed Slicers | 宣言あり | 掲載 | — | [file.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/file.rs#L75) |

## frame（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `frame.member` | Frame Member | 宣言あり | 未掲載 | — | [frame.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/frame.rs#L33) |
| `frame.profile.insert` | Insert Profile | 宣言あり | 未掲載 | — | [frame.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/frame.rs#L46) |
| `frame.profile.list` | Profile Library | 宣言あり | 未掲載 | — | [frame.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/frame.rs#L38) |
| `frame.profile.properties` | Profile Properties | 宣言あり | 未掲載 | — | [frame.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/frame.rs#L42) |

## inspect（18 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `inspect.accessibility` | Accessibility Analysis | 宣言あり | 掲載 | — | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L38) |
| `inspect.center_of_mass` | Center of Mass | 宣言あり | 掲載 | — | [sketch_inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_inspect.rs#L27) |
| `inspect.clearance` | Clearance | 宣言あり | 未掲載 | — | [clearance.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/clearance.rs#L21) |
| `inspect.colored_components` | Enable Component Colors | 宣言あり | 未掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L19) |
| `inspect.compare` | Compare Bodies | 宣言あり | 未掲載 | — | [compare.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/compare.rs#L15) |
| `inspect.continuity` | Continuity Analysis | 宣言あり | 掲載 | — | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L48) |
| `inspect.curvature_comb` | Curvature Comb Analysis | 宣言あり | 掲載 | `curvaturegraph`, `curvature` | [sketch_inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_inspect.rs#L12) |
| `inspect.curvature_map` | Curvature Map Analysis | 宣言あり | 掲載 | `curvature`, `curvatureanalysis` | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L43) |
| `inspect.draft` | Draft Analysis | 宣言あり | 掲載 | `draftanalysis` | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L28) |
| `inspect.environment_map` | Environment Map Analysis | 宣言あり | 掲載 | — | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L33) |
| `inspect.interference` | Interference | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L96) |
| `inspect.isocurve` | Isocurve Analysis | 宣言あり | 掲載 | `isocurves` | [sketch_inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_inspect.rs#L22) |
| `inspect.measure` | Measure | 宣言あり | 掲載 | `angle`, `area`, `distance`, `length`, `printcheck`, `radius`, `volume`, `audit` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L22) |
| `inspect.minimum_radius` | Minimum Radius Analysis | 宣言あり | 掲載 | `curvature`, `radius` | [sketch_inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_inspect.rs#L17) |
| `inspect.parameter_sweep` | Parameter Sweep | 宣言あり | 未掲載 | — | [parameter_sweep.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameter_sweep.rs#L16) |
| `inspect.section` | Section Analysis | 宣言あり | 掲載 | `clippingplane`, `disableclippingplane`, `enableclippingplane`, `hatch`, `sectionhatch`, `contour`, `section` | [section.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/section.rs#L12) |
| `inspect.silhouette` | Silhouette | 宣言あり | 未掲載 | — | [silhouette.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/silhouette.rs#L15) |
| `inspect.zebra` | Zebra Analysis | 宣言あり | 掲載 | `zebra` | [surface_analysis.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_analysis.rs#L23) |

## joint（18 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `joint.as_built` | As-Built Joint | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L36) |
| `joint.cam` | Cam | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L74) |
| `joint.create` | Joint | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L25) |
| `joint.delete` | Delete Joint | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L81) |
| `joint.drive` | Drive Joints | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L41) |
| `joint.edit` | Edit Joint | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L78) |
| `joint.gear` | Gear | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L55) |
| `joint.inspect` | Inspect Joint | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L86) |
| `joint.limits` | Joint Limits | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L77) |
| `joint.list` | List Joints | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L82) |
| `joint.motion_link` | Motion Link | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L51) |
| `joint.origin` | Joint Origin | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L22) |
| `joint.rack_pinion` | Rack and Pinion | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L62) |
| `joint.reach` | Reach Point | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L48) |
| `joint.rigid_group` | Rigid Group | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L40) |
| `joint.screw` | Screw | 宣言あり | 未掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L68) |
| `joint.solve` | Solve Joints | 宣言あり | 掲載 | — | [joints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/joints.rs#L95) |
| `joint.sweep` | Sweep Joint | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L103) |

## library（3 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `library.apply` | Apply Local Library Entry | 宣言あり | 掲載 | — | [libraries.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/libraries.rs#L13) |
| `library.export` | Export Design Library | 宣言あり | 掲載 | — | [libraries.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/libraries.rs#L11) |
| `library.inspect` | Inspect Local Library | 宣言あり | 掲載 | — | [libraries.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/libraries.rs#L9) |

## material（8 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `material.assign` | Physical Material | 宣言あり | 掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L43) |
| `material.create` | New Physical Material | 宣言あり | 掲載 | — | [materials.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/materials.rs#L18) |
| `material.delete` | Delete Physical Material | 宣言あり | 掲載 | — | [materials.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/materials.rs#L27) |
| `material.edit` | Edit Physical Material | 宣言あり | 掲載 | — | [materials.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/materials.rs#L21) |
| `material.inspect` | Inspect Physical Material | 宣言あり | 未掲載 | — | [materials.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/materials.rs#L24) |
| `material.list` | List Materials | 宣言あり | 掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L64) |
| `material.properties.directional` | Directional Elastic Modulus | 宣言あり | 掲載 | — | [material_properties.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/material_properties.rs#L12) |
| `material.properties.validate` | Validate Physical Material Data | 宣言あり | 掲載 | — | [material_properties.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/material_properties.rs#L7) |

## mesh（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `mesh.convert` | Convert Mesh | 宣言あり | 掲載 | `meshtobrep` | [mesh.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/mesh.rs#L17) |

## model（3 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `model.edges` | List Edges | 宣言あり | 掲載 | `dir`, `what` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L35) |
| `model.faces` | List Faces | 宣言あり | 掲載 | `dir`, `what` | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L36) |
| `model.threads` | List Threads | 宣言あり | 掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L42) |

## motion（11 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `motion.delete` | Delete Motion Study | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L105) |
| `motion.export` | Export Motion Study | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L67) |
| `motion.export_animation` | Export Motion Animation | 宣言あり | 未掲載 | `turntable` | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L71) |
| `motion.interference` | Motion Interference | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L99) |
| `motion.list` | List Motion Studies | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L104) |
| `motion.play` | Play Motion Study | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L61) |
| `motion.plot` | Plot Motion Study | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L98) |
| `motion.study` | Motion Study | 宣言あり | 掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L57) |
| `motion.trace` | Motion Trace | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L83) |
| `motion.trace.edit` | Edit Motion Trace | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L94) |
| `motion.trace.keep` | Keep Motion Trace | 宣言あり | 未掲載 | — | [motion.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/motion.rs#L88) |

## occurrence（6 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `occurrence.copy` | Paste (Instance) | 宣言あり | 掲載 | `block`, `insert`, `copy` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L103) |
| `occurrence.drag` | Drag Occurrence | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L108) |
| `occurrence.ground` | Ground | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L113) |
| `occurrence.move` | Move Occurrence | 宣言あり | 掲載 | `move`, `orient3pt`, `rotate3d` | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L106) |
| `occurrence.properties` | Occurrence Physical Properties | 宣言あり | 掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L82) |
| `occurrence.reparent` | Move to Component | 宣言あり | 未掲載 | — | [component.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/component.rs#L114) |

## parameters（13 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `parameters.add` | Add User Parameter | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L29) |
| `parameters.change` | Change Parameters | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L15) |
| `parameters.comment` | Comment Parameter | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L42) |
| `parameters.complete` | Complete Expression | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L63) |
| `parameters.delete` | Delete Parameter | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L35) |
| `parameters.export` | Export Parameters | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L51) |
| `parameters.favorite` | Favourite Parameter | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L38) |
| `parameters.graph` | Parameter Graph | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L47) |
| `parameters.import` | Import Parameters | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L55) |
| `parameters.list` | List Parameters | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L25) |
| `parameters.move` | Move Parameter | 宣言あり | 未掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L39) |
| `parameters.rename` | Rename Parameter | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L32) |
| `parameters.users` | Parameter Users | 宣言あり | 掲載 | — | [parameters.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parameters.rs#L43) |

## parts（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `parts.fastener` | Insert Fastener | 宣言あり | 掲載 | — | [parts.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parts.rs#L21) |
| `parts.insert` | Insert Part | 宣言あり | 掲載 | — | [parts.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parts.rs#L24) |
| `parts.library` | Standard Parts | 宣言あり | 掲載 | — | [parts.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parts.rs#L18) |
| `parts.resize` | Change Part Size | 宣言あり | 掲載 | — | [parts.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/parts.rs#L29) |

## plastic（7 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `plastic.assign_rule` | Assign Plastic Rule | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L37) |
| `plastic.boss` | Boss | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L17) |
| `plastic.lip` | Lip | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L21) |
| `plastic.manage_rules` | Manage Plastic Rules | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L32) |
| `plastic.rest` | Rest | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L28) |
| `plastic.rules` | List Plastic Rules | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L40) |
| `plastic.snap_fit` | Snap Fit | 宣言あり | 掲載 | — | [plastic.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/plastic.rs#L24) |

## pointcloud（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `pointcloud.delete` | Delete Point Cloud | 宣言あり | 未掲載 | — | [point_cloud.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/point_cloud.rs#L20) |
| `pointcloud.edit` | Edit Point Cloud | 宣言あり | 未掲載 | `pointcloud` | [point_cloud.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/point_cloud.rs#L17) |
| `pointcloud.insert` | Insert Point Cloud | 宣言あり | 未掲載 | `import`, `pointcloud` | [point_cloud.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/point_cloud.rs#L13) |
| `pointcloud.list` | Point Clouds | 宣言あり | 未掲載 | `pointcloud`, `selpointcloud` | [point_cloud.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/point_cloud.rs#L21) |

## render（46 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `render.aging.describe` | Layered Surface Aging | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L32) |
| `render.backends` | Render Backends | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L108) |
| `render.camera.fit` | Frame Render Subject | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L46) |
| `render.camera.get` | Inspect Render Camera | 宣言あり | 未掲載 | `camera` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L44) |
| `render.camera.set` | Set Render Camera | 宣言あり | 未掲載 | `camera` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L42) |
| `render.cancel` | Cancel Render | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L102) |
| `render.effects.describe` | Lens and Creative Effects | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L34) |
| `render.export` | Export Render Image | 宣言あり | 掲載 | `viewcapturetofile` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L106) |
| `render.hdr.inspect` | Inspect HDR Source | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L92) |
| `render.library.appearance.apply` | Apply Library Opaque Appearance | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L82) |
| `render.library.describe` | Inspect Render Library Resource | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L80) |
| `render.library.environment.apply` | Apply Built-in HDR Environment | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L84) |
| `render.library.list` | Browse Render Library | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L78) |
| `render.library.physical.prepare` | Prepare Physical Reference | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L62) |
| `render.library.preview` | Inspect Original Surface Map | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L60) |
| `render.library.studio.apply` | Apply Authored Studio Lights | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L90) |
| `render.library.studio.prepare` | Prepare Authored Studio Lights | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L88) |
| `render.library.surface.bind` | Save Surface Binding | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L70) |
| `render.library.surface.prepare` | Prepare Authored Surface Maps | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L86) |
| `render.library.surface.unbind` | Remove Surface Binding | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L72) |
| `render.library.targets` | Find Render Library Targets | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L76) |
| `render.optics.bind` | Save Optical Binding | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L36) |
| `render.optics.unbind` | Remove Optical Binding | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L38) |
| `render.output.size` | Set Render Output Size | 宣言あり | 未掲載 | — | [render_ui.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render_ui.rs#L21) |
| `render.preflight` | Check Render Request | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L96) |
| `render.preset.apply` | Apply Native Studio | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L50) |
| `render.preset.resolve` | Review Native Studio | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L48) |
| `render.quality.prepare` | Review Fixed Sampling Budget | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L52) |
| `render.result.close` | Close Render Image | 宣言あり | 未掲載 | — | [render_ui.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render_ui.rs#L31) |
| `render.result.copy` | Copy Render Image | 宣言あり | 未掲載 | `viewcapturetoclipboard` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L104) |
| `render.result.pin` | Retain Displayed Render Result | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L74) |
| `render.result.regrade` | Regrade Linear Master | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L40) |
| `render.result.view` | View Render Image | 宣言あり | 未掲載 | — | [render_ui.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render_ui.rs#L26) |
| `render.scene` | Inspect Render Scene | 宣言あり | 掲載 | `rendered` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L116) |
| `render.settings.apply` | Apply Render Camera | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L118) |
| `render.settings.capture_hdr` | Save Render Settings with HDR | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L94) |
| `render.settings.get` | Render Settings | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L112) |
| `render.settings.set` | Save Render Settings | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L114) |
| `render.shot.duplicate` | Duplicate Render Shot | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L54) |
| `render.shot.remove` | Remove Render Shot | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L58) |
| `render.shot.rename` | Rename Render Shot | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L56) |
| `render.stage.backdrop` | Save Physical Render Background | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L66) |
| `render.stage.clear` | Remove Render Stage | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L68) |
| `render.stage.ground` | Save Render Ground | 宣言あり | 未掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L64) |
| `render.start` | Render Image | 宣言あり | 掲載 | `rendered` | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L98) |
| `render.status` | Render Progress | 宣言あり | 掲載 | — | [render.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/render.rs#L100) |

## select（4 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `select.clear` | Clear Selection | 宣言あり | 掲載 | `selnone` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L85) |
| `select.filter` | Selection filter | 宣言あり | 未掲載 | `selfilter`, `selfiltertoggle` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L83) |
| `select.same_look` | Select Same Look | 宣言あり | 未掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L86) |
| `select.set` | Select | 宣言あり | 掲載 | `selall`, `selcrv`, `selname`, `selpt`, `selsolid`, `selsrf` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L80) |

## selection（1 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `selection.delete` | Delete | 宣言あり | 掲載 | `delete` | [delete.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/delete.rs#L15) |

## sheet（13 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `sheet.convert` | Convert to Sheet Metal | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L52) |
| `sheet.export_bend_table` | Export Bend Table | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L95) |
| `sheet.export_dxf` | Export Flat Pattern as DXF | 宣言あり | 掲載 | `exportdxf` | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L82) |
| `sheet.flange` | Flange | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L18) |
| `sheet.flat_pattern` | Create Flat Pattern | 宣言あり | 掲載 | `unrollsrf` | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L75) |
| `sheet.fold` | Fold | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L45) |
| `sheet.hem` | Hem | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L41) |
| `sheet.manage_rules` | Sheet Metal Rules | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L65) |
| `sheet.path_flange` | Path Flange | 宣言あり | 未掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L33) |
| `sheet.perforate` | Perforate | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L38) |
| `sheet.refold` | Refold | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L51) |
| `sheet.rules` | List Sheet Metal Rules | 宣言あり | 掲載 | — | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L74) |
| `sheet.unfold` | Unfold | 宣言あり | 掲載 | `unrollsrf` | [sheet.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sheet.rs#L50) |

## sketch（113 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `sketch.arc.center_point` | Center Point Arc | 宣言あり | 掲載 | `arc` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L99) |
| `sketch.arc.tangent` | Tangent Arc | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L39) |
| `sketch.arc.three_point` | 3-Point Arc | 宣言あり | 掲載 | `arc` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L98) |
| `sketch.arc3d` | 3D Arc | 宣言あり | 未掲載 | `arc` | [sketch_3d.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_3d.rs#L25) |
| `sketch.auto_constrain` | AutoConstrain | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L29) |
| `sketch.auto_constrain_from_datum` | AutoConstrain from datum | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L34) |
| `sketch.auto_project` | Auto-Project Edges on Reference | 宣言あり | 掲載 | — | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L61) |
| `sketch.blend_curve` | Blend Curve | 宣言あり | 掲載 | `blendcrv` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L102) |
| `sketch.break` | Break | 宣言あり | 掲載 | `split` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L75) |
| `sketch.break_link` | Break Link | 宣言あり | 掲載 | `dupedge` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L58) |
| `sketch.cancel` | Cancel Sketch | 宣言あり | 未掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L43) |
| `sketch.centerline` | Centerline | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L110) |
| `sketch.chain` | Chain | 宣言あり | 掲載 | `polyline` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L210) |
| `sketch.chamfer.distance_angle` | Distance and Angle Chamfer | 宣言あり | 掲載 | `chamfer` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L33) |
| `sketch.chamfer.equal_distance` | Equal Distance Chamfer | 宣言あり | 掲載 | `chamfer` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L28) |
| `sketch.chamfer.two_distance` | Two Distance Chamfer | 宣言あり | 掲載 | `chamfer` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L38) |
| `sketch.circle.center` | Center Diameter Circle | 宣言あり | 掲載 | `circle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L78) |
| `sketch.circle.three_point` | 3-Point Circle | 宣言あり | 掲載 | `circle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L93) |
| `sketch.circle.three_tangent` | 3-Tangent Circle | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L48) |
| `sketch.circle.two_point` | 2-Point Circle | 宣言あり | 掲載 | `circle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L88) |
| `sketch.circle.two_tangent` | 2-Tangent Circle | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L43) |
| `sketch.conic` | Conic Curve | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L97) |
| `sketch.constrainer` | Sketch Constrainer | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L44) |
| `sketch.constraint.coincident` | Coincident | 宣言あり | 掲載 | `closecrv`, `join` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L138) |
| `sketch.constraint.collinear` | Collinear | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L178) |
| `sketch.constraint.concentric` | Concentric | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L173) |
| `sketch.constraint.curvature` | Curvature | 宣言あり | 掲載 | `matchcrv` | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L19) |
| `sketch.constraint.equal` | Equal | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L148) |
| `sketch.constraint.fix` | Fix/UnFix | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L163) |
| `sketch.constraint.horizontal_vertical` | Horizontal/Vertical | 宣言あり | 掲載 | `ortho` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L133) |
| `sketch.constraint.midpoint` | MidPoint | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L168) |
| `sketch.constraint.parallel` | Parallel | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L153) |
| `sketch.constraint.perpendicular` | Perpendicular | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L158) |
| `sketch.constraint.polygon` | Polygon | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L24) |
| `sketch.constraint.symmetry` | Symmetry | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L183) |
| `sketch.constraint.tangent` | Tangent | 宣言あり | 掲載 | `matchcrv` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L143) |
| `sketch.construction` | Normal/Construction | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L188) |
| `sketch.copy` | Copy Sketch Entities | 宣言あり | 未掲載 | — | [sketch_clipboard.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_clipboard.rs#L21) |
| `sketch.create` | Create Sketch | 宣言あり | 掲載 | `cplane` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L37) |
| `sketch.cut` | Cut Sketch Entities | 宣言あり | 未掲載 | — | [sketch_clipboard.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_clipboard.rs#L25) |
| `sketch.delete` | Delete Sketch Entities | 宣言あり | 掲載 | `delete`, `removecontrolpoint` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L194) |
| `sketch.dimension` | Sketch Dimension | 宣言あり | 掲載 | `dim`, `dimangle`, `dimdiameter`, `dimradius` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L130) |
| `sketch.dimension_text` | Move Dimension Text | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L204) |
| `sketch.drag` | Drag Sketch Geometry | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L198) |
| `sketch.drag_end` | End Sketch Drag | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L201) |
| `sketch.draw.dimensioned` | Draw Dimensioned Sketch Geometry | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L54) |
| `sketch.drawing_bounds` | Drawing Size | 宣言あり | 未掲載 | — | [sketch_import.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_import.rs#L25) |
| `sketch.edit` | Edit Sketch | 宣言あり | 掲載 | `pointson` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L41) |
| `sketch.edit_text` | Edit Text | 宣言あり | 掲載 | `annotedit`, `textobject` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L124) |
| `sketch.ellipse` | Ellipse | 宣言あり | 掲載 | `ellipse` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L68) |
| `sketch.export_dxf` | Save As DXF | 宣言あり | 掲載 | `exportdxf` | [sketch_import.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_import.rs#L29) |
| `sketch.extend` | Extend | 宣言あり | 掲載 | `extend` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L70) |
| `sketch.fillet` | Fillet | 宣言あり | 掲載 | `fillet`, `filletcorners` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L23) |
| `sketch.finish` | Finish Sketch | 宣言あり | 掲載 | `pointsoff` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L42) |
| `sketch.finish_auto_constrain` | Finish with AutoConstrain | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L39) |
| `sketch.fit_curves_to_section` | Fit Curves to Mesh Section | 宣言あり | 掲載 | — | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L53) |
| `sketch.glyphs` | Constraint Glyphs | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L58) |
| `sketch.include_3d` | Include 3D Geometry | 宣言あり | 掲載 | `dupborder`, `dupedge`, `dupfaceborder` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L28) |
| `sketch.infinite` | Infinite Line | 宣言あり | 未掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L115) |
| `sketch.insert_dxf` | Insert DXF | 宣言あり | 掲載 | `import` | [sketch_import.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_import.rs#L17) |
| `sketch.insert_svg` | Insert SVG | 宣言あり | 掲載 | `import` | [sketch_import.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_import.rs#L21) |
| `sketch.inspect` | Inspect Sketch | 宣言あり | 掲載 | — | [inspect.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/inspect.rs#L34) |
| `sketch.intersect` | Intersect | 宣言あり | 掲載 | `contour`, `intersect`, `section` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L25) |
| `sketch.intersection_curve` | Intersection Curve | 宣言あり | 掲載 | `intersect` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L38) |
| `sketch.isoparametric_curve` | Isoparametric Curve | 宣言あり | 掲載 | `extractisocurve` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L48) |
| `sketch.line` | Line | 宣言あり | 掲載 | `closecrv`, `line`, `polyline` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L48) |
| `sketch.line.midpoint` | Midpoint Line | 宣言あり | 掲載 | `line` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L34) |
| `sketch.line3d` | 3D Line | 宣言あり | 掲載 | `line` | [sketch_3d.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_3d.rs#L17) |
| `sketch.loose_ends` | Loose Ends | 宣言あり | 未掲載 | — | [sketch_ends.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_ends.rs#L14) |
| `sketch.loose_ends.tie` | Tie Loose Ends | 宣言あり | 未掲載 | `closecrv`, `join` | [sketch_ends.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_ends.rs#L17) |
| `sketch.mirror` | Mirror | 宣言あり | 掲載 | `mirror` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L96) |
| `sketch.move` | Move/Copy Sketch Entities | 宣言あり | 掲載 | `copy`, `move` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L91) |
| `sketch.move3d` | Move 3D Point | 宣言あり | 掲載 | `pointson`, `setpt` | [sketch_3d.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_3d.rs#L36) |
| `sketch.move_point` | Drag Sketch Point | 宣言あり | 掲載 | `pointson`, `setpt` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L197) |
| `sketch.offset` | Offset | 宣言あり | 掲載 | `offset` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L43) |
| `sketch.offset.break` | Break Offset Association | 宣言あり | 掲載 | — | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L54) |
| `sketch.options` | Sketch Palette Options | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L207) |
| `sketch.paste` | Paste Sketch Entities | 宣言あり | 未掲載 | — | [sketch_clipboard.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_clipboard.rs#L28) |
| `sketch.pattern.circular` | Circular Pattern | 宣言あり | 掲載 | `arraypolar` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L101) |
| `sketch.pattern.rectangular` | Rectangular Pattern | 宣言あり | 掲載 | `array` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L106) |
| `sketch.point` | Point | 宣言あり | 掲載 | `point` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L129) |
| `sketch.point3d` | 3D Point | 宣言あり | 掲載 | `point` | [sketch_3d.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_3d.rs#L32) |
| `sketch.polygon.circumscribed` | Circumscribed Polygon | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L109) |
| `sketch.polygon.edge` | Edge Polygon | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L114) |
| `sketch.polygon.inscribed` | Inscribed Polygon | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L104) |
| `sketch.project` | Project | 宣言あり | 掲載 | `dupborder`, `dupedge`, `dupfaceborder`, `project`, `projecttocplane` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L24) |
| `sketch.project_to_surface` | Project To Surface | 宣言あり | 掲載 | `project`, `pull` | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L33) |
| `sketch.rectangle.center` | Center Rectangle | 宣言あり | 掲載 | `rectangle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L73) |
| `sketch.rectangle.rounded` | Rounded Rectangle | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L68) |
| `sketch.rectangle.three_point` | 3-Point Rectangle | 宣言あり | 掲載 | `rectangle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L63) |
| `sketch.rectangle.two_point` | 2-Point Rectangle | 宣言あり | 掲載 | `rectangle` | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L57) |
| `sketch.redefine` | Redefine Sketch Plane | 宣言あり | 掲載 | `cplane` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L22) |
| `sketch.scale` | Sketch Scale | 宣言あり | 掲載 | `scale` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L80) |
| `sketch.slot.arc_center` | Center Point Arc Slot | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L63) |
| `sketch.slot.arc_three_point` | Three Point Arc Slot | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L58) |
| `sketch.slot.center_point` | Center Point Slot | 宣言あり | 掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L53) |
| `sketch.slot.center_to_center` | Center to Center Slot | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L119) |
| `sketch.slot.overall` | Overall Slot | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L124) |
| `sketch.snap` | Snap Inference | 宣言あり | 掲載 | `gridsnap`, `ortho`, `osnap`, `snap` | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L62) |
| `sketch.solve` | Solve Sketch | 宣言あり | 掲載 | — | [sketch.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch.rs#L214) |
| `sketch.spline.control_point` | Control Point Spline | 宣言あり | 掲載 | `curve` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L77) |
| `sketch.spline.control_point_5` | Control Point Spline (Degree 5) | 宣言あり | 掲載 | `curve` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L82) |
| `sketch.spline.fit_point` | Fit Point Spline | 宣言あり | 掲載 | `interpcrv` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L72) |
| `sketch.spline.insert_point` | Add Spline Point | 宣言あり | 未掲載 | — | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L87) |
| `sketch.spline.reverse` | Reverse Spline Direction | 宣言あり | 未掲載 | `flip` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L91) |
| `sketch.spline3d` | 3D Spline | 宣言あり | 掲載 | `interpcrv` | [sketch_3d.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_3d.rs#L21) |
| `sketch.spun_profile` | Spun Profile | 宣言あり | 掲載 | — | [sketch_project.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_project.rs#L43) |
| `sketch.text` | Text | 宣言あり | 掲載 | `text`, `textobject` | [sketch_create.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_create.rs#L118) |
| `sketch.toggle_diameter` | Toggle Radius / Diameter | 宣言あり | 未掲載 | `dimdiameter`, `dimradius` | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L52) |
| `sketch.toggle_driven` | Toggle Driven Dimension | 宣言あり | 掲載 | — | [sketch_constraints.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_constraints.rs#L49) |
| `sketch.transform` | Transform | 宣言あり | 未掲載 | `gumball`, `rotate` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L85) |
| `sketch.trim` | Trim | 宣言あり | 掲載 | `trim` | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L57) |
| `sketch.trim.path` | Trim Along Path | 宣言あり | 未掲載 | — | [sketch_modify.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/sketch_modify.rs#L63) |

## solid（37 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `solid.align` | Align | 宣言あり | 掲載 | `osnap`, `orient`, `orient3pt` | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L43) |
| `solid.boundary_fill` | Boundary Fill | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L18) |
| `solid.bounding_solid` | Bounding Solid | 宣言あり | 掲載 | `boundingbox` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L108) |
| `solid.box` | Box | 宣言あり | 掲載 | `box` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L47) |
| `solid.chamfer` | Chamfer | 宣言あり | 掲載 | `chamferedge` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L82) |
| `solid.coil` | Coil | 宣言あり | 掲載 | `helix` | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L21) |
| `solid.combine` | Combine | 宣言あり | 掲載 | `booleandifference`, `booleanintersection`, `booleanunion`, `booleansplit` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L83) |
| `solid.cone` | Cone | 宣言あり | 未掲載 | `cone` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L71) |
| `solid.cylinder` | Cylinder | 宣言あり | 掲載 | `cylinder` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L53) |
| `solid.draft` | Draft | 宣言あり | 掲載 | `taper` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L136) |
| `solid.emboss` | Emboss | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L35) |
| `solid.extrude` | Extrude | 宣言あり | 掲載 | `taper`, `extrude` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L31) |
| `solid.fillet` | Fillet | 宣言あり | 掲載 | `fillet`, `filletedge` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L77) |
| `solid.hole` | Hole | 宣言あり | 掲載 | — | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L97) |
| `solid.loft` | Loft | 宣言あり | 掲載 | `loft` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L46) |
| `solid.melt` | Melt | 旧一覧のみ・未確認 | 掲載 | — | [旧一覧](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/docs/features.md) |
| `solid.mirror` | Mirror | 宣言あり | 掲載 | `mirror` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L96) |
| `solid.move` | Move/Copy | 宣言あり | 掲載 | `gumball`, `copy`, `move`, `orient`, `rotate`, `rotate3d` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L163) |
| `solid.offset_face` | Offset Face | 宣言あり | 掲載 | — | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L124) |
| `solid.pattern.circular` | Circular Pattern | 宣言あり | 掲載 | `arraypolar` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L88) |
| `solid.pattern.path` | Pattern on Path | 宣言あり | 掲載 | `arraypath` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L92) |
| `solid.pattern.rectangular` | Rectangular Pattern | 宣言あり | 掲載 | `array` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L84) |
| `solid.pipe` | Pipe | 宣言あり | 掲載 | `pipe` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L102) |
| `solid.press_pull` | Press Pull | 宣言あり | 掲載 | `pushpull` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L158) |
| `solid.remove` | Remove | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L47) |
| `solid.replace_face` | Replace Face | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L39) |
| `solid.revolve` | Revolve | 宣言あり | 掲載 | `revolve` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L38) |
| `solid.rib` | Rib | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L27) |
| `solid.scale` | Scale | 宣言あり | 掲載 | `scale`, `scale1d`, `scale2d`, `scalenu` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L114) |
| `solid.shell` | Shell | 宣言あり | 掲載 | `shell` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L132) |
| `solid.sphere` | Sphere | 宣言あり | 掲載 | `sphere` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L59) |
| `solid.split_body` | Split Body | 宣言あり | 掲載 | `split`, `booleansplit` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L154) |
| `solid.split_face` | Split Face | 宣言あり | 掲載 | `split` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L148) |
| `solid.sweep` | Sweep | 宣言あり | 掲載 | `twist`, `sweep1`, `sweep2` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L42) |
| `solid.thread` | Thread | 宣言あり | 掲載 | — | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L128) |
| `solid.torus` | Torus | 宣言あり | 掲載 | `torus` | [features.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features.rs#L65) |
| `solid.web` | Web | 宣言あり | 掲載 | — | [features_more.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/features_more.rs#L31) |

## surface（19 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `surface.blend` | Blend Surface | 宣言あり | 掲載 | `blendsrf` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L63) |
| `surface.boundary` | Boundary Surface | 宣言あり | planned表記 | `edgesrf`, `patch` | [surface_boundary.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_boundary.rs#L29) |
| `surface.extend` | Extend | 宣言あり | 掲載 | `extendsrf` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L31) |
| `surface.extrude` | Surface Extrude | 宣言あり | 掲載 | `extrude` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L39) |
| `surface.fair` | Fair Surface | 宣言あり | 未掲載 | — | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L79) |
| `surface.fillet` | Surface Fillet | planned | planned表記 | — | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L83) |
| `surface.loft` | Surface Loft | 宣言あり | 掲載 | `loft` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L51) |
| `surface.match` | Match Surface | 宣言あり | 未掲載 | — | [surface_match.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface_match.rs#L16) |
| `surface.offset` | Offset Surface | 宣言あり | 掲載 | `offsetsrf` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L55) |
| `surface.patch` | Patch | 宣言あり | 掲載 | `cap`, `edgesrf`, `patch`, `planarsrf` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L19) |
| `surface.reverse_normal` | Reverse Normal | 宣言あり | 掲載 | `flip` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L75) |
| `surface.revolve` | Surface Revolve | 宣言あり | 掲載 | `revolve` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L43) |
| `surface.ruled` | Ruled Surface | 宣言あり | 掲載 | — | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L59) |
| `surface.stitch` | Stitch | 宣言あり | 掲載 | `join`, `cap` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L23) |
| `surface.sweep` | Surface Sweep | 宣言あり | 掲載 | `twist`, `sweep1`, `sweep2` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L47) |
| `surface.thicken` | Thicken | 宣言あり | 掲載 | — | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L27) |
| `surface.trim` | Trim | 宣言あり | 掲載 | `trim` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L35) |
| `surface.unstitch` | Unstitch | 宣言あり | 掲載 | `explode`, `extractsrf` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L71) |
| `surface.untrim` | Untrim | 宣言あり | 掲載 | `untrim` | [surface.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/surface.rs#L67) |

## timeline（21 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `timeline.add` | Add Recorded Feature | 宣言あり | 掲載 | — | [script.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/script.rs#L17) |
| `timeline.compute_all` | Compute All | 宣言あり | 掲載 | `recordhistory` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L31) |
| `timeline.copy` | Copy Features | 宣言あり | 掲載 | — | [clipboard.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/clipboard.rs#L15) |
| `timeline.delete` | Delete | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L32) |
| `timeline.dependents` | Feature Dependents | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L62) |
| `timeline.edit` | Edit Feature | 宣言あり | 掲載 | `recordhistory` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L70) |
| `timeline.export_script` | Export Command Script | 宣言あり | 掲載 | — | [script.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/script.rs#L22) |
| `timeline.group` | New Feature Folder | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L38) |
| `timeline.inspect` | Inspect Feature | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L76) |
| `timeline.move_many` | Move Timeline Items | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L49) |
| `timeline.move_to_group` | Move Features to Folder | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L46) |
| `timeline.paste` | Paste Features | 宣言あり | 掲載 | — | [clipboard.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/clipboard.rs#L16) |
| `timeline.redefine` | Edit Feature | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L60) |
| `timeline.rename` | Rename | 宣言あり | 掲載 | `rename` | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L37) |
| `timeline.rename_group` | Rename Feature Folder | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L45) |
| `timeline.reorder` | Reorder Feature | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L55) |
| `timeline.reorder_many` | Reorder Features | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L57) |
| `timeline.roll_to` | Roll History Marker | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L53) |
| `timeline.rollback` | Move Timeline Marker | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L52) |
| `timeline.suppress` | Suppress Feature | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L65) |
| `timeline.ungroup` | Remove Feature Folders | 宣言あり | 掲載 | — | [edit.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/edit.rs#L42) |

## view（35 ID）

| ID | このリポジトリのラベル | 現行の静的登録 | 旧一覧373件 | 比較表のSerpentine主名 | 出典 |
|---|---|---|---|---|---|
| `view.ambient_occlusion` | Ambient Occlusion | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L81) |
| `view.angle_readout` | View Angles | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L85) |
| `view.apply` | Restore Named View | 宣言あり | 掲載 | `namedview` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L41) |
| `view.back` | Back View | 宣言あり | 未掲載 | `back` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L98) |
| `view.bottom` | Bottom View | 宣言あり | 未掲載 | `bottom` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L102) |
| `view.delete` | Delete Named View | 宣言あり | 掲載 | `namedview` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L39) |
| `view.fit` | Fit | 宣言あり | 未掲載 | `zoom`, `zoomextents`, `zoomselected` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L34) |
| `view.front` | Front View | 宣言あり | 未掲載 | `front` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L97) |
| `view.graphics_preset` | Graphics Preset | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L79) |
| `view.grid` | Grid | 宣言あり | 未掲載 | `grid` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L44) |
| `view.ground_shadow` | Ground shadow | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L46) |
| `view.history_layout` | History layout | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L77) |
| `view.home` | Home view | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L36) |
| `view.isometric` | Isometric View | 宣言あり | 未掲載 | `isometric` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L103) |
| `view.layout` | Workspace layout | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L75) |
| `view.left` | Left View | 宣言あり | 未掲載 | `left` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L99) |
| `view.list` | List Named Views | 宣言あり | 掲載 | `namedview` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L40) |
| `view.navigate` | Navigate View | 宣言あり | 掲載 | `camera`, `zoom` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L87) |
| `view.navigation_bar` | Navigation bar | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L48) |
| `view.normal_to` | Normal to Selection | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L92) |
| `view.perspective` | Perspective view | 宣言あり | 未掲載 | `perspective` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L83) |
| `view.rename` | Rename Named View | 宣言あり | 掲載 | `namedview` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L38) |
| `view.render_browser` | Render Browser | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L54) |
| `view.render_dock` | Move Render Dock | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L70) |
| `view.render_focus` | Render Focus View | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L50) |
| `view.render_inspector` | Render Inspector | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L58) |
| `view.render_select` | Render selection | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L61) |
| `view.render_stage` | Render Stage | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L64) |
| `view.right` | Right View | 宣言あり | 未掲載 | `right` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L100) |
| `view.save` | New Named View | 宣言あり | 掲載 | `namedview` | [browser.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/browser.rs#L35) |
| `view.shots_strip` | Shots Strip | 宣言あり | 未掲載 | — | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L67) |
| `view.spin` | Continuous orbit | 宣言あり | 未掲載 | `turntable` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L40) |
| `view.tool` | Navigation tool | 宣言あり | 未掲載 | `zoom`, `zoomwindow` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L38) |
| `view.top` | Top View | 宣言あり | 未掲載 | `top` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L101) |
| `view.visual_style` | Visual style | 宣言あり | 未掲載 | `shaded`, `surfaceedges`, `technical`, `wireframe` | [view_navigation.rs](https://github.com/07egta/melt3D/blob/a309790ac8e265b918da09474a02b069609ab3b5/crates/engine/src/cmd/view_navigation.rs#L42) |

