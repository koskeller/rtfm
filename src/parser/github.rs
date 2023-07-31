use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::types::Source;

pub struct GitHubParser {
    source: Source,
    client: Octocrab,
}

impl GitHubParser {
    pub fn new(source: Source, client: Octocrab) -> Self {
        Self { source, client }
    }

    pub async fn get_paths(&self) -> Result<Vec<Path>> {
        let route = format!(
            "/repos/{}/{}/git/trees/{}?recursive='true'",
            &self.source.owner, &self.source.repo, &self.source.branch
        );
        tracing::info!("Getting git tree {}", route);
        let resp: TreeResponse = self.client.get(route, None::<&()>).await?;

        let paths: Vec<Path> = resp
            .tree
            .into_iter()
            .filter_map(|file| match file.tree_type {
                TreeType::Blob if self.is_target_file(&file.path) => Some(file.path),
                _ => None,
            })
            .collect();

        tracing::info!("Got {} paths", paths.len());
        Ok(paths)
    }

    pub fn filter_paths(&self, files: Vec<Path>) -> Vec<Path> {
        files
            .into_iter()
            .filter(|x| self.is_target_file(x))
            .collect()
    }

    // pub async fn get_changed_files(
    //     &self,
    //     since: DateTime<Utc>,
    // ) -> Result<HashMap<Path, FileStatus>> {
    //     let repository = self.client.repos(&self.source.owner, &self.source.repo);

    //     let mut paths: HashMap<Path, FileStatus> = HashMap::new();
    //     let mut page: u32 = 1;
    //     loop {
    //         let commits = repository
    //             .list_commits()
    //             .since(since)
    //             .per_page(100)
    //             .page(page)
    //             .send()
    //             .await?;

    //         for commit in commits.items {
    //             let route = format!(
    //                 "/repos/{}/{}/commits/{}",
    //                 self.source.owner, self.source.repo, commit.sha
    //             );
    //             let commit: Commit = self.client.get(route, None::<&()>).await?;
    //             for file in commit.files {
    //                 if self.is_target_file(&file.filename) {
    //                     paths.insert(file.filename, file.status);
    //                 }
    //             }
    //         }

    //         if commits.next.is_some() {
    //             page += 1;
    //         } else {
    //             break;
    //         }
    //     }

    //     Ok(paths)
    // }

    pub async fn get_content(&self, path: &Path) -> Result<String> {
        let instant = Instant::now();
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            &self.source.owner, &self.source.repo, &self.source.branch, path,
        );
        let resp = reqwest::get(&url).await?;
        tracing::info!("Getting '{}', elapsed {:?}", url, instant.elapsed());
        match resp.status() {
            StatusCode::OK => match resp.text().await {
                Ok(text) => Ok(text),
                Err(e) => Err(anyhow!("unable to get body text; {}", e)),
            },
            _ => Err(anyhow!(
                "unable to get content from '{}', status is '{}'",
                url,
                resp.status()
            )),
        }
    }

    fn is_target_file(&self, path: &Path) -> bool {
        for dir in &self.source.allowed_dirs {
            if !path.starts_with(dir) {
                return false;
            }
        }

        for dir in &self.source.ignored_dirs {
            if path.starts_with(dir) {
                return false;
            }
        }

        if self.source.allowed_ext.len() > 0
            && !self
                .source
                .allowed_ext
                .iter()
                .any(|ext| path.ends_with(ext))
        {
            return false;
        }

        true
    }
}

// website/docs/r/xray_group.html.markdown
type Path = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub files: Vec<File>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub filename: Path,
    pub additions: i64,
    pub deletions: i64,
    pub changes: i64,
    pub status: FileStatus,
    pub raw_url: String,
    pub blob_url: String,
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Removed,
    Modified,
    Renamed,
    Copied,
    Changed,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeResponse {
    pub sha: String,
    pub url: String,
    pub tree: Vec<Tree>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub tree_type: TreeType,
    pub sha: String,
    pub size: Option<i64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeType {
    Blob,
    Tree,
}
