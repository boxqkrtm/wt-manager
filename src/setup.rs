use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub struct SetupManager;

impl SetupManager {
    /// Run automatic setup based on project files (mise, nvm, pnpm, yarn, npm)
    pub fn run_auto_setup(worktree_path: &Path) -> Result<()> {
        let mut commands = Vec::new();
        let mut shell_cmd = String::new();

        // 1. Environment Setup (mise or nvm)
        if worktree_path.join("mise.toml").exists() || worktree_path.join(".mise.toml").exists() {
            commands.push("mise install");
        } else if worktree_path.join(".nvmrc").exists() {
            commands.push("nvm use");
        }

        // 2. Package Manager Setup
        if worktree_path.join("pnpm-lock.yaml").exists() {
            commands.push("pnpm install");
        } else if worktree_path.join("yarn.lock").exists() {
            commands.push("yarn install");
        } else if worktree_path.join("package-lock.json").exists() {
            commands.push("npm install");
        }

        if commands.is_empty() {
            return Ok(());
        }

        // 3. Command Execution via Shell
        // Source shell config to ensure version managers are available
        if commands.iter().any(|&c| c == "nvm use" || c == "mise install") {
            shell_cmd.push_str("source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; ");
        }
        
        shell_cmd.push_str(&commands.join(" && "));
        
        println!("Running automatic setup: {}", shell_cmd);

        let output = Command::new("zsh")
            .arg("-c")
            .arg(&shell_cmd)
            .current_dir(worktree_path)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("✓ Setup completed successfully");
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("Warning: Setup completed with issues.");
                if !stdout.trim().is_empty() {
                    println!("Output: {}", stdout);
                }
                if !stderr.trim().is_empty() {
                    eprintln!("Error output: {}", stderr);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Warning: Could not run setup command: {}", e);
                Ok(())
            }
        }
    }
}
