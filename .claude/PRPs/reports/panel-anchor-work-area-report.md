# Implementation Report: 패널 앵커를 work area 기준 세로 중심 정렬로 교체

## Summary

펫이 화면 하단에 있을 때 사용량 패널이 작업 표시줄 뒤로 잘리던 문제를 수정했다. `anchor_panel`의 세로 정렬을 상단 정렬에서 펫 중심 정렬로 바꾸고, clamp 기준을 `monitor.size()`(전체 해상도)에서 `Monitor::work_area()`(작업 표시줄/Dock/메뉴바 제외 영역)로 교체했다. 계획에 없던 `lib.rs`의 중복 배치 구현을 발견해 공용 `position_panel`로 통합했고, 첫 오픈 시 위치가 튀는 문제를 layout gate로 제거했다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Medium | Medium |
| Confidence | 8/10 | 검증 부분 달성 — 아래 참조 |
| Files Changed | 4 | 4 |
| Tasks | 6 | 6 + 계획 외 1 (중복 제거) |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | `anchor_panel` 세로 중심 정렬 + `work_area` 파라미터 | 완료 | |
| 2 | `position_panel`이 `work_area()` 사용 | 완료 | 편차 — gap을 DPI 스케일링하도록 변경 |
| 3 | `resize_panel` 매직넘버 상수화 | 완료 | |
| 4 | layout gate로 첫 오픈 튐 제거 | 완료 | |
| 5 | `PanelLayoutGate` managed state 등록 | 완료 | |
| 6 | 회귀 테스트 4건 추가 | 완료 | |
| — | **계획 외**: `lib.rs`의 중복 앵커 구현 제거 | 완료 | 아래 Deviations 참조 |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| rustfmt (파싱 + 포맷) | 통과 | 변경 4개 파일 전부 |
| `window` 모듈 단위 테스트 | 통과 | **격리 크레이트에서 14/14** — 신규 4건 + 기존 10건 |
| Tauri API 시그니처 검증 | 통과 | vendored 소스로 5개 API 확인 (아래) |
| `cargo build` / `clippy` / `cargo test` (전체) | **미실행** | 환경 제약 — 아래 참조 |
| `pnpm test:ci` | **미실행** | 환경 제약 — 아래 참조 |
| 수동 검증 | **미실행** | 앱 실행 불가 (동일 제약) |

### 실행하지 못한 검증과 그 이유

**`cargo build` / `cargo clippy` / `cargo test --all-features`**
WSL 환경에 `pkg-config`와 GTK/WebKitGTK 개발 패키지가 없어 `gdk-sys` 빌드 스크립트가 실패한다. `sudo`는 비밀번호를 요구해 설치할 수 없었다.

```
Could not run `pkg-config --libs --cflags gdk-3.0 'gdk-3.0 >= 3.22'`
The pkg-config command could not be found.
```

설치 명령 (사용자가 직접 실행 필요):
```bash
sudo apt install -y pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf
```

**`pnpm test:ci`**
`node_modules`에 `@rollup/rollup-linux-x64-gnu`가 없다 (Windows에서 설치된 트리를 WSL에서 실행 중). `pnpm install`은 트리를 통째로 재설치하겠다는 확인을 요구했고, 이를 수행하면 Windows용 네이티브 바이너리가 Linux용으로 교체되어 사용자의 Windows 쪽 워크플로가 깨질 수 있어 **의도적으로 중단했다**. `node_modules`는 손상 없이 그대로다.

이 변경은 `src/` 파일을 한 줄도 건드리지 않았으므로 (`git diff --stat`로 확인) 렌더러 회귀 위험은 구조적으로 낮다. 유일한 결합점인 `invoke('show_panel', {})` / `invoke('resize_panel', {height})`의 인자 형태는 보존했다 — 추가한 `State` 파라미터는 Tauri가 네이티브 쪽에서 주입한다.

### 격리 테스트 방법

`window` 모듈은 `serde` / `thiserror`에만 의존하고 tauri에 의존하지 않으며, 유일한 `#[cfg(windows)]` 블록은 Linux에서 컴파일 제외된다. 이를 이용해 스크래치 크레이트로 복사해 실행했다:

```
scratchpad/window-check/
  Cargo.toml          # serde(derive) + thiserror만
  src/lib.rs          # src-tauri/src/window/mod.rs 복사본
  src/tests.rs        # src-tauri/src/window/tests.rs 복사본
```

결과: `14 passed; 0 failed`. 실제 버그 수정 로직(Task 1)과 그 회귀 테스트(Task 6)는 완전히 검증됐다. 검증되지 않은 것은 `ipc.rs` / `lib.rs`의 **타입 체크**다.

### 정적으로 확인한 Tauri API

컴파일 없이 vendored 소스로 대조:

