use std::env;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    English,
    Korean,
}

impl Language {
    pub fn detect() -> Self {
        // Check LANG environment variable
        if let Ok(lang) = env::var("LANG") {
            if lang.starts_with("ko") {
                return Language::Korean;
            }
        }

        // Default to English
        Language::English
    }
}

pub struct Messages {
    lang: Language,
}

impl Messages {
    pub fn new() -> Self {
        Self {
            lang: Language::detect(),
        }
    }

    pub fn with_language(lang: Language) -> Self {
        Self { lang }
    }

    // Project selector
    pub fn select_project(&self) -> &str {
        match self.lang {
            Language::English => "Select Project",
            Language::Korean => "프로젝트 선택",
        }
    }

    pub fn no_projects_found(&self) -> &str {
        match self.lang {
            Language::English => "No projects found in database.",
            Language::Korean => "데이터베이스에 프로젝트가 없습니다.",
        }
    }

    pub fn navigate_to_git_repo(&self) -> &str {
        match self.lang {
            Language::English => "Navigate to a git repository and run 'wt' to add it.",
            Language::Korean => "git 저장소로 이동한 후 'wt'를 실행하여 추가하세요.",
        }
    }

    // Worktree selector
    pub fn select_or_create_worktree(&self) -> &str {
        match self.lang {
            Language::English => "Select or Create Worktree",
            Language::Korean => "워크트리 선택 또는 생성",
        }
    }

    pub fn switching_to_project(&self) -> &str {
        match self.lang {
            Language::English => "✓ Switching to project:",
            Language::Korean => "✓ 프로젝트로 전환:",
        }
    }

    pub fn switching_to_worktree(&self) -> &str {
        match self.lang {
            Language::English => "✓ Switching to worktree:",
            Language::Korean => "✓ 워크트리로 전환:",
        }
    }

    pub fn creating_new_worktree(&self) -> &str {
        match self.lang {
            Language::English => "✓ Creating new worktree:",
            Language::Korean => "✓ 새 워크트리 생성:",
        }
    }

    pub fn deleting_worktree(&self) -> &str {
        match self.lang {
            Language::English => "🗑️  Deleting worktree:",
            Language::Korean => "🗑️  워크트리 삭제:",
        }
    }

    pub fn worktree_deleted(&self) -> &str {
        match self.lang {
            Language::English => "✓ Worktree '{}' deleted successfully",
            Language::Korean => "✓ 워크트리 '{}'가 성공적으로 삭제되었습니다",
        }
    }

    pub fn cannot_delete_main(&self) -> &str {
        match self.lang {
            Language::English => "✗ Cannot delete main worktree",
            Language::Korean => "✗ 메인 워크트리는 삭제할 수 없습니다",
        }
    }

    pub fn failed_to_delete(&self) -> &str {
        match self.lang {
            Language::English => "✗ Failed to delete worktree:",
            Language::Korean => "✗ 워크트리 삭제 실패:",
        }
    }

    pub fn uncommitted_changes_tip(&self) -> &str {
        match self.lang {
            Language::English => "💡 Tip: The worktree may have uncommitted changes.",
            Language::Korean => "💡 팁: 워크트리에 커밋되지 않은 변경사항이 있을 수 있습니다.",
        }
    }

    pub fn force_delete_command(&self) -> &str {
        match self.lang {
            Language::English => "   To force delete, run:",
            Language::Korean => "   강제 삭제하려면 다음 명령을 실행하세요:",
        }
    }

    pub fn deps_installed(&self) -> &str {
        match self.lang {
            Language::English => "✓ Dependencies installed successfully",
            Language::Korean => "✓ 의존성이 성공적으로 설치되었습니다",
        }
    }

    pub fn pnpm_install_warning(&self) -> &str {
        match self.lang {
            Language::English => "Warning: Could not run pnpm install",
            Language::Korean => "경고: pnpm install을 실행할 수 없습니다",
        }
    }

    pub fn running_setup(&self) -> &str {
        match self.lang {
            Language::English => "Running automatic setup",
            Language::Korean => "자동 설정 실행 중",
        }
    }

