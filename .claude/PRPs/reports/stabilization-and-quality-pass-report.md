# Implementation Report: CacheBite 안정화 · 디자인 버그 수정 · 코드 품질 향상

## Summary

계약(`docs/ui-contract.md`)에 정의되어 있으나 배선되지 않았던 기능 7건, 디자인/표현 결함 6건,
코드 품질 이슈 8건을 수정했다. 핵심 패턴이었던 "순수 함수·상수는 구현되어 있으나 `App.svelte`에서
참조가 0건" 문제를 해소해 `SNAPSHOT_TTL_MS`, `expiresAt`, `setRefreshing`, `quit`, `defaultSize`가
모두 프로덕션 경로에서 사용된다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
| --- | --- | --- |
| Complexity | Large | Large — 계획대로 |
| Files Changed | 16 수정, 2 신규 | 29 수정, 4 신규 |
| Tests | — | 149 → 206 (+57) |
| Coverage | ≥ 80% 유지 | 94.95% stmts / 88.93% branch / 90.07% funcs |

파일 수가 예상보다 많은 이유는 전부 **테스트 파일**이다. 계획서가 세지 않은
`PetOverlay.test.ts`, `HistoryGraph.test.ts`, `SpeechBubble.test.ts`, `UsageGauge.test.ts`,
`UsagePanel.test.ts`, `providers.test.ts`, `App.test.ts`가 계약 변경에 따라 갱신됐다.

## Tasks Completed

| # | Task | Status | Notes |
| --- | --- | --- | --- |
| 0 | 검증 환경 복구 | **부분 완료** | pnpm 해결, Rust 컴파일은 여전히 차단 — 아래 참조 |
| 1 | 잘못된 테스트 수정 (RED) | 완료 | 3건 의도적 실패 확인 |
| 2 | 스냅샷 TTL 만료 배선 (GREEN) | 완료 | 신규 케이스 4건 포함 37건 통과 |
| 3 | 말풍선 자동 소멸 | 완료 | **진단 정정** — 아래 Deviations 참조 |
| 4 | 앱 종료 경로 | 완료 | panel 창 전용 인가 확인 |
| 5 | 시각 표기 휴먼화 | 완료 | `format/time.ts` 신규, 24 테스트 |
| 6 | 패널 시스템 상태 안내 | 완료 | `systemGuidance.ts` 신규, 8 테스트 |
| 7 | 접근성 결함 | 완료 | 링 합성 라벨, 그래프 tabpanel |
| 8 | 시간 경과 반영 | 완료 | 60초 `$state` 티커 |
| 9 | 새로고침 진행 표시 | 완료 | 상태 이벤트 해제 + 30초 상한 |
| 10 | macOS 다크 유리 효과 | **코드 완료 / 시각 미검증** | macOS 미보유 |
| 11 | 매니페스트 `defaultSize` | 완료 | 240px 창 크기로 클램프 |
| 12 | 배지 아이콘 토큰화 | 완료 | `--badge-icon` 신규 |
| 13 | `any` 제거 | 완료 | **잠재 결함 발견** — 아래 참조 |
| 14 | 이벤트 타입 캐스팅 제거 | 완료 | `RaisedSeverity` + 타입 술어 |
| 15 | 죽은 코드 인벤토리 | 완료 | 삭제 0건, 모든 항목 grep 실측 |
| 16 | 자잘한 정합성 | **부분 완료** | 렌더러 완료, Rust 컴파일 미검증 |

## Validation Results

| Level | Status | Notes |
| --- | --- | --- |
| `pnpm check` (svelte-check) | **통과** | 0 errors / 0 warnings |
| `pnpm lint` (eslint + prettier) | **통과** | — |
| `pnpm test` (vitest) | **통과** | 206/206 |
| Coverage (≥80% 게이트) | **통과** | 94.95 / 88.93 / 90.07 / 94.95 |
| `pnpm test:ci` (빌드 포함) | **통과** | vite build 성공 |
| `pnpm test:e2e:renderer` | **통과** | 2/2 (dev 서버 기동 필요) |
| `cargo fmt --check` | **통과** | — |
| `cargo clippy` | **미실행 (차단)** | 시스템 라이브러리 부재 |
| `cargo test` | **미실행 (차단)** | 시스템 라이브러리 부재 |
| macOS 시각 확인 | **미실행** | 플랫폼 미보유 |

