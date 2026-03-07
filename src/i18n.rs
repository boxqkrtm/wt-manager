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
            Language::English => "Failed to verify remote ref 'refs/remotes/{}' due to interrupted git process",
            Language::Korean => "git 프로세스 중단으로 인해 원격 ref 'refs/remotes/{}' 확인에 실패했습니다",
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
}

impl Default for Messages {
    fn default() -> Self {
        Self::new()
    }
}
