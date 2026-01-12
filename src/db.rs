use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Database {
    pub projects: HashMap<String, ProjectInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub path: PathBuf,
    pub name: String,
    pub last_accessed: u64,
}

fn get_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    let db_dir = home.join(".wt-manager");
    fs::create_dir_all(&db_dir)?;
    Ok(db_dir.join("db.json"))
}

pub fn load_db() -> Result<Database> {
    let db_path = get_db_path()?;
    
    if !db_path.exists() {
        return Ok(Database::default());
    }

    let content = fs::read_to_string(&db_path)?;
    let db: Database = serde_json::from_str(&content)?;
    Ok(db)
}

pub fn save_db(db: &Database) -> Result<()> {
    let db_path = get_db_path()?;
    let content = serde_json::to_string_pretty(db)?;
    fs::write(&db_path, content)?;
    Ok(())
}

pub fn save_project(repo_path: &Path) -> Result<()> {
    let mut db = load_db()?;
    
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid repository path")?
        .to_string();

    let key = repo_path.to_string_lossy().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    db.projects.insert(
        key,
        ProjectInfo {
            path: repo_path.to_path_buf(),
            name: repo_name,
            last_accessed: now,
        },
    );

    save_db(&db)?;
    Ok(())
}

pub fn get_projects() -> Result<Vec<ProjectInfo>> {
    let db = load_db()?;
    let mut projects: Vec<ProjectInfo> = db.projects.values().cloned().collect();
    
    // Sort by last accessed (most recent first)
    projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
    
    Ok(projects)
}

pub fn update_last_accessed(repo_path: &Path) -> Result<()> {
    let mut db = load_db()?;
    let key = repo_path.to_string_lossy().to_string();
    
    if let Some(project) = db.projects.get_mut(&key) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        project.last_accessed = now;
        save_db(&db)?;
    }
    
    Ok(())
}