## 검증 환경 (Task 0) 실측 결과

### 해결됨 — pnpm

계획서는 `/mnt/c`(DrvFs)에서 `pnpm install`이 `ERR_PNPM_EACCES: rename`으로 실패하며
pnpm 옵션으로는 우회 불가라고 기록했다. 실제로 재현했고(656 중 640에서 중단), 아래로 해결했다.

```bash
pnpm install --frozen-lockfile --virtual-store-dir="$HOME/.cachebite-deps/.pnpm"
```

rename이 일어나는 곳은 virtual store(`.pnpm/<pkg>/node_modules/<pkg>_tmp_*` → 최종 경로)뿐이므로,
**virtual store만 ext4로 옮기면 저장소는 `/mnt/c`에 그대로 두고도 설치가 성공한다.**
`node_modules/<pkg>`는 ext4를 가리키는 심링크가 되며 DrvFs에서 심링크는 정상 동작한다.
저장소 이전도, `.npmrc` 커밋도 필요 없다.

> 시도했으나 실패한 방법: `node_modules` 전체를 심링크 — pnpm이 `ENOTDIR`로 거부한다.

### 미해결 — Rust 컴파일

`rustup`으로 핀된 1.97.1을 설치했고 `cargo fmt --check`는 통과한다. 그러나
`cargo test`/`cargo clippy`는 빌드 단계에서 실패한다.

```
libdbus-sys-0.2.7/build.rs: 'libdbus-1-dev' and 'pkg-config' are installed?
```

`pkg-config`, `libdbus-1-dev`, WebKitGTK 4.1 등이 없고 설치에 sudo 비밀번호가 필요하다.
사용자가 apt 설치를 선택했으나 이 세션 중 실행되지 않았다. **따라서 이번 패스의 Rust 변경
2건은 컴파일 검증을 받지 않았다.**

해제 방법:

```bash
sudo apt install -y pkg-config libdbus-1-dev libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf build-essential libssl-dev

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cachebite-deps/target"   # ext4, DrvFs보다 훨씬 빠름
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

미검증 Rust 변경에 대한 정적 교차 검증은 수행했다.

- `State<'_, HistoryRepository>` 소비처는 `get_history` 하나뿐이며 함께 수정했다.
- `app.manage(...)`로 `HistoryRepository`를 등록하는 곳도 한 곳뿐이다.
- `HistoryRepository { path: PathBuf, lock: Arc<Mutex<()>> }`는 `Send + Sync`이므로
  `Arc<HistoryRepository>`가 Tauri `State`의 트레잇 경계를 만족한다.
- `Arc` deref로 `repository.load()`가 그대로 해결된다.

## Files Changed

| File | Action |
| --- | --- |
| `src/lib/format/time.ts` | CREATED |
| `src/lib/format/time.test.ts` | CREATED |
| `src/lib/components/systemGuidance.ts` | CREATED |
| `src/lib/components/systemGuidance.test.ts` | CREATED |
| `docs/dead-code-inventory.md` | CREATED |
| `src/App.svelte` | UPDATED |
| `src/lib/state/engine.ts` | UPDATED |
| `src/lib/stores/providers.ts` | UPDATED |
| `src/lib/stores/interaction.ts` | UPDATED |
| `src/lib/interaction/bubblePolicy.ts` | UPDATED |
| `src/lib/api/gateway.ts` | UPDATED |
| `src/lib/api/fixtureGateway.ts` | UPDATED |
| `src/lib/components/UsagePanel.svelte` | UPDATED |
| `src/lib/components/UsageGauge.svelte` | UPDATED (`lang="ts"` 전환) |
| `src/lib/components/SplitUsageRing.svelte` | UPDATED |
| `src/lib/components/HistoryGraph.svelte` | UPDATED |
| `src/lib/components/PetOverlay.svelte` | UPDATED |
| `src/lib/components/SpeechBubble.svelte` | UPDATED |
| `src/lib/components/SystemBadge.svelte` | UPDATED |
| `src/lib/components/SettingsPanel.svelte` | UPDATED |
| `src/lib/components/models.ts` | UPDATED |
| `src/lib/styles/tokens.css` | UPDATED |
| `src/lib/styles/global.css` | UPDATED |
| `src-tauri/src/lib.rs` | UPDATED (미검증) |
| `src-tauri/src/refresh/ipc.rs` | UPDATED (미검증) |
| 테스트 7종 | UPDATED |