    pub fn setup_completed_with_issues(&self) -> &str {
        match self.lang {
            Language::English => "Warning: Setup completed with issues.",
            Language::Korean => "경고: 설정 실행 중 문제가 있었습니다.",
        }
    }

    pub fn setup_completed(&self) -> &str {
        match self.lang {
            Language::English => "✓ Setup completed successfully",
            Language::Korean => "✓ 설정이 성공적으로 완료되었습니다",
        }
    }

    pub fn output_label(&self) -> &str {
        match self.lang {
            Language::English => "Output",
            Language::Korean => "출력",
        }
    }

    pub fn error_output_label(&self) -> &str {
        match self.lang {
            Language::English => "Error output",
            Language::Korean => "에러 출력",
        }
    }

    pub fn setup_command_error(&self) -> &str {
        match self.lang {
            Language::English => "Warning: Could not run setup command",
            Language::Korean => "경고: 설정 명령어를 실행할 수 없습니다",
        }
    }

    // TUI help text
    pub fn help_search(&self) -> &str {
        match self.lang {
            Language::English => "Type to search",
            Language::Korean => "검색어 입력",
        }
    }

    pub fn help_tab(&self) -> &str {
        match self.lang {
            Language::English => "Tab: Autocomplete",
            Language::Korean => "Tab: 자동완성",
        }
    }

    pub fn help_enter_select(&self) -> &str {
        match self.lang {
            Language::English => "Enter: Select",
            Language::Korean => "Enter: 선택",
        }
    }

    pub fn help_ctrl_b_create(&self) -> &str {
        match self.lang {
            Language::English => "Ctrl+B: Create",
            Language::Korean => "Ctrl+B: 생성",
        }
    }

    pub fn help_ctrl_x_delete(&self) -> &str {
        match self.lang {
            Language::English => "Ctrl+X: Delete",
            Language::Korean => "Ctrl+X: 삭제",
        }
    }

    pub fn create_new_prefix(&self) -> &str {
        match self.lang {
            Language::English => "→ Create new: '{}'",
            Language::Korean => "→ 새 항목 생성: '{}'",
        }
    }

    pub fn matches_label(&self) -> &str {
        match self.lang {
            Language::English => "Matches",
            Language::Korean => "매칭 항목",
        }
    }

    pub fn loading_worktrees(&self) -> &str {
        match self.lang {
            Language::English => "Loading worktrees...",
            Language::Korean => "워크트리 불러오는 중...",
        }
    }

    pub fn loading_pull_requests(&self) -> &str {
        match self.lang {
            Language::English => "Loading PR previews...",
            Language::Korean => "PR 미리보기 불러오는 중...",
        }
    }

    pub fn pr_preview_unavailable(&self) -> &str {
        match self.lang {
            Language::English => "PR preview unavailable",
            Language::Korean => "PR 미리보기를 불러오지 못했습니다",
        }
    }

    pub fn worktree_ready(&self) -> &str {
        match self.lang {
            Language::English => "✓ Worktree ready at:",
            Language::Korean => "✓ 워크트리 준비 완료:",
        }
    }

    pub fn switch_to_worktree_guide(&self) -> &str {
        match self.lang {
            Language::English => "To switch to this worktree, run:",
            Language::Korean => "이 워크트리로 전환하려면 다음을 실행하세요:",
        }
    }

    pub fn worktree_already_exists(&self) -> &str {
        match self.lang {
            Language::English => "Worktree already exists for branch '{}'",
            Language::Korean => "브랜치 '{}'에 대한 워크트리가 이미 존재합니다",
        }
    }

    pub fn adding_worktree_for_branch(&self) -> &str {
        match self.lang {
            Language::English => "Adding worktree for branch '{}'",
            Language::Korean => "브랜치 '{}'의 워크트리 추가 중",
        }
    }

    pub fn worktree_added_existing_branch(&self) -> &str {
        match self.lang {
            Language::English => "✓ Worktree added for existing branch '{}'",
            Language::Korean => "✓ 기존 브랜치 '{}'의 워크트리가 추가됨",
        }
    }

