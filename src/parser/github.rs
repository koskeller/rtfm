use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use octocrab::Octocrab;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tiktoken_rs::CoreBPE;
use tokio::time::Instant;

use crate::types::Document;

pub struct GitHub {
    client: Octocrab,
    tokenizer: CoreBPE,
    owner: String,
    branch: String,
    repo: String,
    // Without dot: ["md", "markdown"]
    allowed_ext: HashSet<String>,
    // Without leading slash: ["website", "website/docs"]
    allowed_dirs: HashSet<String>,
    // Without leading slash: ["website/cdtk"]
    ignored_dirs: HashSet<String>,
}

impl GitHub {
    pub fn new(client: &Octocrab, owner: &str, repo: &str, branch: &str) -> GitHubBuilder {
        GitHubBuilder::new(client, owner, repo, branch)
    }

    pub async fn get_documents(&self) -> anyhow::Result<Vec<Document>> {
        let mut documents = Vec::new();
        let paths = self.get_paths().await?;
        let paths = self.filter_paths(paths);
        for path in paths {
            let source_id = format!("github.com/{}/{}", self.owner, self.repo);
            let blob = self.get_content(&path).await?;
            let created_at = Utc::now();
            let updated_at = Utc::now();
            let checksum = calculate_checksum(&blob);
            let tokens = self.token_len(&blob);
            documents.push(Document {
                source_id,
                path,
                checksum,
                tokens,
                blob,
                created_at,
                updated_at,
            });
        }
        Ok(documents)
    }

    async fn get_paths(&self) -> Result<Vec<Path>> {
        let route = format!(
            "/repos/{}/{}/git/trees/{}?recursive='true'",
            &self.owner, &self.repo, &self.branch
        );
        println!("Getting tree {}", route);
        let resp: TreeResponse = self.client.get(route, None::<&()>).await?;

        let paths: Vec<Path> = resp
            .tree
            .into_iter()
            .filter_map(|file| match file.tree_type {
                TreeType::Blob if self.is_target_file(&file.path) => Some(file.path),
                _ => None,
            })
            .collect();

        println!("Got {} paths", paths.len());
        Ok(paths)
    }

    fn filter_paths(&self, files: Vec<Path>) -> Vec<Path> {
        files
            .into_iter()
            .filter(|x| self.is_target_file(x))
            .collect()
    }

    pub async fn get_changed_files(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<Path, FileStatus>> {
        let repository = self.client.repos(&self.owner, &self.repo);

        let mut paths: HashMap<Path, FileStatus> = HashMap::new();
        let mut page: u32 = 1;
        loop {
            let commits = repository
                .list_commits()
                .since(since)
                .per_page(100)
                .page(page)
                .send()
                .await?;

            for commit in commits.items {
                let route = format!("/repos/{}/{}/commits/{}", self.owner, self.repo, commit.sha);
                let commit: Commit = self.client.get(route, None::<&()>).await?;
                for file in commit.files {
                    if self.is_target_file(&file.filename) {
                        paths.insert(file.filename, file.status);
                    }
                }
            }

            if commits.next.is_some() {
                page += 1;
            } else {
                break;
            }
        }

        Ok(paths)
    }

    pub async fn get_content(&self, path: &Path) -> Result<String> {
        let instant = Instant::now();
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            &self.owner, &self.repo, &self.branch, path,
        );
        let resp = reqwest::get(&url).await?;
        println!("getting {} took {:?}", url, instant.elapsed());
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
        for dir in &self.allowed_dirs {
            if !path.starts_with(dir) {
                return false;
            }
        }

        for dir in &self.ignored_dirs {
            if path.starts_with(dir) {
                return false;
            }
        }

        if self.allowed_ext.len() > 0 && !self.allowed_ext.iter().any(|ext| path.ends_with(ext)) {
            return false;
        }

        true
    }

    fn token_len(&self, s: &str) -> usize {
        let instant = Instant::now();
        let len = self.tokenizer.encode_with_special_tokens(&s).len();
        println!("tokenizing took {:?}", instant.elapsed());
        len
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

fn calculate_checksum(s: &str) -> u32 {
    crc32fast::hash(s.as_bytes())
}

pub struct GitHubBuilder {
    client: Octocrab,
    tokenizer: CoreBPE,
    owner: String,
    repo: String,
    branch: String,
    allowed_ext: HashSet<String>,
    allowed_dirs: HashSet<String>,
    ignored_dirs: HashSet<String>,
}

impl GitHubBuilder {
    fn new(client: &Octocrab, owner: &str, repo: &str, branch: &str) -> Self {
        GitHubBuilder {
            client: client.clone(),
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            branch: branch.to_owned(),
            allowed_ext: HashSet::new(),
            allowed_dirs: HashSet::new(),
            ignored_dirs: HashSet::new(),
            tokenizer: tiktoken_rs::cl100k_base().expect("Failed to instantiate tokenizer"),
        }
    }

    /// Without dot: "md", "markdown"
    pub fn allowed_ext(mut self, allowed_ext: HashSet<String>) -> Self {
        self.allowed_ext = allowed_ext;
        self
    }

    /// Without leading slash: "website", "website/docs"
    pub fn allowed_dirs(mut self, allowed_dirs: HashSet<String>) -> Self {
        self.allowed_dirs = allowed_dirs;
        self
    }

    /// Without leading slash: "website/cdtk"
    pub fn ignored_dirs(mut self, ignored_dirs: HashSet<String>) -> Self {
        self.ignored_dirs = ignored_dirs;
        self
    }

    pub fn build(self) -> GitHub {
        GitHub {
            client: self.client,
            tokenizer: self.tokenizer,
            owner: self.owner,
            branch: self.branch,
            repo: self.repo,
            allowed_ext: self.allowed_ext,
            allowed_dirs: self.allowed_dirs,
            ignored_dirs: self.ignored_dirs,
        }
    }
}
