# wt-manager

## Quick Start

### macOS / Linux

zsh 또는 bash에서 설치합니다.

```bash
cargo install --git https://github.com/boxqkrtm/wt-manager.git
wt init

# 현재 사용하는 셸 설정을 다시 로드
source ~/.zshrc   # zsh
# source ~/.bashrc  # bash

wt
```

### Windows

PowerShell 5.1 또는 PowerShell 7에서 설치합니다.

```powershell
cargo install --git https://github.com/boxqkrtm/wt-manager.git
wt init --profile $PROFILE
. $PROFILE

wt
```

PowerShell 5.1과 PowerShell 7은 서로 다른 `$PROFILE`을 사용할 수 있으므로 현재 PowerShell의 값을 그대로 전달해야 합니다. Windows의 `cmd.exe` 셸 통합은 지원하지 않습니다.

---

Git worktree 관리 도구 - TUI 기반 워크트리 생성/전환/삭제

![alt preview image](./screenshot.png)

## 요구사항

- **Git**: Git worktree 명령을 지원하는 Git. Windows에서는 Git for Windows 사용
- **셸**:
  - macOS/Linux: zsh 또는 bash
  - Windows: PowerShell 5.1 또는 PowerShell 7
- **cargo**: 소스에서 설치하거나 빌드할 때 필요

## 로컬 소스에서 설치

```text
git clone https://github.com/boxqkrtm/wt-manager.git
cd wt-manager
cargo install --path .
```

설치 후 현재 플랫폼에 맞게 셸 통합을 등록합니다.

```bash
# macOS / Linux
wt init
```

```powershell
# Windows PowerShell
wt init --profile $PROFILE
```

`wt init`은:
- macOS/Linux에서 `~/.wt-manager.sh`를 생성하고 zsh/bash rc에 등록
- Windows에서 `~/.wt-manager.ps1`을 UTF-8로 생성하고 `--profile`로 지정한 PowerShell profile에 등록
- 지원 셸에서 `wt <Tab>`, `wt cd <Tab>`, `wt run <Tab>`, `wt delete <Tab>`의 명령 및 로컬 브랜치 자동완성 등록
- TUI 출력은 터미널에 직접 렌더링하고, 선택 후 이동 경로는 호출별 UTF-8 marker 파일로 전달
- marker 처리 후 호출한 셸에서 `cd` 또는 `Set-Location -LiteralPath`를 수행하고 임시 파일과 환경 변수를 정리

> `~/.wt-manager.sh`와 `~/.wt-manager.ps1`은 generated file이며 `wt init` 재실행 시 갱신됩니다. PowerShell profile은 기존 UTF-8/UTF-16 인코딩을 보존하고 같은 통합 블록을 중복 등록하지 않습니다.

## 사용법

### 기본 사용

```bash
# TUI로 워크트리 검색/생성
wt

# TUI로 강제 진입
wt tui

# shell integration 설치 (macOS / Linux)
wt init
# Windows PowerShell: wt init --profile $PROFILE

# 특정 브랜치 워크트리 생성/이동
wt feature-branch

# 명령형
wt list
wt cd feature-branch
wt run feature-branch -- pnpm test
wt delete feature-branch
wt delete feature-branch --force
wt clean
wt clean --merged
wt clean --remote origin --dry-run

# 저장된 프로젝트 목록
wt project list

# 저장소별 wt 설정
wt config show
wt config copy add .env.local
wt config copy remove .env.local
wt config hook add post-create "pnpm install"
wt config hook remove post-create 0
```

### `wt worktree clean` 도움말

- 용도: 리모트에서 삭제된 추적 브랜치 기준으로 워크트리를 정리합니다.
- 동작:
  - 기본적으로 각 워크트리의 추적 브랜치(`branch@{upstream}`)가 존재하지 않으면 삭제 대상
  - 실행 전 `git fetch --prune <remote>`로 원격 상태를 갱신(전체는 `--skip-fetch`로 생략 가능)
  - 기본은 현재 설정된 모든 추적 upstream 기준, 특정 원격만 지정하려면 `--remote origin`
  - 삭제 전 미리 확인하려면 `--dry-run`
  - 강제 삭제가 필요하면 `--force`
  - 추적 브랜치가 없는 워크트리까지 포함하려면 `--include-untracked`

### `wt worktree clean --merged`

- 용도: 기본 브랜치에 이미 merge된 worktree 브랜치를 정리합니다.
- 기준 브랜치:
  - `--base <branch>` 지정 시 해당 브랜치
  - 미지정 시 `origin/HEAD` 기준 default branch를 우선 사용
  - 실패 시 `origin/main`, `origin/master`, `main`, `master` 순서 fallback
- `--dry-run`, `--force`는 기존 clean과 동일하게 동작합니다.

### `wt config`

- 설정은 repo-local 파일이 아니라 내장 `~/.wt-manager/db.json`에 repo별로 저장됩니다.
- 지원 항목:
  - `copy_files`: 새 worktree 생성 시 복사할 상대 경로 목록
  - `postCreate` hooks: 새 worktree 생성 직후 실행
  - `postCd` hooks: worktree 진입 직전 실행
