use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotKey {
    pub repo_root: String,
    pub worktree_root: String,
    pub git_sha: String,
    pub cargo_target: String,
    pub feature_set: Vec<String>,
    pub cfg_profile: String,
}

impl SnapshotKey {
    pub fn new(
        repo_root: impl Into<String>,
        worktree_root: impl Into<String>,
        git_sha: impl Into<String>,
        cargo_target: impl Into<String>,
        feature_set: impl IntoIterator<Item = impl AsRef<str>>,
        cfg_profile: impl Into<String>,
    ) -> Self {
        Self {
            repo_root: repo_root.into(),
            worktree_root: worktree_root.into(),
            git_sha: git_sha.into(),
            cargo_target: cargo_target.into(),
            feature_set: normalize_feature_set(feature_set),
            cfg_profile: cfg_profile.into(),
        }
    }

    pub fn id(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.repo_root,
            self.worktree_root,
            self.git_sha,
            self.cargo_target,
            self.feature_set.join(","),
            self.cfg_profile
        )
    }
}

fn normalize_feature_set(feature_set: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut normalized: Vec<String> = feature_set
        .into_iter()
        .map(|feature| feature.as_ref().trim().to_string())
        .filter(|feature| !feature.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

#[cfg(test)]
mod tests {
    use super::SnapshotKey;

    #[test]
    fn normalizes_feature_set() {
        let key = SnapshotKey::new(
            "/repo",
            "/repo/.worktrees/alpha",
            "abc123",
            "x86_64-unknown-linux-gnu",
            [" sqlite ", "default", "sqlite"],
            "dev",
        );

        assert_eq!(key.feature_set, vec!["default", "sqlite"]);
    }

    #[test]
    fn id_changes_with_worktree_root() {
        let left = SnapshotKey::new(
            "/repo",
            "/repo/.worktrees/alpha",
            "abc123",
            "x86_64-unknown-linux-gnu",
            ["default"],
            "dev",
        );
        let right = SnapshotKey::new(
            "/repo",
            "/repo/.worktrees/beta",
            "abc123",
            "x86_64-unknown-linux-gnu",
            ["default"],
            "dev",
        );

        assert_ne!(left.id(), right.id());
    }
}
