#!/bin/bash
set -euo pipefail

echo "=== Building release binary ==="
cargo build --release

echo "=== Packaging .app and .dmg ==="
cargo packager --release --formats app,dmg

# Find the generated .app bundle
APP_PATH=$(find dist -name "*.app" -maxdepth 2 | head -1)
DMG_PATH=$(find dist -name "*.dmg" -maxdepth 2 | head -1)

if [ -z "$APP_PATH" ]; then
    echo "ERROR: No .app found in dist/"
    exit 1
fi

echo "=== Signing .app with hardened runtime ==="
# Try keychain-based signing first (preferred on local macOS)
# If this fails, user needs to export certificate to PEM and use --pem-source
rcodesign sign \
    --for-notarization \
    --entitlements-xml-path build/entitlements.plist \
    --keychain-domain user \
    --code-signature-flags runtime \
    "$APP_PATH"

echo "=== Verifying signature ==="
codesign --verify --deep --strict "$APP_PATH"
codesign -dv "$APP_PATH"

if [ -n "$DMG_PATH" ]; then
    echo "=== Signing DMG ==="
    rcodesign sign \
        --for-notarization \
        --keychain-domain user \
        "$DMG_PATH"

    echo "=== Notarizing DMG ==="
    # Requires App Store Connect API key at ~/.appstoreconnect/key.json
    # Create with: rcodesign encode-app-store-connect-api-key
    if [ -f ~/.appstoreconnect/key.json ]; then
        rcodesign notary-submit \
            --api-key-file ~/.appstoreconnect/key.json \
            --staple \
            "$DMG_PATH"
        echo "=== Notarization complete ==="
    else
        echo "WARNING: ~/.appstoreconnect/key.json not found. Skipping notarization."
        echo "Create it with: rcodesign encode-app-store-connect-api-key -o ~/.appstoreconnect/key.json <issuer-id> <key-id> <path-to-.p8>"
    fi
fi

echo "=== Done ==="
echo "Output: $APP_PATH"
[ -n "$DMG_PATH" ] && echo "DMG: $DMG_PATH"
