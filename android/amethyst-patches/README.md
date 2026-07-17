# Amethyst / Pojav patches for FreshwaterLauncher

These scripts adapt [Amethyst-Android](https://github.com/AngelAuraMC/Amethyst-Android) so it can be linked into the Tauri Android app as a **library** (LGPL-3.0).

- `convert-app-to-library.py` — applied to a **staged copy** under `src-tauri/gen/android/fwl_amethyst_tree/` by `scripts/patch-android-project.sh` (never mutates the git submodule in place). Also adds `BuildConfig.VERSION_NAME` (library modules do not emit it automatically).

Host project must set (done by `patch-android-project.sh`):

```
android.nonFinalResIds=false
android.nonTransitiveRClass=false
```

See `NOTICE.android` at the repo root.
