//! Opening diffs in WinMerge, the external diff tool.
//!
//! Only WinMerge found through its installer's registry key (or its default
//! install folders) is ever started: the webview can't name a program to run.
//! Old versions of files are written to `%TEMP%\leveret-diff`, which is cleaned
//! up on later opens (WinMerge may hand files to an already running instance, so
//! the folder can't be removed when the started process exits).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::error::{AppError, Result};
use crate::git::Git;

/// Temporary folders older than this are removed.
const KEEP_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Where WinMerge is installed, if it is.
pub fn find() -> Option<PathBuf> {
    registered()
        .into_iter()
        .chain(default_locations())
        .find(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
}

fn default_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(dir) = std::env::var_os(var) {
            out.push(PathBuf::from(dir).join(r"WinMerge\WinMergeU.exe"));
        }
    }
    if let Some(dir) = dirs::data_local_dir() {
        out.push(dir.join(r"Programs\WinMerge\WinMergeU.exe"));
    }
    out
}

/// `Executable` under `Software\Thingamahoochie\WinMerge` (current user first).
#[cfg(windows)]
fn registered() -> Vec<PathBuf> {
    use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::w;

    let read = |root: HKEY| -> Option<PathBuf> {
        let mut buf = vec![0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: `buf` is valid for `size` bytes and outlives the call; the
        // key and value names are static, NUL-terminated strings.
        let status = unsafe {
            RegGetValueW(
                root,
                w!(r"Software\Thingamahoochie\WinMerge"),
                w!("Executable"),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if status.is_err() {
            return None;
        }
        let len = (size as usize / 2).min(buf.len());
        let text = String::from_utf16_lossy(&buf[..len]);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| PathBuf::from(text))
    };
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE].into_iter().filter_map(read).collect()
}

#[cfg(not(windows))]
fn registered() -> Vec<PathBuf> {
    Vec::new()
}

/// One side of a comparison: a path to open and the caption WinMerge shows.
struct Side {
    file: PathBuf,
    caption: String,
}

/// A fresh folder for one comparison, removing those left by earlier ones.
fn temp_folder() -> Result<PathBuf> {
    let base = std::env::temp_dir().join("leveret-diff");
    std::fs::create_dir_all(&base)?;
    if let Ok(entries) = std::fs::read_dir(&base) {
        let now = SystemTime::now();
        for entry in entries.flatten() {
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| now.duration_since(t).unwrap_or_default() > KEEP_FOR);
            if old {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(tempfile::Builder::new().prefix("d").tempdir_in(&base)?.keep())
}

/// Writes `rev:path` (with the repository's filters applied, as a checkout would)
/// to `dir/<file name>`. A path missing from `rev` gives an empty file.
fn extract(repo: &Path, rev: &str, path: &str, dir: &Path) -> Result<PathBuf> {
    let out = Git::new(repo)
        .args(["cat-file", "--filters", &format!("{rev}:{path}")])
        .ok_codes(&[0, 128])
        .run()?;
    write(path, dir, if out.code == 0 { &out.stdout } else { &[] })
}

/// An empty stand-in for a file that doesn't exist on one side.
fn empty(path: &str, dir: &Path) -> Result<PathBuf> {
    write(path, dir, &[])
}

/// Writes `bytes` to `dir/<file name of path>`, keeping the extension so
/// WinMerge picks the right syntax highlighting.
fn write(path: &str, dir: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let name = path.rsplit('/').next().unwrap_or(path);
    std::fs::create_dir_all(dir)?;
    let file = dir.join(name);
    std::fs::write(&file, bytes)?;
    Ok(file)
}

/// Opens the uncommitted changes of `path`: HEAD against the working tree file
/// (editable, so changes can be made right there), or for a file with merge
/// conflicts a 3-way merge whose result is saved to the working tree file.
pub fn open_working(repo: &Path, path: &str, orig_path: Option<&str>) -> Result<()> {
    let exe = find().ok_or(AppError::DiffToolNotFound)?;
    let unmerged = Git::new(repo).args(["ls-files", "-u", "-z", "--", path]).run()?;
    let dir = temp_folder()?;
    // Native separators, for WinMerge's captions and its recent files.
    let file = path.split('/').fold(repo.to_path_buf(), |p, part| p.join(part));
    if !unmerged.stdout.is_empty() {
        let side = |stage: u8, folder: &str, caption: &str| -> Result<Side> {
            Ok(Side {
                file: extract(repo, &format!(":{stage}"), path, &dir.join(folder))?,
                caption: format!("{caption}: {path}"),
            })
        };
        let ours = side(2, "ours", "HEAD")?;
        let base = side(1, "base", "base")?;
        let theirs = side(3, "theirs", "incoming")?;
        return start(&exe, &[ours, base, theirs], Some(&file));
    }
    let head = Side {
        file: extract(repo, "HEAD", orig_path.unwrap_or(path), &dir.join("head"))?,
        caption: format!("HEAD: {}", orig_path.unwrap_or(path)),
    };
    let working = Side {
        // A deleted file is compared with an empty one.
        file: if file.is_file() { file } else { empty(path, &dir.join("working"))? },
        caption: path.to_string(),
    };
    start(&exe, &[head, working], None)
}

/// Opens one file of a commit against `parent` (a revision; `None` for a root
/// commit). Both sides are read-only.
pub fn open_commit(repo: &Path, hash: &str, parent: Option<&str>, path: &str, orig_path: Option<&str>) -> Result<()> {
    let exe = find().ok_or(AppError::DiffToolNotFound)?;
    let dir = temp_folder()?;
    let old_path = orig_path.unwrap_or(path);
    let old = match parent {
        Some(p) => extract(repo, p, old_path, &dir.join("old"))?,
        None => empty(old_path, &dir.join("old"))?,
    };
    let short = |rev: &str| rev.chars().take(7).collect::<String>();
    let old = Side {
        file: old,
        caption: format!("{}: {old_path}", parent.map_or("(none)".into(), short)),
    };
    let new = Side {
        file: extract(repo, hash, path, &dir.join("new"))?,
        caption: format!("{}: {path}", short(hash)),
    };
    start(&exe, &[old, new], None)
}

/// Starts WinMerge without waiting for it. With `output`, the sides are
/// HEAD / base / incoming and saving writes the merge result to `output`.
fn start(exe: &Path, sides: &[Side], output: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new(exe);
    // Close with Esc, keep these files out of the recent list.
    cmd.args(["/e", "/u"]);
    match (sides, output) {
        ([left, base, right], Some(out)) => {
            cmd.args(["/wl", "/wr", "/fm", "/dl"])
                .arg(&left.caption)
                .arg("/dm")
                .arg(&base.caption)
                .arg("/dr")
                .arg(&right.caption)
                .args([&left.file, &base.file, &right.file])
                .arg("/o")
                .arg(out);
        }
        ([left, right], None) => {
            // The right side stays editable only when it is the working tree file.
            let right_temp = right.file.starts_with(std::env::temp_dir().join("leveret-diff"));
            cmd.arg("/wl");
            if right_temp {
                cmd.arg("/wr");
            }
            cmd.arg("/dl").arg(&left.caption).arg("/dr").arg(&right.caption).args([&left.file, &right.file]);
        }
        _ => return Err(AppError::Other("unsupported comparison".into())),
    }
    cmd.spawn().map_err(AppError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    #[test]
    fn extracts_files_from_revisions_and_stages() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "user.email", "test@example.com"]);
        git(p, &["config", "core.autocrlf", "false"]);
        std::fs::create_dir(p.join("src")).unwrap();
        std::fs::write(p.join("src/メモ.txt"), "base\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "base"]);
        git(p, &["switch", "-qc", "other"]);
        std::fs::write(p.join("src/メモ.txt"), "theirs\n").unwrap();
        git(p, &["commit", "-qam", "theirs"]);
        git(p, &["switch", "-q", "main"]);
        std::fs::write(p.join("src/メモ.txt"), "ours\n").unwrap();
        git(p, &["commit", "-qam", "ours"]);
        Git::new(p).args(["merge", "-q", "other"]).ok_codes(&[0, 1]).run().unwrap();

        let out = tempfile::tempdir().unwrap();
        let file = extract(p, "HEAD~1", "src/メモ.txt", &out.path().join("a")).unwrap();
        assert_eq!(file.file_name().unwrap(), "メモ.txt");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "base\n");
        for (stage, text) in [(1, "base\n"), (2, "ours\n"), (3, "theirs\n")] {
            let file = extract(p, &format!(":{stage}"), "src/メモ.txt", &out.path().join(stage.to_string())).unwrap();
            assert_eq!(std::fs::read_to_string(file).unwrap(), text);
        }
        // Missing on that side: an empty file.
        let file = extract(p, "HEAD", "src/new.txt", &out.path().join("b")).unwrap();
        assert_eq!(std::fs::read(file).unwrap(), b"");
    }
}
