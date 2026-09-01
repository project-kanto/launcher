# Third-party software

## Impactor

The desktop iOS signing and device integration pins reusable crates from
[claration/Impactor](https://github.com/claration/Impactor) rather than copy its application UI.

Audited upstream revision: `79c72e9762ffc7886201e34ba0068f6fbeae5403`

- Impactor workspace and utility crates: MIT
- `plume_core`: MPL-2.0
- bundled `omnisette`: MPL-2.0
- bundled `decompress`: Apache-2.0

The integration does not use Impactor's plain-JSON account store. Apple sessions and private signing
keys are saved through the platform credential store and certificate files exist only in a temporary
directory while signing.

Direct device access also pins [jkcoxson/idevice](https://github.com/jkcoxson/idevice) revision
`5171e34e3a236842af1160a84d955d037ddca3af` (MIT).

Before publishing release binaries, include the full upstream licence texts with each package and
audit the complete resolved Cargo and npm dependency licence set.