    pub fn branch_not_found_create_new(&self) -> &str {
        match self.lang {
            Language::English => "Branch '{}' not found, creating new branch",
            Language::Korean => "브랜치 '{}'를 찾을 수 없어 새 브랜치를 생성합니다",
        }
    }

    pub fn created_new_branch_with_worktree(&self) -> &str {
        match self.lang {
            Language::English => "✓ Created new branch '{}' with worktree",
            Language::Korean => "✓ 새 브랜치 '{}'와 워크트리가 생성됨",
        }
    }

    pub fn cannot_find_worktree(&self) -> &str {
        match self.lang {
            Language::English => "Worktree not found: {}",
            Language::Korean => "워크트리를 찾을 수 없습니다: {}",
        }
    }

    pub fn no_stale_worktrees_found(&self) -> &str {
        match self.lang {
            Language::English => "No stale worktrees found.",
            Language::Korean => "삭제 대상이 된 스테일 워크트리가 없습니다.",
        }
    }

    pub fn found_stale_worktrees(&self) -> &str {
        match self.lang {
            Language::English => "🧹 Found {} stale worktree(s):",
            Language::Korean => "🧹 {}개의 삭제 대상 스테일 워크트리를 찾았습니다:",
        }
    }

    pub fn stale_upstream_missing_reason(&self) -> &str {
        match self.lang {
            Language::English => "upstream '{}' missing",
            Language::Korean => "추적 브랜치 '{}'가 없음",
        }
    }

    pub fn no_upstream_reason(&self) -> &str {
        match self.lang {
            Language::English => "no upstream",
            Language::Korean => "upstream 없음",
        }
    }

    pub fn dry_run_enabled(&self) -> &str {
        match self.lang {
            Language::English => "Dry run enabled. No worktrees were deleted.",
            Language::Korean => "드라이 런 모드입니다. 워크트리를 삭제하지 않습니다.",
        }
    }

    pub fn path_label(&self) -> &str {
        match self.lang {
            Language::English => "path",
            Language::Korean => "경로",
        }
    }

    pub fn main_marker(&self) -> &str {
        match self.lang {
            Language::English => " (main)",
            Language::Korean => " (메인)",
        }
    }

    pub fn saved_projects_title(&self) -> &str {
        match self.lang {
            Language::English => "Saved Projects:",
            Language::Korean => "저장된 프로젝트:",
        }
    }

    pub fn list_items_item_path(&self, name: &str, path: &str) -> String {
        match self.lang {
            Language::English => format!("{} ({})", name, path),
            Language::Korean => format!("{} ({})", name, path),
        }
    }

    pub fn cmd_requires_repo(&self) -> &str {
        match self.lang {
            Language::English => "This command requires a git repository. Run `wt` in a repository or with a project selected.",
            Language::Korean => "이 명령은 Git 저장소가 필요합니다. 저장소에서 `wt`를 실행하거나 프로젝트를 선택해 주세요.",
        }
    }

    pub fn ctrlc_handler_error(&self) -> &str {
        match self.lang {
            Language::English => "Error setting Ctrl+C handler",
            Language::Korean => "Ctrl+C 핸들러 설정 중 오류가 발생했습니다",
        }
    }

    pub fn failed_get_home_dir(&self) -> &str {
        match self.lang {
            Language::English => "Failed to get home directory",
            Language::Korean => "홈 디렉터리 조회에 실패했습니다",
        }
    }

    pub fn invalid_repository_path(&self) -> &str {
        match self.lang {
            Language::English => "Invalid repository path",
            Language::Korean => "유효하지 않은 저장소 경로",
        }
    }

    pub fn failed_list_worktrees(&self) -> &str {
        match self.lang {
            Language::English => "Failed to list worktrees",
            Language::Korean => "워크트리 목록을 가져오지 못했습니다",
        }
    }

    pub fn failed_add_worktree(&self) -> &str {
        match self.lang {
            Language::English => "Failed to add worktree",
            Language::Korean => "워크트리 추가에 실패했습니다",
        }
    }