- 기본값:
  - 새 repo entry 생성 시 lockfile / env manager 파일을 기준으로 `postCreate` 기본 hook이 자동 seed 됩니다.
  - 기존 repo entry는 자동으로 변경되지 않습니다.
- copy 규칙:
  - repo root 기준 상대 경로만 지원
  - `..`가 포함된 상위 디렉터리 경로는 허용하지 않음
  - source가 없으면 skip
  - destination이 이미 있으면 overwrite하지 않음
- hook은 macOS/Linux에서 zsh/bash/sh, Windows에서 `pwsh` 또는 `powershell`로 실행됩니다. Windows에서는 cmd/sh로 fallback하지 않습니다.
- 기존 Windows DB에 자동 seed된 POSIX profile-source prefix는 실행 시 제거하며, 사용자가 직접 추가한 hook은 현재 플랫폼의 셸 문법으로 작성해야 합니다.
- 런타임에서는 파일 탐지 기반 자동 설치를 하지 않고, DB에 저장된 hook만 실행합니다.

### `--help`에서 확인할 수 있는 항목

- `wt --help`에 기본 동작(`wt`, `wt <branch>`), TUI 진입(`wt tui`), 워크트리/프로젝트 명령이 노출됩니다.
- `wt worktree switch`와 `wt <branch>`는 동작이 같습니다.
- 삭제는 기본적으로 안전 삭제입니다. 메인 워크트리는 삭제할 수 없고, 실패 시 메시지에 `--force` 재시도 권장안이 표시됩니다.
- 실제 작업 디렉터리 이동은 `wt init`이 생성한 셸 통합(`~/.wt-manager.sh` 또는 `~/.wt-manager.ps1`)이 호출별 marker 파일을 읽어 수행합니다.

### TUI 조작법

#### 프로젝트 선택 화면
- **타이핑**: Fuzzy 검색
- **Left / Right / Home / End**: 입력 커서 이동
- **Backspace / Delete**: 커서 기준 삭제
- **Up / Down**: 후보 선택
- **Tab**: 최상위 매치로 자동완성
- **Enter**: 선택한 프로젝트로 이동
- **Ctrl+C / Esc**: 취소

#### 워크트리 선택 화면
- **타이핑**: Fuzzy 검색
- **Left / Right / Home / End**: 입력 커서 이동
- **Backspace / Delete**: 커서 기준 삭제
- **Up / Down**: 후보 선택
- **Tab**: 현재 선택 후보로 자동완성
- **Enter**: 현재 선택된 워크트리 선택
- **Ctrl+B**: 새 브랜치/워크트리 생성
- **Ctrl+X**: 워크트리 삭제 (정확히 일치할 때만 활성화)
- **Ctrl+C / Esc**: 취소

### 주요 기능

#### 1. 스마트 워크트리 생성
- 기존 브랜치로 먼저 시도
- 브랜치가 없으면 자동으로 새 브랜치 생성

#### 2. 안전한 삭제
- **정확한 일치**: 입력값이 100% 일치할 때만 삭제 가능
- **메인 보호**: 메인 워크트리는 삭제 불가
- **변경사항 보호**: 커밋되지 않은 파일이 있으면 삭제 차단

#### 3. 프로젝트 관리
- 워크트리 안에서 실행 시 메인 저장소 자동 인식
- 최근 사용 프로젝트 우선 표시

#### 4. GitHub PR 표시 제한
- `gh` CLI가 설치되어 있으면 워크트리 목록에 열린 PR 번호/제목을 함께 표시합니다.
- PR 메타데이터 표시는 최대 100개의 open PR까지만 지원합니다.

### 동작 방식

1. 새 워크트리는 기본적으로 `~/_wt/{owner}-{repo}-{hash5}/` 아래에 생성됩니다. macOS/Linux는 브랜치 경로를 유지하고, Windows는 금지 문자·예약명·긴 경로·대소문자 충돌을 피하는 `slug-{hash16}` 디렉터리명을 사용합니다.
2. 기존 버전에서 이미 사용 중인 `~/_wt/{프로젝트명}_{해시}/` 경로가 있으면 그 경로를 계속 재사용
   이 fallback은 구버전 호환 유지를 위한 동작이며, 충분한 마이그레이션 이후 제거될 수 있습니다.
3. 새 repo entry가 처음 생성될 때 lockfile / env manager 파일 기준으로 기본 `postCreate` hook 값이 seed 될 수 있음
4. worktree 생성/이동 시에는 파일 자동 탐지 없이 `~/.wt-manager/db.json`에 저장된 `postCreate` / `postCd` hook만 실행
5. 실제 셸 이동은 생성된 zsh/bash 또는 PowerShell 통합이 호출별 marker 파일에서 이동 경로를 읽고, 이동 후 임시 파일과 전달용 환경 변수를 정리합니다.

## 라이선스

MIT
