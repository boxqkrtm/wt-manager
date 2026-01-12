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
