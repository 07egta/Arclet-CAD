# Arclet CAD native plugins — opt-in desktop MVP

Status: source prototype (native build and CI verification required). This
describes Arclet's **native worker** protocol, not SolidCraft's planned
Wasm/WIT plugin host in [plugins.md](plugins.md). Arclet follows
[docs/policy.md](policy.md): desktop-first, Rust-first, no GPL/LGPL/AGPL code.

## Host decision

| Mode | Benefits | Limitations | Status |
|---|---|---|---|
| Separate native executable with JSON stdio | Plugin panic/exit isolated from host; no unsafe ABI; checked commands | Native process still has user's OS privileges; IPC overhead | **Chosen**, manual trust required |
| In-process DLL/C++ ABI | Low overhead | Memory corruption, ABI mismatch, crashes affect Arclet itself | Rejected for MVP |
| WASI/Wasm component runtime | Enforceable capability isolation with a suitable runtime | Requires separate runtime, WIT host and compatibility work | Future option |

**IMPORTANT: This is NOT an OS sandbox.** A native executable can still
read local files, use network or spawn child processes. Environment clearing,
a five-second timeout and command allowlists restrict the host protocol,
not the native process's ambient privileges. Enable **only trusted binaries**.
Future hardening requires OS sandboxes, resource limits, code signing/pinning
and process-tree termination before a marketplace for untrusted plugins.

## Discovery, installation and UI

The Arclet desktop app discovers manifests on launch but executes none. An
absent plugin folder is normal; Arclet runs with zero plugins. The folder is:

- Windows: `%APPDATA%\Arclet\plugins`
- macOS: `~/Library/Application Support/Arclet/plugins`
- Linux: `$XDG_DATA_HOME/arclet/plugins` or `~/.local/share/arclet/plugins`
- Development override: `ARCLET_PLUGIN_DIR`

Each plugin lives in `<root>/<plugin-id>/plugin.json`. Its directory must
match the declared ID. No sibling folders or legacy SolidCraft folder are scanned.

Build the included example with
`cargo build -p arclet --bin arclet-sample-plugin`, then copy
[plugin.json](../examples/arclet-plugin-sample/plugin.json) and
`target/debug/arclet-sample-plugin` (on Windows `.exe`) into
`<root>/org.arclet.sample/`. Open **Tools → Plugins**.
Read the trust warning and the declared permissions, then enable it. Enter
`{"size_mm":10}` in the JSON field and click
`plugin.org.arclet.sample.box`. The model receives a 10 mm box. Use `{}` and
`plugin.org.arclet.sample.report` to inspect body count and volume.
Exact namespaced plugin command IDs also run from Arclet's command line using
the same JSON field. Dynamic autocomplete is not yet implemented.

**Rescan** reloads the directory and disables every plugin. Restarts also
disable every plugin. No trust decisions are persisted in documents or prefs.

## Manifest (API version 1.0)

See the example manifest for a complete working file.

- `id`: 1–80 lowercase ASCII letters/digits or `.`/`_`/`-`;
  no leading/trailing punctuation or `..`. Command IDs are always
  `plugin.<id>.<name>` and cannot shadow native engine commands.
- `version`: numeric `major.minor.patch`, no prerelease/metadata.
- `api_version`: **exact match** `"1.0"` required; incompatible manifests
  yield a diagnostic and no process is started.
- `executable`: one filename, never a path. Canonical resolution must remain
  inside the plugin directory. Windows adds missing `.exe` automatically.
- `permissions`: `model_read`, `model_edit`, each at most once.
- `commands`: 1–32 named commands, each with description,
  `input_schema` and `output_schema`. Top-level schemas must be objects.

The host reads a maximum of 64 KiB per manifest and scans at most 64 folders.
Only the following JSON Schema subset is supported: `type`, `properties`,
`required`, `items`, Boolean `additionalProperties`, `minimum`,
`maximum`. Types are object/array/string/number/integer/boolean/null.
Schema and value depth must be at most eight. Keywords such as `$ref`,
`oneOf`, `pattern` and `format` cause rejection, not silent ignoring.

