//! Table-driven checks of the validators with hostile input: path traversal,
//! Windows path quirks, option injection, transport tricks and huge values.

use super::*;

fn accepts(name: &str, valid: impl Fn(&str) -> bool, ok: &[&str]) {
    for input in ok {
        assert!(valid(input), "{name} should accept {input:?}");
    }
}

fn rejects(name: &str, valid: impl Fn(&str) -> bool, bad: &[&str]) {
    for input in bad {
        assert!(!valid(input), "{name} should reject {input:?}");
    }
}

#[test]
fn relative_path_table() {
    accepts(
        "relative_path",
        |s| relative_path(s).is_ok(),
        &[
            "a.txt",
            "src/main.rs",
            "src\\main.rs",
            "./a.txt",
            "docs/設計メモ.md",
            "dir with space/file name.txt",
            ".gitignore",
            ".github/workflows/ci.yml",
            "a..b.txt",
            "...txt",
            "console.log",
            "nullable.rs",
            "com10.txt",
            "lpt.txt",
            "#hash%20&;'$()!.txt",
        ],
    );
    let long = "a/".repeat(2049);
    rejects(
        "relative_path",
        |s| relative_path(s).is_ok(),
        &[
            "",
            // Escaping the repository.
            "..",
            "../x",
            "a/../../x",
            "a\\..\\..\\x",
            "/etc/passwd",
            "\\Windows\\win.ini",
            "C:\\Windows",
            "C:x",
            "c:/x",
            "\\\\server\\share\\x",
            "\\\\?\\C:\\x",
            "//server/share",
            // Windows drops trailing dots and spaces: ".. " would act as "..".
            ".. /x",
            ".. ",
            "... /x",
            "a/.. /b",
            "a./b",
            "a /b",
            "file.",
            "file ",
            // Alternate data streams.
            "a.txt:secret",
            "a.txt::$DATA",
            // Devices.
            "NUL",
            "nul.txt",
            "con",
            "src/aux/x.rs",
            "COM1",
            "lpt9.log",
            "CON .txt",
            "prn.tar.gz",
            // Option injection and control characters.
            "-rf",
            "--output=x",
            "a\0b",
            "a\nb",
            "a\rb",
            "a\tb",
            "\u{7f}",
            long.as_str(),
        ],
    );
}

#[test]
fn object_id_table() {
    accepts(
        "object_id",
        |s| object_id(s).is_ok(),
        &[
            "abcd",
            "ABCDEF12",
            "0123456789abcdef0123456789abcdef01234567",
            &"f".repeat(64),
        ],
    );
    rejects(
        "object_id",
        |s| object_id(s).is_ok(),
        &[
            "",
            "abc",
            "g123",
            "HEAD",
            "HEAD~1",
            "main",
            "-n",
            "--output=x",
            "abc def",
            "abcd\n",
            &"a".repeat(65),
            "１２３４",
        ],
    );
}

#[test]
fn clone_url_table() {
    accepts(
        "clone_url",
        |s| clone_url(s).is_ok(),
        &[
            "https://github.com/a/b",
            "https://github.com/a/b.git",
            "https://user@example.com:8443/a/b.git",
            "ssh://git@github.com/a/b.git",
            "ssh://git@example.com:2222/a/b",
            "git@github.com:a/b.git",
        ],
    );
    let long = format!("https://github.com/{}", "a".repeat(2048));
    rejects(
        "clone_url",
        |s| clone_url(s).is_ok(),
        &[
            "",
            "github.com/a/b",
            "http://github.com/a/b",
            "file:///C:/repo",
            "file://server/share",
            "C:\\repo",
            "C:/repo",
            "\\\\server\\share",
            "/local/path:x",
            "./repo",
            "ext::sh -c touch% /tmp/x",
            "fd::17",
            "https::evil",
            "transport::https://github.com/a/b",
            "-uhttps://x",
            "--upload-pack=evil",
            "https://x y",
            "https://x\ny",
            "https://x\0",
            ":a/b",
            "@host:path",
            "user@/path:x",
            long.as_str(),
        ],
    );
}

#[test]
fn folder_name_table() {
    accepts(
        "folder_name",
        |s| folder_name(s).is_ok(),
        &["leveret", "my.repo", "リポジトリ", "a b", "console", "com10"],
    );
    rejects(
        "folder_name",
        |s| folder_name(s).is_ok(),
        &[
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "a:b",
            "a*b",
            "a?b",
            "a\"b",
            "a<b",
            "a>b",
            "a|b",
            "con",
            "CON",
            "nul.txt",
            "Aux",
            "com1",
            "LPT9.repo",
            "trail.",
            "trail ",
            "-rf",
            "a\0b",
            "a\nb",
            &"x".repeat(256),
        ],
    );
}

#[test]
fn ref_name_table() {
    let ok = [
        "main",
        "feature/login",
        "機能/ログイン",
        "fix-1.2",
        "user@host",
        "v1.0.0",
        "release/2026-10",
        "a.b/c_d",
    ];
    accepts("branch_name", |s| branch_name(s).is_ok(), &ok);
    accepts("tag_name", |s| tag_name(s).is_ok(), &ok);
    let long = "a".repeat(256);
    let bad = [
        "",
        "-f",
        "--force",
        "-",
        "a b",
        " a",
        "a ",
        "a..b",
        "a~1",
        "a^",
        "a^{}",
        "a:b",
        "a?",
        "a*",
        "a[",
        "a\\b",
        "/a",
        "a/",
        "a//b",
        ".hidden",
        "a/.b",
        "a.lock",
        "a/b.lock",
        "a.",
        "@",
        "a@{1}",
        "@{-1}",
        "HEAD",
        "a\tb",
        "a\0b",
        "a\nb",
        "a\u{7f}b",
        long.as_str(),
    ];
    rejects("branch_name", |s| branch_name(s).is_ok(), &bad);
    rejects("tag_name", |s| tag_name(s).is_ok(), &bad);
}

#[test]
fn messages_table() {
    accepts(
        "tag_message",
        |s| tag_message(s).is_ok(),
        &["", "リリース", "line 1\nline 2\r\n\ttabbed"],
    );
    rejects(
        "tag_message",
        |s| tag_message(s).is_ok(),
        &["a\0b", "a\u{1b}[31m", "a\u{7}", &"x".repeat(10_001)],
    );
    accepts(
        "stash_message",
        |s| stash_message(s).is_ok(),
        &["", "作業中", &"x".repeat(1000)],
    );
    rejects(
        "stash_message",
        |s| stash_message(s).is_ok(),
        &["line 1\nline 2", "a\tb", "a\0b", &"x".repeat(1001)],
    );
    accepts(
        "search_text",
        |s| search_text(s).is_ok(),
        &["fix", "[wip] *", "-S--danger", "日本語", &"x".repeat(200)],
    );
    rejects(
        "search_text",
        |s| search_text(s).is_ok(),
        &["", "  ", "a\nb", "a\0b", &"x".repeat(201)],
    );
}
