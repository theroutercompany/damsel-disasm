This directory contains checked-in export-trie payload corpus cases used for
advanced export parsing coverage.

Each `*.toml` case encodes:
- `flags`: export-trie node flags.
- `address`: export-trie node address field.
- `libs`: dylib-ordinal lookup table for reexports.
- `payload_hex`: node payload bytes as space-delimited hex.
- `expect`: expected parse/result class for downstream tests.

Current cases:
- `reexport-same-name.toml`
- `reexport-renamed-symbol.toml`
- `stub-and-resolver.toml`
- `malformed-reexport-ordinal.toml`
- `malformed-stub-resolver-offsets.toml`
- `unknown-flag-bits-regular.toml`
