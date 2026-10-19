---
name: release
description: Bump the version and build a release to artifacts\<version>. Use when asked to release, publish, build the exe or bump the version.
---

# Release

1. Working tree clean, on `main`.
2. `scripts\security.bat` (static rules, `npm audit`, `cargo audit`, security tests). Fix or report findings.
3. If dependencies changed: `scripts\licenses.bat` (PowerShell 7) and commit the regenerated list.
4. `scripts\version.bat <x.y.z | major | minor | patch>`; commit as `chore: release x.y.z`.
5. `scripts\publish.bat` runs `test.bat` then builds `artifacts\<version>\Leveret.exe` and the zip.
   `-SkipTests` only when the checks just passed on the same commit.
6. Report the exe and zip sizes and the build time. A cold build takes about 4 minutes, a rebuild about 2.

Do not push, tag or upload anywhere unless asked.
