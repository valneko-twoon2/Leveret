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
    fn code(&self) -> &'static str {
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
            | AppError::GithubAuthFailed(s)
            | AppError::GithubApi(s)
            | AppError::Network(s)
            | AppError::Credential(s)
            | AppError::Other(s) => s.clone(),
            AppError::Io(e) => e.to_string(),
            AppError::GitNotFound
            | AppError::EmptySummary
            | AppError::NothingSelected
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