    pub fn failed_remove_worktree(&self) -> &str {
        match self.lang {
            Language::English => "Failed to remove worktree",
            Language::Korean => "워크트리 삭제에 실패했습니다",
        }
    }

    pub fn failed_create_worktree_context(&self) -> &str {
        match self.lang {
            Language::English => "Failed to create new branch and worktree",
            Language::Korean => "새 브랜치와 워크트리 생성에 실패했습니다",
        }
    }

    pub fn failed_collect_upstream(&self) -> &str {
        match self.lang {
            Language::English => "Failed to collect branch upstream information",
            Language::Korean => "브랜치의 upstream 정보를 수집하지 못했습니다",
        }
    }

    pub fn failed_verify_remote_ref(&self) -> &str {
        match self.lang {
            Language::English => "Failed to verify remote ref 'refs/remotes/{}'",
            Language::Korean => "원격 ref 'refs/remotes/{}' 확인에 실패했습니다",
        }
    }

    pub fn failed_verify_remote_ref_interrupted(&self) -> &str {
        match self.lang {
            Language::English => {
                "Failed to verify remote ref 'refs/remotes/{}' due to interrupted git process"
            }
            Language::Korean => {
                "git 프로세스 중단으로 인해 원격 ref 'refs/remotes/{}' 확인에 실패했습니다"
            }
        }
    }

    pub fn failed_fetch_prune(&self) -> &str {
        match self.lang {
            Language::English => "fetch --prune failed: {}",
            Language::Korean => "fetch --prune 실패: {}",
        }
    }

    pub fn help_cancel(&self) -> &str {
        match self.lang {
            Language::English => "Ctrl+C/Esc: Cancel",
            Language::Korean => "Ctrl+C/Esc: 취소",
        }
    }

    pub fn help_backspace(&self) -> &str {
        match self.lang {
            Language::English => "Backspace: Edit",
            Language::Korean => "Backspace: 편집",
        }
    }

    pub fn help_create_new_branch(&self) -> &str {
        match self.lang {
            Language::English => "Ctrl+B: Create new branch",
            Language::Korean => "Ctrl+B: 새 브랜치 생성",
        }
    }

    pub fn help_exact_match(&self) -> &str {
        match self.lang {
            Language::English => "(exact match)",
            Language::Korean => "(정확히 일치)",
        }
    }

    // Config command messages
    pub fn config_show_header(&self) -> &str {
        match self.lang {
            Language::English => "Repo config for",
            Language::Korean => "저장소 설정:",
        }
    }

    pub fn config_copy_files_label(&self) -> &str {
        match self.lang {
            Language::English => "copy_files:",
            Language::Korean => "복사 파일 목록:",
        }
    }

    pub fn config_post_create_hooks_label(&self) -> &str {
        match self.lang {
            Language::English => "post_create_hooks:",
            Language::Korean => "워크트리 생성 후 훅:",
        }
    }

    pub fn config_post_cd_hooks_label(&self) -> &str {
        match self.lang {
            Language::English => "post_cd_hooks:",
            Language::Korean => "워크트리 전환 후 훅:",
        }
    }

    pub fn config_no_config_found(&self) -> &str {
        match self.lang {
            Language::English => "No repo-specific wt config found.",
            Language::Korean => "저장소별 wt 설정이 없습니다.",
        }
    }

    pub fn config_added_copy_file(&self) -> &str {
        match self.lang {
            Language::English => "Added copy file '{}'",
            Language::Korean => "복사 파일 '{}'이(가) 추가되었습니다",
        }
    }

    pub fn config_removed_copy_file(&self) -> &str {
        match self.lang {
            Language::English => "Removed copy file '{}'",
            Language::Korean => "복사 파일 '{}'이(가) 삭제되었습니다",
        }
    }

    pub fn config_copy_file_not_configured(&self) -> &str {
        match self.lang {
            Language::English => "Copy file '{}' was not configured",
            Language::Korean => "복사 파일 '{}'은(는) 설정되어 있지 않습니다",
        }
    }

    pub fn config_added_hook(&self) -> &str {
        match self.lang {
            Language::English => "Added {} hook '{}'",
            Language::Korean => "{} 훅 '{}'이(가) 추가되었습니다",
        }
    }

