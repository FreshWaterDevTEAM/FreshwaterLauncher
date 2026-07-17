#!/usr/bin/env bash
# Copy FWL Android overlay + wire Amethyst/Pojav runtime into tauri-generated project.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GEN="${ROOT}/src-tauri/gen/android"
OVERLAY="${ROOT}/android/app-overlay"
AMETHYST="${ROOT}/third_party/amethyst-android"
PATCH_PY="${ROOT}/android/amethyst-patches/convert-app-to-library.py"
STAGED="${GEN}/fwl_amethyst_tree"

if [[ ! -d "$GEN" ]]; then
  echo "ERROR: $GEN not found. Run: npx tauri android init" >&2
  exit 1
fi

if [[ ! -d "$AMETHYST/app_pojavlauncher" ]]; then
  echo "ERROR: Amethyst submodule missing at $AMETHYST" >&2
  echo "Run: git submodule update --init --recursive" >&2
  exit 1
fi

if [[ -f "$AMETHYST/.gitmodules" ]]; then
  git -C "$AMETHYST" submodule update --init --recursive || true
fi

APP_SRC="$(find "$GEN" -type d -path '*/app/src/main/java' | head -n1 || true)"
MANIFEST="$(find "$GEN" -type f -name AndroidManifest.xml -path '*/app/src/main/*' | head -n1 || true)"
APP_BUILD="$(find "$GEN" -type f \( -name 'build.gradle.kts' -o -name 'build.gradle' \) -path '*/app/*' | head -n1 || true)"
SETTINGS="$(find "$GEN" -maxdepth 2 -type f \( -name 'settings.gradle.kts' -o -name 'settings.gradle' \) | head -n1 || true)"

if [[ -z "$APP_SRC" || -z "$MANIFEST" || -z "$APP_BUILD" || -z "$SETTINGS" ]]; then
  echo "ERROR: could not locate generated Android app/settings" >&2
  exit 1
fi

PKG_DIR="${APP_SRC}/com/freshwater/fwl"
mkdir -p "$PKG_DIR"
cp -f "${OVERLAY}/src/main/java/com/freshwater/fwl/"*.kt "$PKG_DIR/"
echo "Copied Kotlin sources → $PKG_DIR"

# Stage a writable copy of Amethyst so we never dirty the git submodule
rm -rf "$STAGED"
mkdir -p "$STAGED"
echo "Staging Amethyst tree → $STAGED"
cp -a "$AMETHYST/app_pojavlauncher" "$STAGED/"
cp -a "$AMETHYST/jre_lwjgl3glfw" "$STAGED/"
cp -a "$AMETHYST/forge_installer" "$STAGED/"
cp -a "$AMETHYST/arc_dns_injector" "$STAGED/"
cp -a "$AMETHYST/methods_injector_agent" "$STAGED/"
# MobileGlues / SDL live inside jni paths already under app_pojavlauncher when submodules present
if [[ -d "$AMETHYST/MobileGlues" ]]; then
  cp -a "$AMETHYST/MobileGlues" "$STAGED/" || true
fi

python3 "$PATCH_PY" "$STAGED/app_pojavlauncher/build.gradle"
# Library R.id is always non-final — convert the few switch(R.id) sites.
python3 "${ROOT}/android/amethyst-patches/fix-library-switch-rids.py" \
  "$STAGED/app_pojavlauncher"

# Component modules (arc_dns/forge/methods) inherit writeVersionFile() from the
# Amethyst root build.gradle, which is not applied here — inject the helper.
python3 "${ROOT}/android/amethyst-patches/inject-version-helper.py" "$STAGED"

# Manifest / AAR namespace fixes for merger into Tauri app (done after MANIFEST is known)
# — see patch-embed-manifests.py call near end of this script.

