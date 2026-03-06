# wt-manager

Git worktree 관리 도구 - TUI 기반 워크트리 생성/전환/삭제

## 요구사항

- **zsh**: Shell wrapper 사용
- **cargo**: Rust 빌드 도구

## 설치

```bash
./install.sh
source ~/.zshrc  # 현재 셸에서 활성화
```

설치 스크립트가 자동으로:
- `cargo install`로 바이너리를 `~/.cargo/bin/wt`에 설치
- `~/.zshrc`에 wrapper 추가

> **참고**: 새 터미널을 열면 자동으로 활성화됩니다.

## 사용법

### 기본 사용

```bash
# TUI로 워크트리 검색/생성
wt

# TUI로 강제 진입
wt tui

# 특정 브랜치 워크트리 생성/이동
wt feature-branch

# 명령형(worktree)
wt worktree list
wt worktree switch feature-branch
wt worktree delete feature-branch
wt worktree delete feature-branch --force

# 저장된 프로젝트 목록
wt project list
```

### `--help`에서 확인할 수 있는 항목

- `wt --help`에 기본 동작(`wt`, `wt <branch>`), TUI 진입(`wt tui`), 워크트리/프로젝트 명령이 노출됩니다.
- `wt worktree switch`와 `wt <branch>`는 동작이 같습니다.
- 삭제는 기본적으로 안전 삭제입니다. 메인 워크트리는 삭제할 수 없고, 실패 시 메시지에 `--force` 재시도 권장안이 표시됩니다.
- 실제로 셸의 작업 디렉터리가 이동되는 것은 `wt-wrapper.sh`의 `cd` 파싱 동작입니다.

### TUI 조작법

#### 프로젝트 선택 화면
- **타이핑**: Fuzzy 검색
- **Tab**: 최상위 매치로 자동완성
- **Enter**: 선택한 프로젝트로 이동
- **Ctrl+C / Esc**: 취소

#### 워크트리 선택 화면
- **타이핑**: Fuzzy 검색
- **Tab**: 최상위 매치로 자동완성
- **Enter**: Fuzzy 매치된 워크트리 선택
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

### 동작 방식

1. 워크트리는 `~/_wt/{프로젝트명}_{해시}/{브랜치}/`에 생성
2. 생성/이동 후 자동으로 `pnpm install` 실행
3. 자동으로 해당 디렉토리로 이동

## 라이선스

MIT
