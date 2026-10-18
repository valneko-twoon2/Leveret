use super::*;
use crate::testutil::{self, git, git_out};

fn subjects(p: &Path) -> Vec<String> {
    git_out(p, &["log", "--format=%s"]).lines().map(String::from).collect()
}

fn commit(p: &Path, file: &str, text: &str, subject: &str) -> String {
    std::fs::write(p.join(file), text).unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-qm", subject]);
    git_out(p, &["rev-parse", "HEAD"])
}

fn step(hash: &str, action: Action) -> Step {
    Step {
        hash: hash.into(),
        action,
        message: None,
    }
}

/// A repository in a folder with spaces and Japanese characters, so the
/// sequence editor and exec lines are tested with such paths.
fn repo_with_commits() -> (tempfile::TempDir, std::path::PathBuf, Vec<String>) {
    let outer = testutil::repo();
    let p = outer.path().join("日本語 リポジトリ");
    std::fs::create_dir(&p).unwrap();
    git(&p, &["init", "-q", "-b", "main"]);
    git(&p, &["config", "user.name", "Test"]);
    git(&p, &["config", "user.email", "test@example.com"]);
    let base = commit(&p, "base.txt", "base\n", "base");
    let a = commit(&p, "a.txt", "a\n", "add a");
    let b = commit(&p, "b.txt", "b\n", "add b");
    let c = commit(&p, "c.txt", "c\n", "add c");
    (outer, p, vec![base, a, b, c])
}

#[test]
fn plans_reorders_rewords_squashes_and_drops() {
    let (_dir, p, h) = repo_with_commits();
    let commits = plan(&p, Some(&h[0])).unwrap();
    let names: Vec<&str> = commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(names, ["add a", "add b", "add c"]);
    assert!(commits.iter().all(|c| !c.pushed));

    // c first (reworded), then a with b squashed into it.
    let mut reword = step(&h[3], Action::Reword);
    reword.message = Some("c を追加\n\n本文".into());
    let steps = [reword, step(&h[1], Action::Pick), step(&h[2], Action::Squash)];
    assert_eq!(run(&p, Some(&h[0]), &steps).unwrap(), Outcome::Done);
    assert_eq!(subjects(&p), ["add a", "c を追加", "base"]);
    assert_eq!(git_out(&p, &["log", "-1", "--format=%B", "HEAD~1"]), "c を追加\n\n本文");
    // The squash kept both messages and both files.
    let message = git_out(&p, &["log", "-1", "--format=%B"]);
    assert!(message.contains("add a") && message.contains("add b"), "{message}");
    assert!(p.join("b.txt").exists());
    assert!(operation::current(&p).unwrap().is_none());

    // Fixup and drop, from the very first commit.
    let all = plan(&p, None).unwrap();
    let steps = [
        step(&all[0].hash, Action::Pick),
        step(&all[1].hash, Action::Drop),
        step(&all[2].hash, Action::Fixup),
    ];
    assert_eq!(run(&p, None, &steps).unwrap(), Outcome::Done);
    assert_eq!(subjects(&p), ["base"]);
    assert!(!p.join("c.txt").exists() && p.join("a.txt").exists());
}

#[test]
fn stops_on_conflicts_and_rewords_after_continuing() {
    let (_dir, p, h) = repo_with_commits();
    let x = commit(&p, "a.txt", "x\n", "edit a to x");
    let y = commit(&p, "a.txt", "y\n", "edit a to y");
    // Applying y before x conflicts.
    let mut reword = step(&x, Action::Reword);
    reword.message = Some("x（書き換え）".into());
    let steps = [step(&y, Action::Pick), reword];
    assert_eq!(run(&p, Some(&h[3]), &steps).unwrap(), Outcome::Conflicts);
    let op = operation::current(&p).unwrap().unwrap();
    assert!(op.interactive);

    std::fs::write(p.join("a.txt"), "y\n").unwrap();
    operation::mark_resolved(&p, &["a.txt".into()]).unwrap();
    assert_eq!(operation::continue_(&p).unwrap(), Outcome::Conflicts);
    std::fs::write(p.join("a.txt"), "x\n").unwrap();
    operation::mark_resolved(&p, &["a.txt".into()]).unwrap();
    assert_eq!(operation::continue_(&p).unwrap(), Outcome::Done);
    assert_eq!(subjects(&p)[..2], ["x（書き換え）", "edit a to y"]);
}

#[test]
fn refuses_merges_pushed_commits_and_stale_plans() {
    let (_dir, p, h) = repo_with_commits();
    // Nothing changes: no rebase at all.
    let steps: Vec<Step> = h[1..].iter().map(|c| step(c, Action::Pick)).collect();
    assert_eq!(run(&p, Some(&h[0]), &steps).unwrap(), Outcome::Done);
    assert_eq!(git_out(&p, &["rev-parse", "HEAD"]), h[3]);

    // A missing commit or a squash with nothing before it.
    assert!(matches!(
        run(&p, Some(&h[0]), &steps[..2]),
        Err(AppError::InvalidInput(_))
    ));
    let squash_first = [
        step(&h[1], Action::Squash),
        step(&h[2], Action::Pick),
        step(&h[3], Action::Pick),
    ];
    assert!(matches!(
        run(&p, Some(&h[0]), &squash_first),
        Err(AppError::InvalidInput(_))
    ));

    // Pushed commits would need a force push.
    git(&p, &["update-ref", "refs/remotes/origin/main", &h[2]]);
    assert!(plan(&p, Some(&h[0])).unwrap()[1].pushed);
    let swap = [
        step(&h[2], Action::Pick),
        step(&h[1], Action::Pick),
        step(&h[3], Action::Pick),
    ];
    assert!(matches!(run(&p, Some(&h[0]), &swap), Err(AppError::AlreadyPushed)));
    git(&p, &["update-ref", "-d", "refs/remotes/origin/main"]);

    // Merges in the range.
    git(&p, &["switch", "-qc", "side", &h[1]]);
    commit(&p, "s.txt", "s\n", "side");
    git(&p, &["switch", "-q", "main"]);
    git(&p, &["merge", "-q", "--no-edit", "side"]);
    assert!(matches!(plan(&p, Some(&h[0])), Err(AppError::MergesInRange)));
    // A commit outside the current branch is not a base.
    let side = git_out(&p, &["rev-parse", "side"]);
    git(&p, &["switch", "-q", "--detach", &h[2]]);
    assert!(matches!(plan(&p, Some(&side)), Err(AppError::InvalidInput(_))));
}