    pub fn config_removed_hook_at_index(&self) -> &str {
        match self.lang {
            Language::English => "Removed {} hook at index {}",
            Language::Korean => "{} 훅 인덱스 {}이(가) 삭제되었습니다",
        }
    }

    pub fn config_no_hook_found_at_index(&self) -> &str {
        match self.lang {
            Language::English => "No {} hook found at index {}",
            Language::Korean => "{} 훅이 인덱스 {}에 없습니다",
        }
    }

    // Merged worktrees messages
    pub fn merged_into(&self) -> &str {
        match self.lang {
            Language::English => "merged into '{}'",
            Language::Korean => "'{}'에 머지됨",
        }
    }

    pub fn no_merged_worktrees_found(&self) -> &str {
        match self.lang {
            Language::English => "No merged worktrees found.",
            Language::Korean => "머지된 워크트리가 없습니다.",
        }
    }

    pub fn found_merged_worktrees(&self) -> &str {
        match self.lang {
            Language::English => "Found {} merged worktree(s):",
            Language::Korean => "{}개의 머지된 워크트리를 찾았습니다:",
        }
    }

    // Shell integration init messages
    pub fn init_already_configured(&self) -> &str {
        match self.lang {
            Language::English => "Already configured {}",
            Language::Korean => "이미 설정되어 있습니다: {}",
        }
    }

    pub fn init_updated(&self) -> &str {
        match self.lang {
            Language::English => "Updated {}",
            Language::Korean => "업데이트됨: {}",
        }
    }

    pub fn init_generated(&self) -> &str {
        match self.lang {
            Language::English => "Generated {}",
            Language::Korean => "생성됨: {}",
        }
    }

    pub fn init_already_set_up(&self) -> &str {
        match self.lang {
            Language::English => "Shell integration was already configured.",
            Language::Korean => "셸 통합이 이미 설정되어 있습니다.",
        }
    }

    // --help / long_about text
    pub fn long_help(&self) -> &'static str {
        match self.lang {
            Language::English => LONG_HELP_EN,
            Language::Korean => LONG_HELP_KO,
        }
    }

    pub fn tui_help(&self) -> &'static str {
        match self.lang {
            Language::English => TUI_HELP_EN,
            Language::Korean => TUI_HELP_KO,
        }
    }

    pub fn cmd_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Advanced git worktree manager",
            Language::Korean => "고급 git 워크트리 관리 도구",
        }
    }

    // Subcommand about text
    pub fn cmd_init_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Initialize shell integration",
            Language::Korean => "셸 통합 설치",
        }
    }

    pub fn cmd_tui_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Open interactive TUI",
            Language::Korean => "인터랙티브 TUI 열기",
        }
    }

    pub fn cmd_list_about(&self) -> &'static str {
        match self.lang {
            Language::English => "List all worktrees in this repository",
            Language::Korean => "이 저장소의 모든 워크트리 목록 출력",
        }
    }

    pub fn cmd_cd_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Switch to existing worktree or create one if branch does not exist",
            Language::Korean => "기존 워크트리로 전환하거나 브랜치가 없으면 새로 생성",
        }
    }

    pub fn cmd_run_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Run a command inside a worktree",
            Language::Korean => "워크트리 내에서 명령어 실행",
        }
    }

    pub fn cmd_delete_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Delete a worktree",
            Language::Korean => "워크트리 삭제",
        }
    }

    pub fn cmd_clean_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Remove worktrees whose tracking branches were deleted from remote or already merged",
            Language::Korean => "원격에서 삭제되었거나 이미 머지된 추적 브랜치를 가진 워크트리 삭제",
        }
    }

    pub fn cmd_project_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Manage saved projects",
            Language::Korean => "저장된 프로젝트 관리",
        }
    }

    pub fn cmd_project_list_about(&self) -> &'static str {
        match self.lang {
            Language::English => "List saved projects by last accessed order",
            Language::Korean => "마지막 접근 순으로 저장된 프로젝트 목록 출력",
        }
    }

    pub fn cmd_config_about(&self) -> &'static str {
        match self.lang {
            Language::English => "Manage wt configuration",
            Language::Korean => "wt 설정 관리",
        }
    }

    // Arg help text
    pub fn arg_branch_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Legacy mode: switch or create worktree for BRANCH",
            Language::Korean => "레거시 모드: BRANCH에 대한 워크트리 전환 또는 생성",
        }
    }

    pub fn arg_force_delete_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Force delete (use when normal delete fails due to uncommitted changes).",
            Language::Korean => "강제 삭제 (미커밋 변경사항으로 인해 일반 삭제가 실패할 때 사용).",
        }
    }

    pub fn arg_dry_run_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Do not delete, only list removable worktrees",
            Language::Korean => "삭제하지 않고 삭제 가능한 워크트리 목록만 출력",
        }
    }

    pub fn arg_force_clean_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Force delete worktrees even when they have local changes",
            Language::Korean => "로컬 변경사항이 있어도 워크트리 강제 삭제",
        }
    }

    pub fn arg_include_untracked_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Include worktrees whose tracking branch does not exist on remote",
            Language::Korean => "원격에 추적 브랜치가 없는 워크트리도 포함",
        }
    }

    pub fn arg_remote_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Optional remote name to check. If not set, checks all upstream remotes.",
            Language::Korean => "확인할 원격 이름 (선택 사항). 설정하지 않으면 모든 upstream 원격을 확인합니다.",
        }
    }

    pub fn arg_skip_fetch_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Skip git fetch --prune before checking remote refs",
            Language::Korean => "원격 ref 확인 전 git fetch --prune을 건너뜁니다",
        }
    }

    pub fn arg_merged_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Remove worktrees whose branch is already merged into the base branch",
            Language::Korean => "베이스 브랜치에 이미 머지된 브랜치의 워크트리 삭제",
        }
    }

    pub fn arg_base_help(&self) -> &'static str {
        match self.lang {
            Language::English => "Base branch used with --merged",
            Language::Korean => "--merged에서 사용할 베이스 브랜치",
        }
    }
}

