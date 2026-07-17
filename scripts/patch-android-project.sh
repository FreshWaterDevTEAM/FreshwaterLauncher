#!/usr/bin/env bash
# Copy FWL Android overlay (GameActivity + JNI bridge) into tauri-generated project.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GEN="${ROOT}/src-tauri/gen/android"
OVERLAY="${ROOT}/android/app-overlay"

if [[ ! -d "$GEN" ]]; then
  echo "ERROR: $GEN not found. Run: npx tauri android init" >&2
  exit 1
fi

APP_SRC="$(find "$GEN" -type d -path '*/app/src/main/java' | head -n1 || true)"
MANIFEST="$(find "$GEN" -type f -name AndroidManifest.xml -path '*/app/src/main/*' | head -n1 || true)"

if [[ -z "$APP_SRC" || -z "$MANIFEST" ]]; then
  echo "ERROR: could not locate app/src/main in generated Android project" >&2
  find "$GEN" -maxdepth 4 -type d | head -n 40 >&2
  exit 1
fi

PKG_DIR="${APP_SRC}/com/freshwater/fwl"
mkdir -p "$PKG_DIR"
cp -f "${OVERLAY}/src/main/java/com/freshwater/fwl/"*.kt "$PKG_DIR/"
echo "Copied Kotlin sources → $PKG_DIR"

# Register FwlGameActivity if missing
if ! grep -q 'FwlGameActivity' "$MANIFEST"; then
  python3 - <<'PY' "$MANIFEST"
import sys
from pathlib import Path
path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
activity = '''
        <activity
            android:name=".FwlGameActivity"
            android:exported="false"
            android:configChanges="orientation|screenSize|keyboard|keyboardHidden|navigation"
            android:launchMode="singleTask"
            android:theme="@android:style/Theme.Black.NoTitleBar.Fullscreen" />
'''
if "FwlGameActivity" in text:
    print("manifest already has FwlGameActivity")
    raise SystemExit(0)
# insert before closing </application>
needle = "</application>"
if needle not in text:
    raise SystemExit(f"no </application> in {path}")
text = text.replace(needle, activity + "\n    " + needle, 1)
path.write_text(text, encoding="utf-8")
print(f"Patched AndroidManifest: {path}")
PY
else
  echo "AndroidManifest already references FwlGameActivity"
fi

echo "Android overlay applied."
