use serde::Serialize;
use serde::ser::SerializeStruct;

/// Errors returned to the frontend.
///
/// They are serialized as `{ code, detail }` so the frontend can show a
/// localized message for `code` and fall back to `detail` (raw text, usually
/// git's stderr) when more context is needed. Every code needs an
/// `errors.<code>` message in the frontend locales.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("git executable was not found")]
    GitNotFound,
    #[error("not a git repository: {0}")]
    NotARepository(String),
    #[error("repository is already registered: {0}")]
    RepoAlreadyAdded(String),
    #[error("repository is not registered: {0}")]
    RepoNotRegistered(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("commit summary is empty")]
    EmptySummary,
    #[error("no files selected")]
    NothingSelected,
    #[error("git user identity is not configured")]
    IdentityMissing(String),
    #[error("git command failed: {0}")]
    GitFailed(String),
    #[error("clone target already exists: {0}")]
    CloneTargetExists(String),
    #[error("the current branch has no upstream branch")]
    NoUpstream,
    #[error("the repository has no remote")]
    NoRemote,
    #[error("HEAD is detached")]
    DetachedHead,
    #[error("push was rejected: {0}")]
    PushRejected(String),
    #[error("authentication to the remote failed: {0}")]
    RemoteAuthFailed(String),
    #[error("local changes would be overwritten: {0}")]
    LocalChangesConflict(String),
    #[error("unresolved conflicts remain")]
    ConflictsRemain,
    #[error("conflict markers remain in: {0}")]
    ConflictMarkersRemain(String),
    #[error("no merge, rebase, revert or cherry-pick is in progress")]
    NoOperation,
    #[error("branch already exists: {0}")]
    BranchExists(String),
    #[error("tag already exists: {0}")]
    TagExists(String),
    #[error("branch is not fully merged: {0}")]
    BranchNotMerged(String),
    #[error("there are no local changes")]
    NoChanges,
    #[error("the commit has already been pushed")]
    AlreadyPushed,
    #[error("issue #{0} was not found")]
    IssueNotFound(u64),
    #[error("the repository has no github.com remote")]
    NoGithubRemote,
    #[error("WinMerge was not found")]
    DiffToolNotFound,
    #[error("GitHub OAuth client id is not configured")]
    GithubClientIdMissing,
    #[error("not signed in to GitHub")]
    GithubNotSignedIn,
    #[error("GitHub rejected the credentials")]
    GithubUnauthorized,
    #[error("GitHub sign-in failed: {0}")]
    GithubAuthFailed(String),
    #[error("GitHub API error: {0}")]
    GithubApi(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("credential store error: {0}")]
    Credential(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            AppError::GitNotFound => "gitNotFound",
            AppError::NotARepository(_) => "notARepository",
            AppError::RepoAlreadyAdded(_) => "repoAlreadyAdded",
            AppError::RepoNotRegistered(_) => "repoNotRegistered",
            AppError::InvalidInput(_) => "invalidInput",
            AppError::EmptySummary => "emptySummary",
            AppError::NothingSelected => "nothingSelected",
            AppError::IdentityMissing(_) => "identityMissing",
            AppError::GitFailed(_) => "gitFailed",
            AppError::CloneTargetExists(_) => "cloneTargetExists",
            AppError::NoUpstream => "noUpstream",
            AppError::NoRemote => "noRemote",
            AppError::DetachedHead => "detachedHead",
            AppError::PushRejected(_) => "pushRejected",
            AppError::RemoteAuthFailed(_) => "remoteAuthFailed",
            AppError::LocalChangesConflict(_) => "localChangesConflict",
            AppError::ConflictsRemain => "conflictsRemain",
            AppError::ConflictMarkersRemain(_) => "conflictMarkersRemain",
            AppError::NoOperation => "noOperation",
            AppError::BranchExists(_) => "branchExists",
            AppError::BranchNotMerged(_) => "branchNotMerged",
            AppError::TagExists(_) => "tagExists",
            AppError::NoChanges => "noChanges",
            AppError::AlreadyPushed => "alreadyPushed",
            AppError::IssueNotFound(_) => "issueNotFound",
            AppError::NoGithubRemote => "noGithubRemote",
            AppError::DiffToolNotFound => "diffToolNotFound",
            AppError::GithubClientIdMissing => "githubClientIdMissing",
            AppError::GithubNotSignedIn => "githubNotSignedIn",
            AppError::GithubUnauthorized => "githubUnauthorized",
            AppError::GithubAuthFailed(_) => "githubAuthFailed",
            AppError::GithubApi(_) => "githubApi",
            AppError::Network(_) => "network",
            AppError::Credential(_) => "credential",
            AppError::Io(_) => "io",
            AppError::Other(_) => "other",
        }
    }

    fn detail(&self) -> String {
        match self {
            AppError::NotARepository(s)
            | AppError::RepoAlreadyAdded(s)
            | AppError::RepoNotRegistered(s)
            | AppError::InvalidInput(s)
            | AppError::IdentityMissing(s)
            | AppError::GitFailed(s)
            | AppError::CloneTargetExists(s)
            | AppError::PushRejected(s)
            | AppError::RemoteAuthFailed(s)
            | AppError::LocalChangesConflict(s)
            | AppError::BranchExists(s)
            | AppError::BranchNotMerged(s)
            | AppError::TagExists(s)
            | AppError::ConflictMarkersRemain(s)
            | AppError::GithubAuthFailed(s)
            | AppError::GithubApi(s)
            | AppError::Network(s)
            | AppError::Credential(s)
            | AppError::Other(s) => s.clone(),
            AppError::Io(e) => e.to_string(),
            AppError::IssueNotFound(n) => n.to_string(),
            AppError::GitNotFound
            | AppError::EmptySummary
            | AppError::NothingSelected
            | AppError::NoUpstream
            | AppError::NoRemote
            | AppError::DetachedHead
            | AppError::ConflictsRemain
            | AppError::NoOperation
            | AppError::NoChanges
            | AppError::AlreadyPushed
            | AppError::NoGithubRemote
            | AppError::DiffToolNotFound
            | AppError::GithubClientIdMissing
            | AppError::GithubNotSignedIn
            | AppError::GithubUnauthorized => String::new(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("detail", &self.detail())?;
        s.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// Turns a failed git command into a more specific error when its output
    /// shows a well-known cause (git runs with `LC_ALL=C`, so messages are English).
    pub fn classify_git(self) -> AppError {
        let AppError::GitFailed(msg) = self else { return self };
        let has = |needle: &str| msg.contains(needle);
        if has("would be overwritten by") || has("Please commit your changes or stash them") {
            AppError::LocalChangesConflict(msg)
        } else if has("[rejected]") || has("non-fast-forward") || has("fetch first") || has("[remote rejected]") {
            AppError::PushRejected(msg)
        } else if has("Authentication failed")
            || has("could not read Username")
            || has("could not read Password")
            || has("Permission denied (publickey")
            || has("The requested URL returned error: 403")
            || has("The requested URL returned error: 401")
        {
            AppError::RemoteAuthFailed(msg)
        } else if has("Could not resolve host")
            || has("Failed to connect to")
            || has("Connection timed out")
            || has("Could not read from remote repository")
            || has("unable to access")
        {
            AppError::Network(msg)
        } else if has("user.email") || has("user.name") || has("Please tell me who you are") {
            AppError::IdentityMissing(msg)
        } else {
            AppError::GitFailed(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_git_errors() {
        let c = |m: &str| AppError::GitFailed(m.into()).classify_git().code();
        assert_eq!(c(" ! [rejected]        main -> main (fetch first)"), "pushRejected");
        assert_eq!(
            c("fatal: Authentication failed for 'https://github.com/a/b.git/'"),
            "remoteAuthFailed"
        );
        assert_eq!(
            c("fatal: unable to access 'https://x/': Could not resolve host: x"),
            "network"
        );
        assert_eq!(
            c("error: Your local changes to the following files would be overwritten by checkout:"),
            "localChangesConflict"
        );
        assert_eq!(c("fatal: something else"), "gitFailed");
        assert_eq!(AppError::NoRemote.classify_git().code(), "noRemote");
    }

    #[test]
    fn classify_git_table() {
        for (message, code) in [
            (
                "error: Your local changes to the following files would be overwritten by merge:",
                "localChangesConflict",
            ),
            (
                "error: The following untracked working tree files would be overwritten by checkout:",
                "localChangesConflict",
            ),
            (
                "Please commit your changes or stash them before you switch branches.",
                "localChangesConflict",
            ),
            (" ! [rejected]        main -> main (non-fast-forward)", "pushRejected"),
            (
                " ! [remote rejected] main -> main (protected branch hook declined)",
                "pushRejected",
            ),
            (
                "hint: Updates were rejected because the remote contains work (fetch first)",
                "pushRejected",
            ),
            (
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
                "remoteAuthFailed",
            ),
            (
                "fatal: could not read Password for 'https://x@github.com': No such device",
                "remoteAuthFailed",
            ),
            ("git@github.com: Permission denied (publickey).", "remoteAuthFailed"),
            (
                "fatal: unable to access 'https://x/': The requested URL returned error: 403",
                "remoteAuthFailed",
            ),
            (
                "fatal: unable to access 'https://x/': The requested URL returned error: 401",
                "remoteAuthFailed",
            ),
            (
                "fatal: unable to access 'https://x/': Failed to connect to x port 443",
                "network",
            ),
            ("ssh: connect to host x port 22: Connection timed out", "network"),
            ("fatal: Could not read from remote repository.", "network"),
            (
                "fatal: unable to access 'https://x/': SSL certificate problem",
                "network",
            ),
            ("*** Please tell me who you are.", "identityMissing"),
            (
                "fatal: empty ident name (for <>) not allowed; set user.name",
                "identityMissing",
            ),
            (
                "fatal: not a git repository (or any of the parent directories): .git",
                "gitFailed",
            ),
            ("", "gitFailed"),
        ] {
            assert_eq!(
                AppError::GitFailed(message.into()).classify_git().code(),
                code,
                "{message:?}"
            );
        }
        // Only GitFailed is reclassified.
        for e in [
            AppError::NoUpstream,
            AppError::Other("Authentication failed".into()),
            AppError::DetachedHead,
        ] {
            let code = e.code();
            assert_eq!(e.classify_git().code(), code);
        }
    }

    #[test]
    fn serializes_as_code_and_detail() {
        let json = serde_json::to_value(AppError::TagExists("v1".into())).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "tagExists", "detail": "v1" }));
        let json = serde_json::to_value(AppError::NoRemote).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "noRemote", "detail": "" }));
    }
}
