#!/usr/bin/env python3
"""Convert Amethyst app_pojavlauncher application module into an Android library for FWL."""
from __future__ import annotations

import re
import sys
from pathlib import Path


def convert(build_gradle: Path) -> None:
    text = build_gradle.read_text(encoding="utf-8")

    # Must NOT pin AGP version — Tauri already puts AGP on the classpath.
    text = text.replace(
        "id 'com.android.application' version '8.7.2'",
        "id 'com.android.library'",
    )
    text = re.sub(
        r"id\s+['\"]com\.android\.(application|library)['\"]\s+version\s+['\"][^'\"]+['\"]",
        "id 'com.android.library'",
        text,
        count=1,
    )

    # Library modules cannot declare applicationId / applicationIdSuffix
    text = re.sub(r"\s*applicationId\s+[\"'][^\"']+[\"']\s*\n", "\n", text)
    text = re.sub(r"\s*applicationIdSuffix\s+[\"'][^\"']+[\"']\s*\n", "\n", text)

    # App-only DSL
    text = re.sub(r"\s*shrinkResources\s+(true|false)\s*\n", "\n", text)
    text = re.sub(
        r"\n\s*bundle\s*\{(?:[^{}]|\{[^{}]*\})*\}\s*\n",
        "\n",
        text,
        count=1,
        flags=re.M,
    )

    # Neutralize signing that needs vendor keystores
    text = re.sub(
        r"signingConfig\s+signingConfigs\.(customRelease|googlePlayBuild|customDebug)",
        "signingConfig null",
        text,
    )

    # Keep packaging pickFirst for bytehook (also needed at app merge — see patch script)
    if "pickFirst '**/libbytehook.so'" not in text and 'pickFirst("**/libbytehook.so")' not in text:
        text = text.replace(
            "packagingOptions {",
            "packagingOptions {\n        pickFirst '**/libbytehook.so'\n",
            1,
        )

    # Library modules do not auto-emit BuildConfig.VERSION_NAME (app-only).
    if 'buildConfigField "String", "VERSION_NAME"' not in text:
        text = text.replace(
            "multiDexEnabled true //important",
            'multiDexEnabled true //important\n'
            '        buildConfigField "String", "VERSION_NAME", "\\"${getVersionName()}\\""',
            1,
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

    # Component jars write into src/main/assets/components; lint model + resource tasks
    # read that dir without an explicit dep → Gradle validation failure. Disable lint
    # (unneeded for embedded kernel) and order any asset-reading task after the jars.
    if "FWL_LINT_DISABLE" not in text:
        component_jars = (
            "':forge_installer:jar', ':arc_dns_injector:jar', "
            "':methods_injector_agent:jar', ':jre_lwjgl3glfw:lwjgl-3.3.3:jar', "
            "':jre_lwjgl3glfw:lwjgl-3.4.1:jar'"
        )
        text += (
            "\n// FWL_LINT_DISABLE\n"
            "android {\n"
            "    lint {\n"
            "        checkReleaseBuilds false\n"
            "        abortOnError false\n"
            "    }\n"
            "}\n"
            "tasks.configureEach {\n"
            "    if (name.contains('LintModel') || name.startsWith('lintAnalyze')"
            " || name.startsWith('lintReport') || name.startsWith('lintVital')"
            " || name.contains('LintReport') || name.contains('LintVital')) {\n"
            f"        dependsOn({component_jars})\n"
            "    }\n"
            "}\n"
        )

    # App module's Kotlin compile analyzes library classes and must resolve the
    # supertypes they reference (e.g. TouchControllerInputView -> LauncherProxyClient).
    # implementation deps are not exposed to consumers, so promote them to api.
    if "// FWL_API_DEPS" not in text:
        text = re.sub(
            r"(\n[ \t]*)implementation(\s)",
            r"\1api\2",
            text,
        )
        text = text.replace("dependencies {", "dependencies {\n    // FWL_API_DEPS", 1)

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