# Critical: Tauri-generated project defaults to AGP 8 non-final / non-transitive R,
# which breaks Amethyst Java (switch(R.id.*), portrait-sdp R.dimen._*sdp, AppCompat attrs).
# Match Amethyst's own gradle.properties.
GP="${GEN}/gradle.properties"
touch "$GP"
python3 - <<'PY' "$GP"
from pathlib import Path
import sys
p = Path(sys.argv[1])
text = p.read_text(encoding="utf-8") if p.exists() else ""
lines = []
for key, val in (
    ("android.nonFinalResIds", "false"),
    ("android.nonTransitiveRClass", "false"),
    ("android.useAndroidX", "true"),
):
    prefix = key + "="
    if any(l.startswith(prefix) for l in text.splitlines()):
        text = "\n".join(
            (prefix + val if l.startswith(prefix) else l) for l in text.splitlines()
        )
        if not text.endswith("\n"):
            text += "\n"
    else:
        lines.append(prefix + val)
if lines:
    if text and not text.endswith("\n"):
        text += "\n"
    text += "\n".join(lines) + "\n"
p.write_text(text, encoding="utf-8")
print(f"Patched gradle.properties: {p}")
PY

POJAV_DIR="$STAGED/app_pojavlauncher"
LWJGL_DIR="$STAGED/jre_lwjgl3glfw"
FORGE_DIR="$STAGED/forge_installer"
ARC_DIR="$STAGED/arc_dns_injector"
METHODS_DIR="$STAGED/methods_injector_agent"

# Keep Gradle path :app_pojavlauncher — Amethyst submodules hardcode that name
if ! grep -q "project(\":app_pojavlauncher\")\|project(':app_pojavlauncher')" "$SETTINGS" \
  && ! grep -q 'include.*app_pojavlauncher' "$SETTINGS"; then
  if [[ "$SETTINGS" == *.kts ]]; then
    cat >>"$SETTINGS" <<EOF

// FWL: Amethyst/Pojav play stack (LGPL-3.0) — see NOTICE.android
// Path must remain :app_pojavlauncher (referenced by forge/arc/lwjgl modules)
include(":app_pojavlauncher")
project(":app_pojavlauncher").projectDir = file("$POJAV_DIR")
include(":jre_lwjgl3glfw")
project(":jre_lwjgl3glfw").projectDir = file("$LWJGL_DIR")
include(":jre_lwjgl3glfw:lwjgl-3.3.3")
project(":jre_lwjgl3glfw:lwjgl-3.3.3").projectDir = file("$LWJGL_DIR/lwjgl-3.3.3")
include(":jre_lwjgl3glfw:lwjgl-3.4.1")
project(":jre_lwjgl3glfw:lwjgl-3.4.1").projectDir = file("$LWJGL_DIR/lwjgl-3.4.1")
include(":forge_installer")
project(":forge_installer").projectDir = file("$FORGE_DIR")
include(":arc_dns_injector")
project(":arc_dns_injector").projectDir = file("$ARC_DIR")
include(":methods_injector_agent")
project(":methods_injector_agent").projectDir = file("$METHODS_DIR")
EOF
  else
    cat >>"$SETTINGS" <<EOF

// FWL: Amethyst/Pojav play stack (LGPL-3.0) — see NOTICE.android
include ':app_pojavlauncher'
project(':app_pojavlauncher').projectDir = new File('$POJAV_DIR')
include ':jre_lwjgl3glfw'
project(':jre_lwjgl3glfw').projectDir = new File('$LWJGL_DIR')
include ':jre_lwjgl3glfw:lwjgl-3.3.3'
project(':jre_lwjgl3glfw:lwjgl-3.3.3').projectDir = new File('$LWJGL_DIR/lwjgl-3.3.3')
include ':jre_lwjgl3glfw:lwjgl-3.4.1'
project(':jre_lwjgl3glfw:lwjgl-3.4.1').projectDir = new File('$LWJGL_DIR/lwjgl-3.4.1')
include ':forge_installer'
project(':forge_installer').projectDir = new File('$FORGE_DIR')
include ':arc_dns_injector'
project(':arc_dns_injector').projectDir = new File('$ARC_DIR')
include ':methods_injector_agent'
project(':methods_injector_agent').projectDir = new File('$METHODS_DIR')
EOF
  fi
  echo "Patched settings: $SETTINGS"
else
  echo "settings already includes app_pojavlauncher"
fi

python3 - <<PY
from pathlib import Path
p = Path(r'''$APP_BUILD''')
t = p.read_text(encoding="utf-8")
changed = False

