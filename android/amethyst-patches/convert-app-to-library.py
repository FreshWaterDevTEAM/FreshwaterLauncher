#!/usr/bin/env python3
"""Convert Amethyst app_pojavlauncher application module into an Android library for FWL."""
from __future__ import annotations

import re
import sys
from pathlib import Path


def convert(build_gradle: Path) -> None:
    text = build_gradle.read_text(encoding="utf-8")

    text = text.replace(
        "id 'com.android.application' version '8.7.2'",
        "id 'com.android.library' version '8.7.2'",
    )

    text = re.sub(r"\s*applicationId\s+[\"'][^\"']+[\"']\s*\n", "\n", text)

    # Neutralize signing that needs vendor keystores
    text = re.sub(
        r"signingConfig\s+signingConfigs\.(customRelease|googlePlayBuild|customDebug)",
        "signingConfig null",
        text,
    )

    # Library asset merge task wiring
    text = text.replace(
        "tasks.mergeDebugAssets.dependsOn(",
        "tasks.findByName('mergeDebugAssets')?.dependsOn(",
    )
    text = text.replace(
        "tasks.mergeReleaseAssets.dependsOn(",
        "tasks.findByName('mergeReleaseAssets')?.dependsOn(",
    )

    marker = "// FWL_LIBRARY_PATCH"
    if marker not in text:
        text = marker + "\n" + text

    build_gradle.write_text(text, encoding="utf-8")
    print(f"Patched {build_gradle} → com.android.library")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: convert-app-to-library.py <app_pojavlauncher/build.gradle>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    if not path.is_file():
        print(f"missing {path}", file=sys.stderr)
        return 1
    convert(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
