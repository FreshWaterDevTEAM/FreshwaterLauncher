#!/usr/bin/env python3
"""Inject writeVersionFile()/hashFileWithDigest() into staged Amethyst modules.

Amethyst defines these as top-level static methods in its root build.gradle, which
subprojects inherit. FWL stages the component modules into the Tauri-generated
Gradle project, so the root script is not applied — inject the helpers directly.

Groovy/Gradle constraints:
- import statements must precede the plugins {} block
- the plugins {} block must be the first *statement*, so method defs go after it
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

IMPORT_LINE = "import java.security.MessageDigest"

METHODS = """\

// FWL_VERSION_HELPER — Amethyst root build.gradle is not applied to staged modules
static void hashFileWithDigest(File fileToHash, MessageDigest digest){
    fileToHash.withInputStream { is ->
        byte[] buffer = new byte[8192]
        int read
        while ((read = is.read(buffer)) != -1) {
            digest.update(buffer, 0, read)
        }
    }
}

static void writeVersionFile(File jarFile, File versionFile){
    def sha1 = MessageDigest.getInstance("SHA-1")
    hashFileWithDigest(jarFile, sha1)
    versionFile.write(sha1.digest().collect { String.format("%02x", it) }.join())
}

static void writeVersionFile(File[] jarFileArray, File versionFile){
    def sha1 = MessageDigest.getInstance("SHA-1")
    jarFileArray.each {jarFile ->
        hashFileWithDigest(jarFile, sha1)
    }
    versionFile.write(sha1.digest().collect { String.format("%02x", it) }.join())
}
"""

MODULES = ("arc_dns_injector", "forge_installer", "methods_injector_agent")


def _plugins_block_end(text: str) -> int:
    """Return index just after the top-level plugins {} block, or -1."""
    m = re.search(r"plugins\s*\{", text)
    if not m:
        return -1
    depth = 0
    for i in range(m.start(), len(text)):
        c = text[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
    return -1


def inject(build_gradle: Path) -> None:
    text = build_gradle.read_text(encoding="utf-8")
    if "FWL_VERSION_HELPER" in text:
        print(f"already injected: {build_gradle}")
        return
    if "static void writeVersionFile" in text:
        print(f"module defines helper already: {build_gradle}")
        return

    if IMPORT_LINE not in text:
        text = IMPORT_LINE + "\n" + text

    end = _plugins_block_end(text)
    if end == -1:
        # No plugins block — safe to append methods at end
        text = text + "\n" + METHODS
    else:
        text = text[:end] + "\n" + METHODS + text[end:]

    build_gradle.write_text(text, encoding="utf-8")
    print(f"injected version helper: {build_gradle}")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: inject-version-helper.py <staged tree>", file=sys.stderr)
        return 2
    staged = Path(sys.argv[1])
    for mod in MODULES:
        bg = staged / mod / "build.gradle"
        if bg.is_file():
            inject(bg)
        else:
            print(f"skip missing module: {bg}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