const LONG_HELP_EN: &str = r#"Advanced Git worktree manager for terminal users.

Usage:
  wt init                     # install shell integration into ~/.wt-manager.sh and your shell rc
  wt                          # interactive project/worktree selector (default)
  wt <branch>                 # legacy mode: create or switch worktree
  wt tui                      # explicitly open interactive TUI
  wt list                     # list worktrees (main repo)
  wt cd <branch>              # same as `wt <branch>`
  wt run <branch> -- <cmd...>
  wt clean                    # delete worktrees with deleted remote tracking branches
  wt clean --merged [--base origin/main]
  wt clean --remote origin --dry-run
  wt delete <branch> [--force] # delete a worktree
  wt project list             # list registered projects (recent first)
  wt config show
  wt config copy add .env.local
  wt config hook add post-create "pnpm install"

Branch behavior:
  wt <branch> / wt cd <branch>
  - Try existing branch first
  - If branch does not exist, create it automatically
  - Run configured postCreate/postCd hooks from ~/.wt-manager/db.json

Delete safety:
  - Main worktree is protected and cannot be removed
  - If deletion fails (e.g., uncommitted changes), retry with --force

Note:
  Actual directory change is performed by the generated shell integration (`~/.wt-manager.sh`)
  by parsing `cd` output from the wt command.

Examples:
  wt
  wt feature/login
  wt list
  wt run feature/login -- cargo test
  wt delete feature/login --force
  wt project list
"#;

const LONG_HELP_KO: &str = r#"터미널 사용자를 위한 고급 Git 워크트리 관리 도구.

