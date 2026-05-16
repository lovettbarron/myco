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
# Resolve the Developer ID Application signing certificate fingerprint from the keychain.
# The SHA-256 fingerprint is used to select the correct certificate for signing.
# To find your fingerprint: security find-certificate -a -c "Developer ID Application" -Z ~/Library/Keychains/login.keychain-db
CERT_FINGERPRINT="${CODESIGN_FINGERPRINT:-}"
if [ -z "$CERT_FINGERPRINT" ]; then
    # Auto-detect: find the Developer ID Application certificate SHA-256 fingerprint
    CERT_FINGERPRINT=$(security find-certificate -a -c "Developer ID Application" -Z ~/Library/Keychains/login.keychain-db 2>/dev/null \
        | grep -B1 "Developer ID Application" | grep "SHA-256" | tail -1 | awk '{print $NF}')
fi

if [ -z "$CERT_FINGERPRINT" ]; then
    echo "ERROR: No Developer ID Application certificate found in keychain."
    echo "Install your certificate or set CODESIGN_FINGERPRINT env var."
    exit 1
fi

echo "Using certificate fingerprint: $CERT_FINGERPRINT"

rcodesign sign \
    --for-notarization \
    --entitlements-xml-file build/entitlements.plist \
    --keychain-fingerprint "$CERT_FINGERPRINT" \
    --code-signature-flags runtime \
    "$APP_PATH"

echo "=== Verifying signature ==="
codesign --verify --deep --strict "$APP_PATH"
codesign -dv "$APP_PATH"

if [ -n "$DMG_PATH" ]; then
    echo "=== Signing DMG ==="
    rcodesign sign \
        --for-notarization \
        --keychain-fingerprint "$CERT_FINGERPRINT" \
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