# Depend on Amethyst library
if "app_pojavlauncher" not in t:
    if p.suffix == ".kts":
        dep = 'implementation(project(":app_pojavlauncher"))'
    else:
        dep = "implementation project(':app_pojavlauncher')"
    if "dependencies {" in t:
        t = t.replace("dependencies {", "dependencies {\n    " + dep, 1)
    else:
        t += "\n\ndependencies {\n    " + dep + "\n}\n"
    changed = True

# Resolve duplicate JNI from Amethyst + bytehook AAR at the *app* merge step
if "libbytehook.so" not in t:
    if p.suffix == ".kts":
        pack = '''
android {
    packaging {
        jniLibs {
            pickFirsts += setOf(
                "**/libbytehook.so",
                "**/libc++_shared.so",
            )
        }
    }
}
'''
        # Prefer nesting into existing android { } if present
        if "android {" in t and "packaging {" not in t:
            t = t.replace(
                "android {",
                """android {
    packaging {
        jniLibs {
            pickFirsts += setOf(
                "**/libbytehook.so",
                "**/libc++_shared.so",
            )
        }
    }
""",
                1,
            )
        else:
            t += "\n" + pack
    else:
        pack = """
android {
    packagingOptions {
        pickFirst '**/libbytehook.so'
        pickFirst '**/libc++_shared.so'
    }
}
"""
        if "android {" in t and "packagingOptions" not in t and "packaging {" not in t:
            t = t.replace(
                "android {",
                """android {
    packagingOptions {
        pickFirst '**/libbytehook.so'
        pickFirst '**/libc++_shared.so'
    }
""",
                1,
            )
        else:
            t += "\n" + pack
    changed = True

if changed:
    p.write_text(t, encoding="utf-8")
    print("app build patched (deps + jni pickFirst):", p)
else:
    print("app build already patched:", p)
PY

# Ensure JitPack is available (Kotlin DSL vs Groovy)
python3 - <<'PY' "$SETTINGS" "$GEN"
import sys
from pathlib import Path

settings = Path(sys.argv[1])
gen = Path(sys.argv[2])
text = settings.read_text(encoding="utf-8")
if "jitpack.io" in text:
    print("jitpack already in settings")
    raise SystemExit(0)

if settings.name.endswith(".kts"):
    needle = "mavenCentral()"
    if needle in text:
        text = text.replace(
            needle,
            needle + '\n        maven { url = uri("https://jitpack.io") }',
            1,
        )
        settings.write_text(text, encoding="utf-8")
        print(f"Added jitpack to {settings}")
        raise SystemExit(0)
    # fallback append
    settings.write_text(
        text
        + """

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_SETTINGS)
    repositories {
        google()
        mavenCentral()
        maven { url = uri("https://jitpack.io") }
    }
}
""",
        encoding="utf-8",
    )
    print(f"Appended dependencyResolutionManagement jitpack to {settings}")
else:
    settings.write_text(
        text
        + """

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_SETTINGS)
    repositories {
        google()
        mavenCentral()
        maven { url 'https://jitpack.io' }
    }
}
""",
        encoding="utf-8",
    )
    print(f"Appended groovy jitpack to {settings}")

# Do NOT append Groovy allprojects{} into build.gradle.kts (breaks Kotlin DSL)
for root_build in list(gen.glob("build.gradle.kts")) + list(gen.glob("build.gradle")):
    if "app" in root_build.parts:
        continue
    bt = root_build.read_text(encoding="utf-8")
    if "jitpack.io" in bt:
        continue
    if root_build.suffix == ".kts":
        # Prefer settings injection only for kts roots
        print(f"Skipping groovy-style repo inject for {root_build}")
        continue
    root_build.write_text(
        bt
        + """

allprojects {
    repositories {
        google()
        mavenCentral()
        maven { url 'https://jitpack.io' }
    }
}
""",
        encoding="utf-8",
    )
    print(f"Added groovy jitpack to {root_build}")
PY

python3 "${ROOT}/android/amethyst-patches/patch-embed-manifests.py" \
  "$STAGED/app_pojavlauncher" "$MANIFEST"

echo "Android overlay + Amethyst wiring applied."
