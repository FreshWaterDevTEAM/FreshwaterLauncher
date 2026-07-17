#!/usr/bin/env python3
"""Rewrite switch(R.id.*) to if/else in staged Amethyst sources.

Library R fields are non-final (ADT 14+ / AGP); switch cases require compile-time
constants. android.nonFinalResIds=false only affects application modules.
"""
from __future__ import annotations

import sys
from pathlib import Path

OLD = """\
        switch (v.getId()) {
            case R.id.installmod_mouse_pri:
                AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON1_DOWN_MASK, isDown);
                break;
                
            case R.id.installmod_mouse_sec:
                AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON3_DOWN_MASK, isDown);
                break;
        }
        if(isDown) switch(v.getId()) {
            case R.id.installmod_window_moveup:
                AWTInputBridge.nativeMoveWindow(0, -10);
                break;
            case R.id.installmod_window_movedown:
                AWTInputBridge.nativeMoveWindow(0, 10);
                break;
            case R.id.installmod_window_moveleft:
                AWTInputBridge.nativeMoveWindow(-10, 0);
                break;
            case R.id.installmod_window_moveright:
                AWTInputBridge.nativeMoveWindow(10, 0);
                break;
        }
"""

NEW = """\
        int viewId = v.getId();
        if (viewId == R.id.installmod_mouse_pri) {
            AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON1_DOWN_MASK, isDown);
        } else if (viewId == R.id.installmod_mouse_sec) {
            AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON3_DOWN_MASK, isDown);
        }
        if (isDown) {
            if (viewId == R.id.installmod_window_moveup) {
                AWTInputBridge.nativeMoveWindow(0, -10);
            } else if (viewId == R.id.installmod_window_movedown) {
                AWTInputBridge.nativeMoveWindow(0, 10);
            } else if (viewId == R.id.installmod_window_moveleft) {
                AWTInputBridge.nativeMoveWindow(-10, 0);
            } else if (viewId == R.id.installmod_window_moveright) {
                AWTInputBridge.nativeMoveWindow(10, 0);
            }
        }
"""


def patch_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if "viewId == R.id.installmod_mouse_pri" in text:
        print(f"already patched: {path}")
        return False
    if OLD not in text:
        # tolerate CRLF
        alt = OLD.replace("\n", "\r\n")
        if alt in text:
            text = text.replace(alt, NEW.replace("\n", "\r\n"))
            path.write_text(text, encoding="utf-8")
            print(f"patched (CRLF): {path}")
            return True
        raise SystemExit(f"expected switch block not found in {path}")
    path.write_text(text.replace(OLD, NEW), encoding="utf-8")
    print(f"patched: {path}")
    return True


def main() -> int:
    if len(sys.argv) != 2:
        print(
            "usage: fix-library-switch-rids.py <staged app_pojavlauncher dir>",
            file=sys.stderr,
        )
        return 2
    root = Path(sys.argv[1])
    target = root / "src/main/java/net/kdt/pojavlaunch/JavaGUILauncherActivity.java"
    if not target.is_file():
        print(f"missing {target}", file=sys.stderr)
        return 1
    patch_file(target)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
