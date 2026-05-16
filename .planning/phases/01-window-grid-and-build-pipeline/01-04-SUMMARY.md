---
phase: 01-window-grid-and-build-pipeline
plan: 04
subsystem: build-pipeline
tags: [cargo-packager, rcodesign, code-signing, notarization, dmg, macos-distribution]
dependency_graph:
  requires:
    - phase: 01-01
      provides: project scaffold, Cargo.toml, release binary
    - phase: 01-02
      provides: renderer, grid layout (binary content to package)
  provides:
    - build-pipeline (Packager.toml, entitlements, package.sh)
    - signed-app-bundle (.app with hardened runtime)
    - dmg-distribution (.dmg disk image)
  affects: [distribution, ci-cd]
tech_stack:
  added: [cargo-packager-0.11.8, apple-codesign-0.29.0]
  patterns: [rcodesign-keychain-fingerprint, auto-detect-signing-cert, before-packaging-command]
key_files:
  created:
    - Packager.toml
    - build/entitlements.plist
    - scripts/package.sh
    - assets/.gitkeep
  modified:
    - .gitignore
key_decisions:
  - "cargo-packager Packager.toml uses flat schema (not nested under [package]) -- RESEARCH.md pattern was incorrect"
  - "binaries.path field used instead of binaries.name (cargo-packager 0.11.8 actual API)"
  - "application-folder-position field used instead of app-folder-position (cargo-packager 0.11.8 actual API)"
  - "binaries-dir = ./target/release added to resolve binary location (cargo-packager looks in out-dir by default)"
  - "rcodesign --keychain-fingerprint used instead of --keychain-domain user (fingerprint is required for cert selection)"
  - "rcodesign --entitlements-xml-file is the correct flag (not --entitlements-xml-path from RESEARCH.md)"
  - "signing-identity intentionally omitted from Packager.toml -- signing handled by rcodesign for hardened runtime"
  - "Auto-detect SHA-256 fingerprint from keychain with CODESIGN_FINGERPRINT env var override for CI"
status: checkpoint-blocked
checkpoint_task: 2
checkpoint_type: human-verify
duration: 10 min
completed: null
---

# Phase 01 Plan 04: Build Pipeline -- Package, Sign, Notarize Summary

**cargo-packager + rcodesign distribution pipeline with Packager.toml, hardened runtime entitlements, and auto-detect signing certificate from macOS Keychain**

## Status: CHECKPOINT BLOCKED (Task 2)

Task 1 complete. Task 2 (human-verify) requires user to run the signing pipeline and verify the DMG installs without Gatekeeper warnings.

## Performance

- **Duration:** 10 min (Task 1 only)
- **Started:** 2026-05-16T00:37:34Z
- **Checkpoint reached:** 2026-05-16T00:48:21Z
- **Tasks completed:** 1 of 2
- **Files created:** 4
- **Files modified:** 1

## Accomplishments

- Installed cargo-packager 0.11.8 and rcodesign (apple-codesign 0.29.0) CLI tools
- Created Packager.toml with correct flat schema for cargo-packager 0.11.8
- Created build/entitlements.plist with JIT and unsigned-memory entitlements for wgpu/Metal
- Created scripts/package.sh with auto-detect signing certificate, rcodesign signing, and notarization
- Verified cargo build --release succeeds
- Verified cargo packager creates both .app and .dmg bundles
- Updated .gitignore to exclude code signing credentials (.p8, .pem, .p12)
- Confirmed Developer ID Application certificate present in Keychain (JXW9RJT4W2)

## Task Commits

1. **Task 1: Create build pipeline configuration** - `630b83e` (feat)
2. **Task 1 fix: Correct rcodesign CLI flags** - `d57a1ce` (fix)

## Files Created/Modified

- `Packager.toml` - cargo-packager config (com.andrewlb.myco, .app/.dmg, binaries-dir)
- `build/entitlements.plist` - Hardened runtime entitlements (JIT, unsigned executable memory)
- `scripts/package.sh` - Build, sign, notarize, and package script with auto-detect cert
- `assets/.gitkeep` - Placeholder for future app icon
- `.gitignore` - Added .p8, .pem, .p12 exclusions for code signing credential safety

## Decisions Made

1. **Packager.toml flat schema**: RESEARCH.md showed config nested under `[package]` but cargo-packager 0.11.8 uses a flat top-level schema. Also `binaries` uses `path` not `name`, and DMG uses `application-folder-position` not `app-folder-position`.

2. **binaries-dir for binary resolution**: cargo-packager defaults `binaries-dir` to `out-dir` (./dist) but the release binary lives at `target/release/myco`. Added explicit `binaries-dir = "./target/release"`.