| API | 위치 | 확인 내용 |
|---|---|---|
| `Monitor::work_area()` | `tauri-2.11.5/src/window/mod.rs:95` | `-> &PhysicalRect<i32, u32>` |
| `PhysicalRect` | `tauri-runtime-2.11.3/src/dpi.rs:28` | `.position` / `.size` 필드 (메서드 아님) |
| `tauri::Monitor` | `tauri-2.11.5/src/lib.rs:231` | 크레이트 루트 재export 확인 |
| `WebviewWindow::is_visible()` | `tauri-2.11.5/src/webview/webview_window.rs:1800` | `-> crate::Result<bool>` |
| `Manager::state<T>()` | `tauri-2.11.5/src/lib.rs:729` | `-> State<'_, T>` |

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src-tauri/src/window/mod.rs` | UPDATED | +16 / −6 |
| `src-tauri/src/window/tests.rs` | UPDATED | +120 / −0 |
| `src-tauri/src/refresh/ipc.rs` | UPDATED | +124 / −34 |
| `src-tauri/src/lib.rs` | UPDATED | +6 / −32 |

합계 `+260 / −72`, 4개 파일. 렌더러 `src/` 변경 0줄 (계획대로).

## Deviations from Plan

### 1. `lib.rs`에 중복 앵커 구현이 있었다 (계획 누락)

**WHAT**: 계획은 `anchor_panel` 호출부를 2곳(`ipc.rs`, 테스트 전용 `panel_position`)으로 봤으나, `restore_window_positions`(`lib.rs`, 시작 시 1회 실행)에 세 번째 호출부가 있었다. `position_panel`의 복제본이었다.

**WHY**: 놔두면 같은 버그가 시작 경로에 남는다. 게다가 gap이 `12.0 * scale`로 `ipc.rs`의 `12.0`과 달라 HiDPI에서 두 경로가 서로 다른 위치를 계산했다. `position_panel`을 `pub(crate)`로 승격하고 중복 블록(32줄)을 호출 한 줄로 교체했다.

### 2. gap을 DPI 스케일링하도록 변경

**WHAT**: 계획은 `PANEL_ANCHOR_GAP: f64 = 12.0`을 물리 픽셀 상수로 두었으나, `PANEL_ANCHOR_GAP_LOGICAL = 12.0`으로 이름을 바꾸고 `monitor.scale_factor()`를 곱해 넘긴다.

**WHY**: 편차 1의 통합 과정에서 두 경로의 gap 정의가 충돌했다. 물리 12px 고정은 200% 디스플레이에서 논리 6px로 보여 의도보다 좁다. `lib.rs`가 쓰던 스케일링 방식이 시각적으로 옳으므로 그쪽으로 통일했다. 스케일 값 가드(`is_finite() && > 0.0`)는 `lib.rs:273-277`의 기존 패턴을 그대로 따랐다 (`usable_scale` 헬퍼).

### 3. `hide_panel` 시그니처 변경

**WHAT**: 계획대로 `State<'_, PanelLayoutGate>`를 추가했다. 계획서 "Files to Change" 표에는 `hide_panel`이 명시돼 있지 않았으나 Task 4 본문에는 있었다 — 실제로 필요하다. 대기 중인 grace 타이머가 방금 닫은 패널을 되살리는 것을 막는다.

## Issues Encountered

| 문제 | 해결 |
|---|---|
| `cargo`가 PATH에 없음 | `~/.cargo/bin`을 각 명령에서 export |
| GTK/pkg-config 부재로 네이티브 빌드 불가 | 격리 스크래치 크레이트로 `window` 모듈만 검증. 나머지는 미검증으로 보고 |
| `pnpm install`이 node_modules 전체 재설치 요구 | 중단. Windows 바이너리 파괴 위험이 사용자 결정 사항이라 판단 |
| rustfmt가 신규 테스트 한 곳의 `Point` 리터럴을 여러 줄로 요구 | `cargo fmt` 적용 후 재검증 |

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src-tauri/src/window/tests.rs` | 4건 신규 | 하단 펫 작업 표시줄 회귀, 세로 중심 정렬 정책 판별, work area 상단 clamp(macOS 메뉴바), 오버사이즈 패널 상단 고정 |

기존 `panel_anchor_flips_then_clamps_inside_display`와 `controller_recovers_position_anchors_panel_and_hides_only_presentation`은 계획서 Notes의 예측대로 **기댓값이 바뀌지 않고 통과**했다 (두 정렬 정책이 clamp 후 동일한 값을 내는 입력이라 정책을 판별하지 못함). 판별 역할은 신규 `panel_anchor_centers_vertically_on_pet_when_space_allows`가 맡는다.

## Next Steps

- [ ] **필수**: GTK 개발 패키지 설치 후 `cargo clippy --all-features -- -D warnings`와 `cargo test --all-features` 실행. `ipc.rs` / `lib.rs`는 아직 타입 체크되지 않았다
- [ ] **필수**: `pnpm test:ci` 실행 (또는 CI에 맡기기)
- [ ] **필수**: `pnpm tauri dev`로 계획서의 수동 검증 체크리스트 수행 — 특히 하단/상단 펫 배치와 콜드 스타트 첫 오픈
- [ ] `/code-review`로 리뷰
- [ ] `/prp-pr`로 PR 생성