총 +742 / -89 라인.

## Deviations from Plan

### 1. Task 3 — 말풍선 결함의 원인이 계획서 진단과 다름

**WHAT**: 계획서는 A2를 "말풍선이 클릭할 때까지 사라지지 않음 / `expiresAt` 참조 0건"으로
기록했다. 실제로는 `SpeechBubble.svelte`에 `onMount` 기반 8초 타이머가 **이미 있었고**
`SpeechBubble.test.ts`가 이를 검증하고 있었다.

**WHY**: 진짜 결함은 다른 것이다. `onMount`는 컴포넌트 인스턴스당 한 번만 실행되므로,
말풍선이 **교체**될 때(계약 §7.1-3) 타이머가 재무장되지 않는다. 7초 시점에 도착한 새 말풍선은
전체 8초가 아니라 1초 뒤에 사라진다. 또한 타이밍 로직이 컴포넌트에 있어
"컴포넌트에는 로직을 두지 않는다"는 계획서 패턴을 위반한다.

**조치**: `App.svelte`의 `$effect`가 `bubble.expiresAt`을 관찰하도록 배선했다. 말풍선 객체
identity가 바뀌면 effect가 재실행되어 타이머가 재무장된다. 중복이 된 컴포넌트 타이머는
제거했고, `SpeechBubble.test.ts`는 "타이밍은 정책 계층 소유"를 단언하도록 바꿨다.

### 2. Task 5 — `absoluteShort` 시그니처

**WHAT**: 계획서는 `absoluteShort(isoTimestamp, nowMs)`를 명시했으나 `(isoTimestamp, timeZone?)`로 구현.

**WHY**: 벽시계 라벨("Mon 09:00")은 현재 시각에 의존하지 않아 `nowMs`가 미사용 인자가 되고,
`@typescript-eslint/no-unused-vars`에 걸린다. 계획서 GOTCHA가 요구한 "테스트에서 UTC 고정"도
`timeZone` 인자로 직접 충족된다. `relativeFromNow`/`capturedAgo`는 계획대로 `nowMs`를 받는다.

### 3. Task 13 — `any` 제거가 잠재 결함을 드러냄

**WHAT**: `SettingsPanel.svelte`의 `onChange?: (settings: any) => void`를 `SettingsStoreState`로
좁히자 `svelte-check`가 실제 타입 오류를 보고했다.

```
Type 'string' is not assignable to type 'Provider'.
  onChange({ ...settings, primaryProvider: event.currentTarget.value })
```

**WHY**: `<select>`의 `value`는 DOM 상 `string`이며, `any`가 이 불일치를 **가려 왔다**.
경계에서 좁히는 `asProvider()` 헬퍼를 추가했다. 실행 중 문제를 일으키진 않았지만(마크업이
두 값만 제공) 타입 안전성이 마크업 규율에만 의존하고 있었다.

### 4. Task 16 — `HistoryRepository` 중복은 동작 버그가 아님

**WHAT**: 계획서는 `lib.rs:85`와 `:101`의 중복 생성을 결함으로 기록했다.

