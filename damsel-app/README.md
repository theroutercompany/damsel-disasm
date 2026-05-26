# Damsel Workbench

Damsel Workbench is the native macOS shell for the Damsel Mach-O analysis UI.
It is intentionally a sibling app, not a decoder rewrite: the app launches the
existing `damsel-cli ui --no-open --port 0` server and talks to the same upload,
symbol, and disassembly API already covered by the Rust UI tests.

## Run

From the repository root:

```sh
./script/build_and_run.sh
```

To build the app bundle and verify it launches with the `semantic-switch`
fixture:

```sh
./script/build_and_run.sh --verify-fixture
```

The script builds `damsel-cli`, builds the SwiftPM app, stages
`dist/native/DamselWorkbench.app`, and copies the CLI into
`Contents/Resources/damsel-cli` so the app does not depend on a shell `PATH`.

## Shape

- `DamselWorkbench`: the SwiftUI app entrypoint and singleton workbench window.
- `DamselAppCore/Services`: the local server lifecycle and typed HTTP client.
- `DamselAppCore/Stores`: the main workbench state machine and request
  validation.
- `DamselAppCore/Views`: the three-pane native shell: target controls,
  instruction listing, and inspector.
- `DamselAppCore/Models`: typed hot-path JSON models plus flexible `JSONValue`
  payloads for evolving analysis details.

## Performance Notes

The instruction surface is optimized for long disassembly listings:

- The listing uses `ScrollView` plus `LazyVStack`, so rows are created lazily
  instead of materializing every decoded instruction view.
- Rows have stable fixed height and render only address, opcode, and one
  truncated instruction line in the hot scroll path.
- Expensive details such as structured operands, annotations, recovered values,
  and graph summary metrics live in the inspector for the selected instruction.
- The default decode limit is `1000` rows, with explicit range/limit controls so
  heavy windows are user-directed.

## Validation

Current focused gates:

```sh
swift test --package-path damsel-app
cargo test -p damsel-cli --test ui_api
./script/build_and_run.sh --verify-fixture
```