사용법:
  wt init                     # 셸 통합 설치 (~/.wt-manager.sh 및 shell rc에 저장)
  wt                          # 인터랙티브 프로젝트/워크트리 선택기 (기본값)
  wt <브랜치>                 # 레거시 모드: 워크트리 생성 또는 전환
  wt tui                      # 인터랙티브 TUI 열기
  wt list                     # 워크트리 목록 출력 (메인 저장소)
  wt cd <브랜치>              # `wt <브랜치>`와 동일
  wt run <브랜치> -- <명령어...>
  wt clean                    # 원격에서 삭제된 추적 브랜치를 가진 워크트리 삭제
  wt clean --merged [--base origin/main]
  wt clean --remote origin --dry-run
  wt delete <브랜치> [--force] # 워크트리 삭제
  wt project list             # 저장된 프로젝트 목록 (최근 순)
  wt config show
  wt config copy add .env.local
  wt config hook add post-create "pnpm install"

브랜치 동작:
  wt <브랜치> / wt cd <브랜치>
  - 기존 브랜치를 먼저 찾습니다
  - 브랜치가 없으면 자동으로 생성합니다
  - ~/.wt-manager/db.json에 설정된 postCreate/postCd 훅을 실행합니다

삭제 안전장치:
  - 메인 워크트리는 보호되어 삭제할 수 없습니다
  - 삭제 실패 시 (예: 미커밋 변경사항) --force로 재시도하세요

참고:
  실제 디렉터리 변경은 생성된 셸 통합 (`~/.wt-manager.sh`)이
  wt 명령의 `cd` 출력을 파싱하여 수행합니다.

예시:
  wt
  wt feature/login
  wt list
  wt run feature/login -- cargo test
  wt delete feature/login --force
  wt project list
"#;

const TUI_HELP_EN: &str = r#"TUI mode keys:
  Type:        search/fuzzy input
  Tab:         autocomplete with top match
  Enter:       select
  Ctrl+B:      create (worktree mode only)
  Ctrl+X:      delete exact match (worktree mode only)
  Ctrl+C/Esc:  cancel

TUI is opened automatically when:
- no argument and current dir is a git repository -> worktree selector
- no argument and outside git repository -> project selector"#;

const TUI_HELP_KO: &str = r#"TUI 모드 키:
  입력:        검색/퍼지 입력
  Tab:         상위 매칭으로 자동완성
  Enter:       선택
  Ctrl+B:      생성 (워크트리 모드만)
  Ctrl+X:      정확히 일치하는 항목 삭제 (워크트리 모드만)
  Ctrl+C/Esc:  취소

TUI가 자동으로 열리는 경우:
- 인자 없이 실행하고 현재 디렉터리가 git 저장소인 경우 -> 워크트리 선택기
- 인자 없이 실행하고 git 저장소 외부인 경우 -> 프로젝트 선택기"#;

impl Default for Messages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, Messages};

    #[test]
    fn setup_related_messages_have_expected_english_text() {
        let messages = Messages::with_language(Language::English);

        assert_eq!(messages.deps_installed(), "✓ Dependencies installed successfully");
        assert_eq!(messages.pnpm_install_warning(), "Warning: Could not run pnpm install");
        assert_eq!(messages.running_setup(), "Running automatic setup");
        assert_eq!(messages.setup_completed_with_issues(), "Warning: Setup completed with issues.");
        assert_eq!(messages.setup_completed(), "✓ Setup completed successfully");
        assert_eq!(messages.output_label(), "Output");
        assert_eq!(messages.error_output_label(), "Error output");
        assert_eq!(messages.setup_command_error(), "Warning: Could not run setup command");
    }

    #[test]
    fn setup_related_messages_have_expected_korean_text() {
        let messages = Messages::with_language(Language::Korean);

        assert_eq!(messages.deps_installed(), "✓ 의존성이 성공적으로 설치되었습니다");
        assert_eq!(messages.pnpm_install_warning(), "경고: pnpm install을 실행할 수 없습니다");
        assert_eq!(messages.running_setup(), "자동 설정 실행 중");
        assert_eq!(messages.setup_completed_with_issues(), "경고: 설정 실행 중 문제가 있었습니다.");
        assert_eq!(messages.setup_completed(), "✓ 설정이 성공적으로 완료되었습니다");
        assert_eq!(messages.output_label(), "출력");
        assert_eq!(messages.error_output_label(), "에러 출력");
        assert_eq!(messages.setup_command_error(), "경고: 설정 명령어를 실행할 수 없습니다");
    }
}
