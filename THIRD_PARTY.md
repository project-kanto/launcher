# Third-party software

## Impactor

The planned desktop iOS signing and device integration will pin reusable crates from
[claration/Impactor](https://github.com/claration/Impactor) rather than copy its application UI.

Audited upstream revision: `79c72e9762ffc7886201e34ba0068f6fbeae5403`

- Impactor workspace and utility crates: MIT
- `plume_core`: MPL-2.0
- bundled `omnisette`: MPL-2.0
- bundled `decompress`: Apache-2.0

The signing integration is not enabled yet. Before shipping it, retain all upstream notices, audit
the complete resolved Cargo dependency licence set, and replace Impactor's plain-JSON account store
with the operating system credential store.

