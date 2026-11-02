# Arclet CAD: design configurations and parameter sweeps

One design can hold several named configuration rows. Each row changes parameter
expressions, feature suppression, and body material/appearance. Arclet reuses the
existing expression evaluator and feature-rebuild API: it does not create a
parallel parametric-history system.

## Designer Chair example

The following examples are commands for the existing engine/MCP/CLI command
interface. Run each command against one open design, using its `id` as the
command name and `params` as its JSON arguments. Model the chair first; the
configuration column targets below must exist in the document.

Create named user parameters (the chair geometry can reference them in its
sketch dimensions and feature inputs):

```json
{"id":"parameters.add","params":{"name":"overall_width","expression":"45 cm","unit":"mm"}}
{"id":"parameters.add","params":{"name":"overall_height","expression":"90 cm","unit":"mm"}}
{"id":"parameters.add","params":{"name":"seat_height","expression":"overall_height / 2","unit":"mm"}}
{"id":"parameters.add","params":{"name":"leg_thickness","expression":"1 in","unit":"mm"}}
{"id":"parameters.add","params":{"name":"back_angle","expression":"15 deg","unit":"deg"}}
```

The first two lengths are stored in mm, regardless of the units used in the
expressions. `seat_height` follows `overall_height`. Angle expressions use
degrees or radians, with internal values in radians.

Add configuration columns for these parameters and the **actual names** of
your chair's feature and body:

```json
{"id":"config.configure","params":{"params":["overall_width","overall_height","back_angle","leg_thickness"],"features":["Legs"],"materials":["Chair Body"]}}
```

`config.configure` captures the current design as the `Default` row. When
needed, add a `config.column` for other feature suppressions or appearances.
The precise `"Legs"` and `"Chair Body"` names are placeholders here, not
guaranteed names produced by the example.

Make a second configuration. A `values` field can use parameter names
directly, while suppression and material columns use their displayed keys:

```json
{"id":"config.row","params":{"name":"Wide Chair","from":"Default","values":{"overall_width":"20 in","overall_height":"1 m","back_angle":"0.25 rad","suppress:Legs":false,"material:Chair Body":"Steel"}}}
```

Preview and activate without confusing the two operations:

```json
{"id":"config.preview","params":{"row":"Wide Chair"}}
{"id":"config.activate","params":{"row":"Wide Chair"}}
```

`config.preview` leaves the source document and its revision untouched.
`config.activate` changes its parameter expressions and feature suppression
as one undoable command, after a scratch rebuild.

## Validate the full configuration table

```json
{"id":"config.validate","params":{}}
{"id":"config.validate","params":{"details":true}}
```

The response reports `valid`, `checked`, `failed`, and one entry per row.
Each entry has `name`, `valid`, and, if invalid, `error`. With `details:true`,
valid entries also include their evaluated parameter table, rule trace, and
style status. Every row is evaluated from the same original document rather
than from the preceding row. A bad expression, dimension mismatch, cycle,
missing target, rule failure or newly failed rebuild is reported without
changing the document.

Structural errors, such as duplicate rows/columns, an incomplete row, or a
dangling active row, reject validation at the table level. Invalid
**expressions** in individual rows do not prevent examining other rows.

## Generate a Cartesian sweep

```json
{"id":"config.sweep","params":{"prefix":"Chair","from":"Default","axes":{"overall_width":["45 cm","20 in"],"overall_height":["90 cm","1 m"],"back_angle":["15 deg","0.25 rad"]},"preview":true}}
```

The example yields 8 named candidate rows (`Chair-001` through `Chair-008`).
`preview:true` evaluates every candidate, returns its name and cells, and
does not save any. Omit `preview` (or set it to `false`) to add all candidates
in one undoable edit. Every candidate is validated before *any* is added.

Sweep axes may also target a fully qualified configured column key:

```json
{"id":"config.sweep","params":{"prefix":"Materials","from":"Default","axes":{"material:Chair Body":["Steel","Aluminum"],"suppress:Legs":[false,true]}}}
```

A sweep inherits non-axis cells (including material and feature suppression)
from the **same source row**; it does not inherit intermediate states from
previous variants. To switch to a saved candidate, use `config.activate`.

## Correctness and limits

- Maximum table: 500 columns and 2,000 rows. Maximum one sweep: 12 axes and
  128 combinations. No axis may be empty or contain duplicate cell text.
- A sweep needs existing configuration columns; it does not silently create
  missing user parameters or features. Candidate row names must be unique.
- Cell expressions are limited to 4,096 bytes; row names to 128 bytes.
  Sweep names use the supplied prefix plus a stable three-digit suffix.
- Unit conversion accepts mm, cm, m, inch/in, degrees/deg, radians/rad, and
  other units already in the expression engine. Numbers without an explicit
  unit follow existing parameter/feature rules and the design's unit context.
- Cycles, invalid expressions, and inconsistent dimensions fail the existing
  parameter evaluator. Rebuild and appearance checks use existing geometry
  and style resolution. No feature-history internals are changed.
- Validation and sweeping rebuild each candidate to check correctness.
  A very large or complex design can therefore take significant time.
  Identical expression evaluations use the existing definition-sensitive
  bounded cache; results are never shared solely by row name.
- Validation compares rebuild failures with the source design, consistent
  with `config.activate`. Pre-existing identical geometry errors may remain
  in an otherwise accepted row, and geometric warnings/physical suitability
  still need separate inspection.
- A sweep creates **configuration rows**, not multiple exported STEP/STL/3MF
  files. Activate each variant and export through the normal file commands.
  Batch export and asynchronous cancellation are not part of this change.
- English/Japanese labels are provided for the two new commands; other
  locales continue to use normal fallback behavior.

## Regression test commands

```sh
cargo test --locked -p solidcraft-doc config::tests
cargo test --locked -p solidcraft-engine cmd::config::tests
cargo test --locked -p solidcraft-doc expr::tests
cargo test --locked -p solidcraft-engine
```

The new tests cover a chair-like unit-aware dependency chain, cycles and
invalid formulas, deterministic switching, failed-switch rollback, bulk
validation with multiple errors, bounded Cartesian sweeps, variant isolation,
material/suppression switches, and preservation of the source design after
preview or rejected candidates.
