use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorktreeChanges {
    changed_paths: BTreeSet<String>,
}

impl WorktreeChanges {
    pub fn from_paths<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let changed_paths = paths
            .into_iter()
            .map(|path| normalize_path(path.as_ref()))
            .collect();
        Self { changed_paths }
    }

    pub fn matches(&self, file_path: &str) -> bool {
        if self.changed_paths.is_empty() {
            return false;
        }

        let file_path = normalize_string_path(file_path);
        self.changed_paths
            .iter()
            .any(|changed| file_path == *changed || file_path.ends_with(&format!("/{changed}")))
    }

    pub fn is_empty(&self) -> bool {
        self.changed_paths.is_empty()
    }

    pub fn paths(&self) -> Vec<String> {
        self.changed_paths.iter().cloned().collect()
    }
}

fn normalize_path(path: &Path) -> String {
    normalize_string_path(&path.display().to_string())
}

fn normalize_string_path(path: &str) -> String {
    path.replace('\\', "/")
}
