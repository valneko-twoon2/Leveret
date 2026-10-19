---
name: screenshots
description: Capture and refresh the app screenshots (docs/screenshots and the gallery pages). Use after a UI change, a new screen or dialog, or when asked to check how a screen looks.
---

# Screenshots

1. **Scene**: a new screen or state gets a scene in `frontend/src/lib/scenes/<area>.ts` (ids `NN-name`, next free number).
   A scene that depends on clean mock data starts with `mock.reset()` + `app.init()`; real-repo-only steps are `mockOnly`.
2. **Quick check** while working (no Rust build): `scripts\screenshot.bat -Browser -Scenes <ids>`.
   Narrow layout: add `-Size 900x560` (the minimum window). Look at the PNGs in `artifacts\screenshots`.
3. **Refresh the docs**: `scripts\screenshot.bat -Theme both` (app capture, rebuilds when sources changed),
   then copy `artifacts\screenshots\{light,dark}\*.png` over `docs/screenshots/{light,dark}/`.
4. **Gallery**: add a cell for new scenes to both `docs/screenshots.md` and `docs/screenshots.ja.md`
   (two per row, `<picture>` with the dark source). Keep each page within 400 lines.
5. Mock and screenshot user name is `valneko-twoon2`. Commit screenshots with the docs, separately from code.

Pitfalls: scenes run in order and share the mock, so a scene that commits (rebase, reset) changes later ones;
a stale capture means the app was not rebuilt (`-Build`).
