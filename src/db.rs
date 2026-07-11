use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Database {
    pub projects: HashMap<String, ProjectInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectAutomationConfig {
    #[serde(default)]
    pub copy_files: Vec<String>,
    #[serde(default)]
    pub post_create_hooks: Vec<String>,
    #[serde(default)]
    pub post_cd_hooks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub path: PathBuf,
    pub name: String,
    pub last_accessed: u64,
    #[serde(default)]
    pub automation: Option<ProjectAutomationConfig>,
}

fn get_db_path() -> Result<PathBuf> {
    let messages = crate::i18n::Messages::new();
    let home = dirs::home_dir().context(messages.failed_get_home_dir().to_string())?;
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
    let messages = crate::i18n::Messages::new();
    let mut db = load_db()?;

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context(messages.invalid_repository_path().to_string())?
        .to_string();

    let key = repo_path.to_string_lossy().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let automation = if db.projects.contains_key(&key) {
        db.projects
            .get(&key)
            .and_then(|project| project.automation.clone())
    } else {
        default_automation_for_repo(repo_path)
    };

    db.projects.insert(
        key,
        ProjectInfo {
            path: repo_path.to_path_buf(),
            name: repo_name,
            last_accessed: now,
            automation,
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

fn ensure_project_entry(db: &mut Database, repo_path: &Path) -> Result<String> {
    let messages = crate::i18n::Messages::new();
    let key = repo_path.to_string_lossy().to_string();

    if !db.projects.contains_key(&key) {
        let repo_name = repo_path
            .file_name()
            .and_then(|name| name.to_str())
            .context(messages.invalid_repository_path().to_string())?
            .to_string();

        db.projects.insert(
            key.clone(),
            ProjectInfo {
                path: repo_path.to_path_buf(),
                name: repo_name,
                last_accessed: 0,
                automation: default_automation_for_repo(repo_path),
            },
        );
    }

    Ok(key)
}

#[cfg(not(windows))]
const DEFAULT_POSIX_PROFILE_SOURCE_PREFIX: &str =
    "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; ";

fn default_runtime_hook(command: &str) -> String {
    #[cfg(windows)]
    {
        command.to_string()
    }

    #[cfg(not(windows))]
    {
        format!("{DEFAULT_POSIX_PROFILE_SOURCE_PREFIX}{command}")
    }
}

fn default_automation_for_repo(repo_path: &Path) -> Option<ProjectAutomationConfig> {
    let mut post_create_hooks = Vec::new();

    if repo_path.join("mise.toml").exists() || repo_path.join(".mise.toml").exists() {
        post_create_hooks.push(default_runtime_hook("mise install"));
    } else if repo_path.join(".nvmrc").exists() {
        post_create_hooks.push(default_runtime_hook("nvm use"));
    }

    if repo_path.join("pnpm-lock.yaml").exists() {
        post_create_hooks.push("pnpm install".to_string());
    } else if repo_path.join("yarn.lock").exists() {
        post_create_hooks.push("yarn install".to_string());
    } else if repo_path.join("package-lock.json").exists() {
        post_create_hooks.push("npm install".to_string());
    }

    if post_create_hooks.is_empty() {
        None
    } else {
        Some(ProjectAutomationConfig {
            copy_files: Vec::new(),
            post_create_hooks,
            post_cd_hooks: Vec::new(),
        })
    }
}

fn is_safe_repo_relative_path(path: &str) -> bool {
    let candidate = Path::new(path);
    !candidate.is_absolute()
        && candidate.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

pub fn get_project_automation(repo_path: &Path) -> Result<Option<ProjectAutomationConfig>> {
    let db = load_db()?;
    let key = repo_path.to_string_lossy().to_string();

    Ok(db
        .projects
        .get(&key)
        .and_then(|project| project.automation.clone()))
}

pub fn add_copy_file(repo_path: &Path, path: &str) -> Result<()> {
    if !is_safe_repo_relative_path(path) {
        anyhow::bail!(
            "Copy path must be a repo-root relative path without '..': {}",
            path
        );
    }

    let mut db = load_db()?;
    let key = ensure_project_entry(&mut db, repo_path)?;
    let project = db
        .projects
        .get_mut(&key)
        .expect("project entry should exist");
    let automation = project
        .automation
        .get_or_insert_with(ProjectAutomationConfig::default);

    if !automation.copy_files.iter().any(|entry| entry == path) {
        automation.copy_files.push(path.to_string());
    }

    save_db(&db)?;
    Ok(())
}

pub fn remove_copy_file(repo_path: &Path, path: &str) -> Result<bool> {
    let mut db = load_db()?;
    let key = ensure_project_entry(&mut db, repo_path)?;
    let project = db
        .projects
        .get_mut(&key)
        .expect("project entry should exist");
    let Some(automation) = project.automation.as_mut() else {
        return Ok(false);
    };
    let original_len = automation.copy_files.len();
    automation.copy_files.retain(|entry| entry != path);
    let removed = automation.copy_files.len() != original_len;
    save_db(&db)?;
    Ok(removed)
}

pub fn add_hook(repo_path: &Path, hook_kind: &str, command: &str) -> Result<()> {
    let mut db = load_db()?;
    let key = ensure_project_entry(&mut db, repo_path)?;
    let project = db
        .projects
        .get_mut(&key)
        .expect("project entry should exist");
    let automation = project
        .automation
        .get_or_insert_with(ProjectAutomationConfig::default);

    let hooks = match hook_kind {
        "post-create" => &mut automation.post_create_hooks,
        "post-cd" => &mut automation.post_cd_hooks,
        _ => anyhow::bail!("Unsupported hook kind: {}", hook_kind),
    };

    hooks.push(command.to_string());
    save_db(&db)?;
    Ok(())
}

pub fn remove_hook(repo_path: &Path, hook_kind: &str, index: usize) -> Result<bool> {
    let mut db = load_db()?;
    let key = ensure_project_entry(&mut db, repo_path)?;
    let project = db
        .projects
        .get_mut(&key)
        .expect("project entry should exist");
    let Some(automation) = project.automation.as_mut() else {
        return Ok(false);
    };

    let hooks = match hook_kind {
        "post-create" => &mut automation.post_create_hooks,
        "post-cd" => &mut automation.post_cd_hooks,
        _ => anyhow::bail!("Unsupported hook kind: {}", hook_kind),
    };

    if index >= hooks.len() {
        return Ok(false);
    }

    hooks.remove(index);
    save_db(&db)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::default_automation_for_repo;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wt-manager-db-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_automation_uses_platform_commands_for_mise_and_pnpm() {
        let repo = make_temp_dir("mise-pnpm");
        fs::write(repo.join("mise.toml"), "").unwrap();
        fs::write(repo.join("pnpm-lock.yaml"), "").unwrap();

        let automation = default_automation_for_repo(&repo).unwrap();

        #[cfg(windows)]
        assert_eq!(
            automation.post_create_hooks,
            ["mise install", "pnpm install"]
        );
        #[cfg(not(windows))]
        assert_eq!(
            automation.post_create_hooks,
            [
                "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; mise install",
                "pnpm install",
            ]
        );

        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn default_automation_uses_platform_commands_for_nvm_and_npm() {
        let repo = make_temp_dir("nvm-npm");
        fs::write(repo.join(".nvmrc"), "").unwrap();
        fs::write(repo.join("package-lock.json"), "").unwrap();

        let automation = default_automation_for_repo(&repo).unwrap();

        #[cfg(windows)]
        assert_eq!(automation.post_create_hooks, ["nvm use", "npm install"]);
        #[cfg(not(windows))]
        assert_eq!(
            automation.post_create_hooks,
            [
                "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; nvm use",
                "npm install",
            ]
        );

        fs::remove_dir_all(repo).unwrap();
    }
}