## Request/response protocol

For each invocation the host starts the executable directly (never a shell)
with a cleared environment (Windows preserves `SystemRoot` for DLL
resolution), piped stdin and stdout, ignored stderr, and working directory
set to the plugin folder. The worker reads ONE JSON request and writes ONE
JSON reply, then exits.

Request example:

```json
{
  "protocol":"arclet.plugins/1",
  "api_version":"1.0",
  "plugin_id":"org.arclet.sample",
  "command":"box",
  "params":{"size_mm":10},
  "model":null
}
```

When `model_read` is declared, `model` contains the current revision,
body count, aggregate measurements, and up to 32 body summaries; it is
otherwise `null`. It includes no document file paths, full B-rep, raw
memory pointers or unrestricted engine state.

Response example:

```json
{
  "protocol":"arclet.plugins/1",
  "ok":true,
  "output":{"size_mm":10},
  "actions":[
    {"command":"solid.box",
     "params":{"length":10,"width":10,"height":10,"body_name":"Plugin box"}}
  ]
}
```

Failure example:

```json
{"protocol":"arclet.plugins/1","ok":false,"error":"A useful explanation"}
```

The host checks output schema, declared permission and every action **before**
calling the shared CAD engine. Successful edits go through
`Session::execute`, preserving normal parameter validation and undo.
There is at most **one model-edit action per call**; this avoids partial
multi-edit transactions. The only permitted engine command IDs are:

```text
sketch.arc.three_point   sketch.circle.center
sketch.line              sketch.spline.control_point
solid.box                solid.cone
solid.cylinder           solid.sphere
solid.torus
```

Scripts, file reads/writes, configuration changes, MCP access and arbitrary
engine commands are never exposed as *host permissions*. They are NOT
prevented from being invoked independently by native plugin code using
ordinary OS privileges. There is no full Rhino plugin ABI, parametric
timeline feature evaluator, multi-step transaction or graphical schema form.

## Failure isolation and limits

- Disabled, missing, invalid or API-incompatible plugins cannot run.
- Output schema failure, malformed/oversized data, unknown commands,
  permission denial, process nonzero exit and plugin-reported errors are
  returned as errors. No proposed edit is applied in these cases.
- Request and stdout limits: **64 KiB** each; process timeout: **5 seconds**.
  Direct child is killed and reaped on timeout, not its descendants.
- The host currently waits synchronously for the worker; malicious or stuck
  plugins can pause its UI for up to the timeout. Non-blocking execution
  is needed in a follow-up.
- Process separation does not limit a worker's own filesystem/network
  operations, subprocesses, memory usage or CPU usage. Plugins are not
  considered safe for untrusted code without a real OS sandbox.

## Verification

Run on native Rust 1.95+:

```sh
cargo test -p arclet --test plugins
cargo test -p arclet --bin arclet
cargo xtask ci
```

The integration test launches the **real compiled sample binary**, verifies
manifest discovery and metadata, explicit enable/disable, schema rejection,
actual model creation and precise volume, read-only report, worker failure,
invalid manifest/API version, undeclared edit permissions, unknown command,
missing plugin directory, and functionality after disabling.
Do not claim tests passed until CI or local execution confirms them.

## Versioning and licenses

The manifest's `version` (plugin release) and `api_version` (host contract)
are separate. Only host API `1.0` / protocol `arclet.plugins/1` is
supported. A future 2.0 must be explicitly negotiated or rejected; never
reinterpret old messages silently. This interface is independent of the
SolidCraft MCP JSON-RPC and WIT plans, though it reuses the central engine's
typed command and undo APIs.

The Arclet host code and included example are original and inherit the
workspace's **MIT OR Apache-2.0** terms. Third-party plugin executables
may have separate licenses chosen by their authors; the host API grants no
rights in plugin dependencies. Plugins must be reviewed before distribution
with Arclet. The application maintains its **no GPL/LGPL/AGPL code**
policy, and other CAD applications' implementation code is not copied.