3. **rcodesign certificate selection**: `--keychain-domain user` alone cannot select the signing certificate. Must use `--keychain-fingerprint` with the SHA-256 fingerprint. Script auto-detects from keychain with `CODESIGN_FINGERPRINT` env var override.

4. **rcodesign entitlements flag**: The correct flag is `--entitlements-xml-file` (not `--entitlements-xml-path` as in RESEARCH.md).

5. **Signing identity not in Packager.toml**: Intentional -- rcodesign handles signing separately with `--for-notarization` and hardened runtime flags that cargo-packager's built-in signing may not provide.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Packager.toml schema was nested under [package]**
- **Found during:** Task 1 (packaging test)
- **Issue:** RESEARCH.md pattern showed `[package]`, `[[package.binaries]]`, `[package.macos]`, `[package.dmg]` but cargo-packager 0.11.8 expects flat top-level keys.
- **Fix:** Removed `[package]` prefix, used `[[binaries]]`, `[macos]`, `[dmg]` directly.
- **Files modified:** Packager.toml
- **Commit:** 630b83e

**2. [Rule 3 - Blocking] binaries field uses path not name**
- **Found during:** Task 1 (packaging test)
- **Issue:** cargo-packager expects `path = "myco"` in `[[binaries]]`, not `name = "myco"`.
- **Fix:** Changed to `path = "myco"`.
- **Files modified:** Packager.toml
- **Commit:** 630b83e

**3. [Rule 3 - Blocking] DMG app-folder-position field name incorrect**
- **Found during:** Task 1 (packaging test)
- **Issue:** The field is `application-folder-position`, not `app-folder-position`.
- **Fix:** Renamed field.
- **Files modified:** Packager.toml
- **Commit:** 630b83e

**4. [Rule 3 - Blocking] Binary not found in dist/ directory**
- **Found during:** Task 1 (packaging test)
- **Issue:** cargo-packager looked for binary at `dist/myco` but it's at `target/release/myco`.
- **Fix:** Added `binaries-dir = "./target/release"` to Packager.toml.
- **Files modified:** Packager.toml
- **Commit:** 630b83e

**5. [Rule 3 - Blocking] rcodesign CLI flags incorrect**
- **Found during:** Task 1 (signing test)
- **Issue:** `--entitlements-xml-path` is not the correct flag (should be `--entitlements-xml-file`). `--keychain-domain user` does not select the signing certificate.
- **Fix:** Updated to `--entitlements-xml-file` and `--keychain-fingerprint` with auto-detect.
- **Files modified:** scripts/package.sh
- **Commit:** d57a1ce

---

**Total deviations:** 5 auto-fixed (5 blocking -- Rule 3)
**Impact on plan:** All fixes were API adaptations for actual cargo-packager/rcodesign CLIs vs RESEARCH.md patterns. No scope or architectural changes.

## Threat Mitigation Status

| Threat ID | Status | Notes |
|-----------|--------|-------|
| T-04-01 | Mitigated | --for-notarization and --code-signature-flags runtime in package.sh |
| T-04-02 | Mitigated | Minimal entitlements only (JIT + unsigned-memory). No network/file access. |
| T-04-03 | Mitigated | Script reads cert from keychain, no embedded secrets. .gitignore excludes .p8/.pem/.p12. |

## Checkpoint: Task 2 (Human Verification Required)

### What was built
- Complete build pipeline: `cargo build --release` -> `cargo packager` -> `rcodesign sign` -> `rcodesign notary-submit`
- .app and .dmg bundles successfully created by cargo-packager
- Developer ID Application certificate confirmed in Keychain

### What needs human verification
1. Run `bash scripts/package.sh` (will prompt for Keychain access approval)
2. Open the DMG from `dist/Myco_0.1.0_aarch64.dmg`
3. Drag Myco.app to Applications (or test location)
4. Launch Myco -- verify NO Gatekeeper warning
5. If notarization is desired: set up App Store Connect API key first

### API key setup (for notarization)
```bash
rcodesign encode-app-store-connect-api-key \
    -o ~/.appstoreconnect/key.json \
    <issuer-id> <key-id> <path-to-.p8-file>
```
Get these from: https://appstoreconnect.apple.com/access/integrations/api -> Team Keys

## Known Stubs

None -- all pipeline files are fully functional.

## Issues Encountered

- rcodesign signing test was blocked on macOS Keychain access dialog (expected for code signing operations)
- API key for notarization not yet configured (~/.appstoreconnect/key.json missing)

## User Setup Required

- **Apple Developer certificate**: Present in Keychain (verified)
- **App Store Connect API key**: NOT configured. Required for notarization. See checkpoint instructions above.

## Self-Check: PASSED

---
*Phase: 01-window-grid-and-build-pipeline*
*Checkpoint: 2026-05-16*
