# Arclet CAD — Precision Industrial Workspace (Mission 21)

This is an **Arclet desktop UI** change. It does not implement multiple cameras,
rewrite the shared SolidCraft egui kit, or import any third-party CAD artwork.

## Design system

The default `Classic` theme keeps a neutral industrial canvas, thin dividers,
dark neutral text, a muted blue interaction accent, and a restrained yellow
selected-body cue. Existing user-selected dark and light themes remain selectable
and are not silently overwritten.

The small `classic_look::chrome` module contains **logical-point** Arclet chrome
metrics. It is not a second global token system: typography, colour, corners,
and control heights continue to use `Tokens::get()`.

| Element | Measurement / rule |
|---|---|
| Standard tool hit target | 32 logical points |
| Main tool hit target | 36 logical points |
| Main dock | 46 logical points |
| Icon drawing | 20 logical points, using the existing original painter glyphs |
| Controls gap | 2 points |
| Section divider gap | 4 points |
| Flyout | 248-point preferred width, clamped to viewport |
| Flyout positioning | Prefer right, flip left if necessary, clamp Y within viewport |
| Status layout | One non-wrapping line, compact below 1380 logical points |
| Inspector | One non-wrapping tab strip, independent tab scroll position, bounded body scroll |

Selected states use `accent_soft` and `accent`; hovered controls use the
theme's `hover`; pressed controls show the selection surface. Keyboard focus
uses a visible accent outline. Disabled file actions use muted ink and explain
why an action is unavailable instead of appearing enabled.

### Contrast of the edited Classic light palette

These values were calculated from the theme hex colours using the WCAG relative
luminance formula in a local Python calculation, **not** from a screenshot.
The Rust regression test independently checks that each ratio is at least 4.5:1.

| Pair | Calculated contrast |
|---|---:|
| Primary text on panel | 13.53:1 |
| Secondary text on panel | 6.31:1 |
| Primary text on application bar | 12.49:1 |
| Selected text on selected background | 11.43:1 |
| White text on blue accent | 6.52:1 |
| Error text on panel | 6.06:1 |
| Warning text on panel | 5.60:1 |

Contrast of rendered anti-aliased glyph edges, alternate themes, hover fills,
and screen calibration still require visual review.

## Behavior changes

- The upper tool row scrolls horizontally rather than wrapping and shifting the
  viewport when the available width shrinks.
- Native flyouts cannot be positioned beyond the nominal viewport; the menu is
  vertically scrollable if its content exceeds the available height.
- Right-click, long-press, Shift+F10, and Alt+Down are flyout gestures when the
  corresponding tool has focus; normal primary click still runs its command.
- The properties panel uses one-line scrollable tabs and preserves a visible
  fold control at the bottom. Long property values offer their full text on hover.
- The status bar no longer grows into multiple rows when selection or coordinates
  change. In compact mode it shows coordinate/unit/selection and supported
  Grid Snap; unimplemented object snaps and modes remain disabled in full mode
  and are not misleadingly presented as active in compact mode.

## Verification matrix

The pure `classic_look` test covers every pair in this 4 × 4 matrix, including
top-left and bottom-right flyout positions. This only checks **geometry in egui
logical coordinates**; it is not a substitute for a native click test or actual
HiDPI rendering.

| Physical display | 100% | 125% | 150% | 200% |
|---|---|---|---|---|
| 1280 × 720 | test | test | test | test |
| 1366 × 768 | test | test | test | test |
| 1920 × 1080 | test | test | test | test |
| 2560 × 1600 | test | test | test | test |

**No Rust test from this PR has yet been observed passing.** The test cases
are written and committed; the Actions result must be inspected separately.
The same caveat applies to `cargo fmt`, `cargo clippy`,
`cargo xtask lists`, and `cargo xtask ui-containers`.

### Actual-render capture automation

`.github/workflows/arclet-visual-review.yml` builds the real `arclet` binary
and, *only if the build succeeds*, attempts these five native Xvfb captures
through the application's existing `--screenshot` control path:

| State | Invocation | Availability |
|---|---|---|
| First frame / no sample | `--screenshot` | Action configured; result pending |
| Model open | `--sample --screenshot` | Action configured; result pending |
| Curve sketch / rebuild | `--curve-sample --screenshot` | Action configured; result pending |
| Nodes workspace | `--nodes --screenshot` | Action configured; result pending |
| Zebra / analysis | `--sample --zebra --screenshot` | Action configured; result pending |

It also archives `docs/screenshots/arclet-ui-2026-11-02.png` as a
**repository reference**, not an identically staged before-shot.
It verifies the PNG header, dimensions, and SHA-256 and saves logs as artifacts.
It does **not** claim visual correctness just because a PNG exists.

The following requested screenshots are still **unverified** without an
interactive native session: selected solid, toolbar flyout, command input,
error state, manually expanded panel, and side-by-side dark/light themes.
Likewise, real X11/Windows/macOS render DPI matrices, hit-area overlap,
tooltip placement, tab navigation, and final manual visual inspection remain
open until actual screenshots and interactions can be reviewed.

## Change boundaries and review

- Mission 22 owns four-viewport cameras and rendering internals.
- PR #2 owns the responsive main header and panel width selection. Reconcile
  its panel changes before merging this PR; do not reimplement its menu policy.
- PR #1 owns shared UI-kit exports and existing UI guard fixes. Resolve any
  conflicts in the small shared panel/status sections at merge time.
- `docs/policy.md` explicitly scopes Arclet to desktop, whereas the inherited
  `AGENTS.md` still mentions WebAssembly as a global SolidCraft rule.
  This PR introduces no new WebAssembly-specific behavior.
- Some skills named by `AGENTS.md` are not present in the checked-in
  `skills/` directory, so their unavailable contents cannot be claimed read.
- Source edits were made with the GitHub connector. In the current execution
  environment `cargo` and `rustc` are absent and direct GitHub cloning fails
  DNS resolution. The PR stays **draft**, and no build, screenshot, or test run
  is reported successful until GitHub Actions confirms it.

Changes a default: no change to default selection behavior or persisted settings.
The already-selected Classic theme's colours and chrome presentation are refined.
