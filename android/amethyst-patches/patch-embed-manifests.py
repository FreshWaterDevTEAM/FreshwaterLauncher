#!/usr/bin/env python3
"""Patch staged Amethyst manifests / AARs so they merge into the Tauri app."""
from __future__ import annotations

import re
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

ANDROID = "{http://schemas.android.com/apk/res/android}"
TOOLS = "{http://schemas.android.com/tools}"


def _strip_ns(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def patch_library_manifest(path: Path) -> None:
    ET.register_namespace("android", "http://schemas.android.com/apk/res/android")
    ET.register_namespace("tools", "http://schemas.android.com/tools")
    tree = ET.parse(path)
    root = tree.getroot()

    app = root.find("application")
    if app is None:
        raise SystemExit(f"no <application> in {path}")

    # Host app owns Application / theme / process / icons / label.
    for attr in (
        f"{ANDROID}name",
        f"{ANDROID}theme",
        f"{ANDROID}process",
        f"{ANDROID}icon",
        f"{ANDROID}roundIcon",
        f"{ANDROID}label",
    ):
        if attr in app.attrib:
            del app.attrib[attr]

    # Avoid a second launcher icon from Amethyst's TestStorageActivity.
    for activity in app.findall("activity"):
        name = activity.attrib.get(f"{ANDROID}name", "")
        if name.endswith(".TestStorageActivity") or name == ".TestStorageActivity":
            for intent in list(activity.findall("intent-filter")):
                actions = [
                    a.attrib.get(f"{ANDROID}name", "")
                    for a in intent.findall("action")
                ]
                if "android.intent.action.MAIN" in actions:
                    activity.remove(intent)

    tree.write(path, encoding="utf-8", xml_declaration=True)
    # ElementTree may drop the tools xmlns if unused; ensure file stays valid UTF-8
    text = path.read_text(encoding="utf-8")
    if "xmlns:tools" not in text and "tools:" in text:
        text = text.replace(
            "<manifest ",
            '<manifest xmlns:tools="http://schemas.android.com/tools" ',
            1,
        )
        path.write_text(text, encoding="utf-8")
    print(f"patched library manifest: {path}")


def patch_lwjgl_aar_namespaces(libs_dir: Path) -> None:
    mapping = {
        "lwjgl-3.3.3-natives-release.aar": "org.angelauramc.lwjgl3x.v333",
        "lwjgl-3.4.1-natives-release.aar": "org.angelauramc.lwjgl3x.v341",
    }
    for name, new_pkg in mapping.items():
        aar = libs_dir / name
        if not aar.is_file():
            print(f"skip missing AAR: {aar}")
            continue
        tmp = aar.with_suffix(".aar.tmp")
        with zipfile.ZipFile(aar, "r") as zin, zipfile.ZipFile(
            tmp, "w", compression=zipfile.ZIP_DEFLATED
        ) as zout:
            for info in zin.infolist():
                data = zin.read(info.filename)
                if info.filename == "AndroidManifest.xml":
                    text = data.decode("utf-8")
                    text2 = re.sub(
                        r'package="[^"]+"',
                        f'package="{new_pkg}"',
                        text,
                        count=1,
                    )
                    if text2 == text:
                        raise SystemExit(f"package= not found in {aar}")
                    data = text2.encode("utf-8")
                    print(f"{name}: namespace → {new_pkg}")
                zout.writestr(info, data)
        tmp.replace(aar)


def patch_host_manifest(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    changed = False

    if "xmlns:tools=" not in text:
        text = re.sub(
            r"(<manifest\b)",
            r'\1 xmlns:tools="http://schemas.android.com/tools"',
            text,
            count=1,
        )
        changed = True

    # Prefer host theme over Amethyst AppTheme
    if "tools:replace=" not in text:
        text2, n = re.subn(
            r"(<application\b)([^>]*)(>)",
            r'\1\2 tools:replace="android:theme"\3',
            text,
            count=1,
        )
        if n:
            text = text2
            changed = True
    elif 'tools:replace="android:theme"' not in text and "tools:replace=" in text:
        text2, n = re.subn(
            r'tools:replace="([^"]*)"',
            lambda m: f'tools:replace="{m.group(1)},android:theme"'
            if "android:theme" not in m.group(1)
            else m.group(0),
            text,
            count=1,
        )
        if n:
            text = text2
            changed = True

    # Library already declares MainActivity — drop any host duplicate.
    if "net.kdt.pojavlaunch.MainActivity" in text:
        text2, n = re.subn(
            r"\s*<activity\b[^>]*android:name=\"net\.kdt\.pojavlaunch\.MainActivity\"[^>]*(?:/>|>.*?</activity>)",
            "",
            text,
            count=1,
            flags=re.S,
        )
        if n:
            text = text2
            changed = True
            print("removed duplicate MainActivity from host manifest")

    if "com.freshwater.fwl.FwlGameActivity" not in text:
        block = """
        <activity
            android:name="com.freshwater.fwl.FwlGameActivity"
            android:exported="false"
            android:configChanges="orientation|screenSize|keyboard|keyboardHidden|navigation"
            android:launchMode="singleTask"
            android:theme="@android:style/Theme.Black.NoTitleBar.Fullscreen" />
"""
        if "</application>" not in text:
            raise SystemExit(f"no </application> in {path}")
        text = text.replace("</application>", block + "\n    </application>", 1)
        changed = True

    if "android.permission.INTERNET" not in text:
        text2, n = re.subn(
            r"(<manifest\b[^>]*>)",
            r"""\1
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
""",
            text,
            count=1,
        )
        if n:
            text = text2
            changed = True

    if changed:
        path.write_text(text, encoding="utf-8")
        print(f"patched host manifest: {path}")
    else:
        print(f"host manifest already ok: {path}")


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: patch-embed-manifests.py <staged app_pojavlauncher> <host AndroidManifest.xml>",
            file=sys.stderr,
        )
        return 2
    staged = Path(sys.argv[1])
    host_manifest = Path(sys.argv[2])
    lib_manifest = staged / "src/main/AndroidManifest.xml"
    if not lib_manifest.is_file():
        print(f"missing {lib_manifest}", file=sys.stderr)
        return 1
    if not host_manifest.is_file():
        print(f"missing {host_manifest}", file=sys.stderr)
        return 1

    patch_library_manifest(lib_manifest)
    patch_lwjgl_aar_namespaces(staged / "libs")
    patch_host_manifest(host_manifest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