**WHY**: 실측 결과 두 경로는 동일하며(둘 다 `app.path().app_data_dir()?`),
`store::path_lock`이 정규화된 경로별로 동일 `Arc<Mutex<()>>`를 반환한다. 즉 두 인스턴스가
**같은 뮤텍스를 공유**하므로 쓰기 경합은 없다. 순수한 중복 생성이며, 계획대로 Arc 공유로
정리했다(`get_history` 시그니처가 `State<'_, Arc<HistoryRepository>>`로 변경).

### 5. Task 15 — 인벤토리 검증 강화

계획서 항목을 그대로 옮기지 않고 `grep -rn`으로 **모든 항목의 참조 건수를 실측**했다.
window 모듈 13개 심볼 전부 프로덕션 참조 0건임을 확인했다.

## Issues Encountered

| 문제 | 해결 |
| --- | --- |
| DrvFs `pnpm install` 실패 | virtual store만 ext4로 이전 (위 참조) |
| Rust 시스템 라이브러리 부재 | **미해결** — sudo 필요 |
| Prettier가 `<p role="status">`를 줄바꿈해 공백 텍스트 노드 생성 → `:empty` 무력화 | 라이브 리전을 grid 밖으로 이동, `{guidance ?? ''}` 인라인 표현식으로 고정 |
| E2E 최초 실행 실패 (`Error: Timeout`) | 앱 회귀가 아니라 dev 서버 미기동. CI(`ci.yml:40`)처럼 `pnpm dev`를 먼저 띄우면 2/2 통과 |

## Tests Written

| Test File | 추가 | 검증 영역 |
| --- | --- | --- |
| `src/lib/format/time.test.ts` | 24 (신규) | 상대/절대/경과 시각, 경계값, 파싱 실패 |
| `src/lib/components/systemGuidance.test.ts` | 8 (신규) | 상태 6종 × provider 2종 안내 문구 |
| `src/lib/state/domain.test.ts` | +4 | TTL 만료 → error/offline/auth_required 유지/리비전 역행 |
| `src/lib/stores/providers.test.ts` | +5 | refreshing 토글, provider 독립성, 30초 상한 |
| `src/App.test.ts` | +4 | 말풍선 8초 소멸, 21분 stale 전이, 만료 안내, 종료 배선 |
| `src/lib/interaction/bubblePolicy.test.ts` | +3 | 만료 전 동일 참조, 만료 후 null, 빈 상태 no-op |
| `src/lib/components/UsageGauge.test.ts` | +2 | 5시간 카운트다운, 파싱 실패 시 `<time>` 제거 |
| `src/lib/components/UsagePanel.test.ts` | +6 | 종료 버튼, 안내 문구 4종, 빈 라이브 리전 |
| `src/lib/components/PetOverlay.test.ts` | +1 | 매니페스트 크기 반영 |

## Acceptance Criteria

- [x] Phase 1 (A1~A3) 전부 수정 + 회귀 테스트
- [x] Phase 2 디자인 결함 수정 (B4는 시각 미검증으로 기록)
- [x] `pnpm test:ci` 통과, 커버리지 ≥ 80%
- [ ] `cargo clippy -- -D warnings` 통과 — **차단 (시스템 라이브러리)**
- [x] `docs/dead-code-inventory.md` 작성 (삭제 미수행)
- [x] 참조 건수 모두 0 초과 (프로덕션 경로 기준): `SNAPSHOT_TTL_MS`=1, `expiresAt`=5,
      `setRefreshing`=5, `quit`=2, `defaultSize`=5
- [x] 새 의존성 0개
- [x] UI 문자열 영어 유지
- [x] hex 하드코딩 0건 (`#fff` 2곳 → `--badge-icon`)
- [x] `any` 0건, 강제 캐스팅 0건

## Next Steps

- [ ] **apt 설치 후 `cargo clippy` + `cargo test` 실행** — Rust 변경 2건이 미검증 상태
- [ ] macOS에서 라이트/다크 패널 시각 확인 (Task 10)
- [ ] `docs/dead-code-inventory.md`의 5개 Open question에 대한 사용자 결정
- [ ] `pnpm tauri dev` 수동 검증 체크리스트 (계획서 §6)
- [ ] `/code-review` → `/prp-pr`
