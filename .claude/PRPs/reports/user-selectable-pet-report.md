# Implementation Report: User-Selectable Pet

## Summary

펫 선택을 primary provider로부터 분리했다. Settings에 펫 드롭다운을 추가하고, 목록은 네이티브가 설치된 패키지를 열거해 채운다. 번들 설치 로직의 하드코딩(`["cat","corgi"]`)과 stock 판별 하드코딩(`"cat" => "Cat"`)을 제거해, 이후 버전에서 펫 추가가 `resources/pets/`에 폴더를 넣는 것만으로 끝나도록 했다. 함께 `cat` 패키지를 `tabby`로 개명하고 기존 사용자 마이그레이션과 구 패키지 정리를 포함했다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Medium | Medium |
| Confidence | 8/10 | 정확 — 계획된 GOTCHA가 전부 실제로 발생 |
| Files Changed | 20 | 38 (프레임 PNG 16개 리네임 포함, 실질 소스 21개) |
| Tasks | 12 | 12 완료 |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | 펫 패키지 열거 (`PetPackageRepository::list`) | 완료 | |
| 2 | `should_preserve_installed` 하드코딩 제거 | 완료 | `bundled_manifest_version` → `bundled_manifest_info` |
| 3 | `list_pet_packages` IPC 커맨드 | 완료 | panel 전용 인가 |
| 4 | 번들 설치 스캔 + 은퇴 패키지 정리 | 완료 | `RETIRED_PET_IDS = ["cat"]` |
| 5 | settings 기본값 + `cat`→`tabby` 마이그레이션 | 완료 | V1/V2/Legacy 경로 `load_locked()` 재진입 |
| 6 | `tabby` 패키지 생성 | **완료 (편차)** | 재생성 대신 커밋된 패키지 개명 — 아래 참조 |
| 7 | 게이트웨이 계약 확장 | 완료 | `PetSummaryModel`, `listPetPackages` |
| 8 | fixture/목 갱신 | 완료 | |
| 9 | 뷰 모델에 `selectedPetId` 노출 | 완료 | 두 타입 모두 갱신 |
| 10 | Settings 펫 드롭다운 | 완료 | 열거 실패 시 현재 펫 폴백 |
| 11 | 결합 제거 + 목록 배선 | 완료 | `PROVIDER_PET` 삭제 |
| 12 | 테스트 갱신 및 추가 | 완료 | |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Static Analysis (TS) | 통과 | `pnpm check` — 0 errors, 0 warnings |
| Lint | 통과 | `pnpm lint` — eslint + prettier 클린 |
| Unit Tests (renderer) | 통과 | 230/230, 23 파일 |
| Coverage | 통과 | 95.29% stmts / 90.18% branch / 91.91% funcs / 95.29% lines (게이트 80%) |
| Build | 통과 | `pnpm build` — 7.86s |
| Rust fmt | 통과 | `cargo fmt --check` 전체 크레이트 클린 |
| Rust tests (store) | 통과 | 37/37, 격리 크레이트 |
| Rust clippy (store) | 통과 | `-D warnings` 클린, 격리 크레이트 |
| **Rust 전체 빌드** | **미검증** | 아래 "환경 제약" 참조 |
| Integration/E2E | 미실행 | 네이티브 실행 불가 (동일 제약) |

### 환경 제약 — Rust 전체 크레이트 미검증 (중요)

이 WSL 환경에서는 Tauri 크레이트를 빌드할 수 없다:

- `pkg-config`와 GTK/WebKitGTK 개발 라이브러리 부재 → `cargo check`가 `gio-sys` 빌드에서 실패
- WSL interop 비활성 → Windows 툴체인(`/mnt/c/Users/user/.cargo/bin/cargo.exe`)도 `Exec format error`로 실행 불가

따라서 **순수 로직 모듈만** 격리 크레이트(`scratchpad/store-check`)로 컴파일·테스트했다:

- 검증됨: `domain.rs`, `store/{mod,pets,settings,history,snapshots,tests}.rs` — 37 테스트 통과, clippy 클린
- **미검증**: `refresh/ipc.rs`(신규 `list_pet_packages`), `lib.rs`(`install_bundled_pet_packages` 재작성 + 테스트 8개), `window/mod.rs`(enum 변종 + 인가)

미검증 변경은 기존 템플릿을 그대로 따른 기계적 코드지만, **머지 전 Windows 또는 CI에서 아래를 반드시 실행해야 한다**:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml --all-features
cargo clippy --manifest-path src-tauri\Cargo.toml --all-features -- -D warnings
```

`lib.rs` 번들 설치 테스트 8개(신규 `installs_every_package_found_in_the_bundle_directory`,
`removes_retired_pet_packages_from_the_install_directory` 포함)는 이 실행 전까지 한 번도 돌지 않았다.

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src-tauri/src/store/pets.rs` | UPDATED | +76 / -21 |
| `src-tauri/src/store/mod.rs` | UPDATED | +4 / -2 |
| `src-tauri/src/store/settings.rs` | UPDATED | +29 / -12 |
| `src-tauri/src/store/tests.rs` | UPDATED | +97 / -3 |
| `src-tauri/src/refresh/ipc.rs` | UPDATED | +11 / -2 |
| `src-tauri/src/window/mod.rs` | UPDATED | +2 |
| `src-tauri/src/window/tests.rs` | UPDATED | +3 |
| `src-tauri/src/lib.rs` | UPDATED | +221 / -76 |
| `src-tauri/resources/pets/cat/` → `tabby/` | RENAMED | 17 파일 (PNG 16 + manifest) |
| `scripts/build-pet-packages.py` | UPDATED | +3 / -1 |
| `src/lib/api/gateway.ts` | UPDATED | +10 |
| `src/lib/api/fixtureGateway.ts` | UPDATED | +3 |
| `src/lib/state/presentation.ts` | UPDATED | +2 |
| `src/lib/stores/settings.ts` | UPDATED | +3 |
| `src/lib/components/SettingsPanel.svelte` | UPDATED | +24 / -3 |
| `src/App.svelte` | UPDATED | +32 / -14 |
| `src/App.test.ts` | UPDATED | +56 / -5 |
| `src/lib/components/SettingsPanel.test.ts` | UPDATED | +15 |
| `src/lib/assets/manifest.test.ts` | UPDATED | +4 / -2 |
| `src/lib/state/presentation.test.ts` | UPDATED | +3 / -1 |
| `src/lib/stores/settings.test.ts` | UPDATED | +1 |

## Deviations from Plan

### 1. Task 6 — 아트 재생성 대신 커밋된 패키지 개명

**WHAT**: `python3 scripts/build-pet-packages.py`로 `tabby`를 새로 생성하는 대신, `git mv`로 `resources/pets/cat/` → `tabby/`, 프레임 파일 16개를 `cat_*` → `tabby_*`로 리네임하고 manifest의 `id`/`displayName`/`frames` 경로만 재작성했다. `PET_SOURCES`는 계획대로 `{"tabby": "cat", "corgi": "corgi"}`로 갱신했다.

**WHY**: 이 환경에 numpy/scipy가 없다(Pillow만 존재). 스크립트는 순수 Python 폴백(`_clean_opaque_frame_fallback`)으로 동작하지만, 커밋된 아트는 numpy 경로로 생성된 것이라 **폴백이 다른 픽셀을 낼 위험**이 있었다. 계획의 Risks 표에 적어둔 항목이 실제로 발생한 셈이다. 개명 방식은 아트 바이트를 보존하므로 기존 사용자에게 시각적 변화가 없다.

**영향**: 없음. 결과물 레이아웃은 스크립트가 생성했을 것과 동일하다(`frames/tabby_{state}_{NN}.png`, `displayName: "Tabby"`, `version: 1`). numpy/scipy가 있는 환경에서 스크립트를 돌리면 같은 구조로 재생성된다.

### 2. `lib.rs` 번들 설치 테스트 전면 재작성 (계획보다 큰 변경)

**WHAT**: 계획은 "`["cat","corgi"]` 픽스처를 `["tabby","corgi"]`로 갱신"으로 예상했으나, 실제로는 5개 테스트를 모두 재작성하고 헬퍼 `write_bundled_manifest`를 추가했으며 테스트 2개를 신규 추가했다.

**WHY**: 기존 테스트들이 번들 manifest로 `"bundled cat"` 같은 **비-JSON 문자열**을 썼다. 구 `bundled_manifest_version`은 파싱 실패 시 `0`을 반환해 통과했지만, 새 `bundled_manifest_info`는 `None`을 반환하고 호출부가 그 패키지를 **건너뛴다**. 픽스처를 유효 JSON으로 바꾸지 않으면 테스트가 아무것도 검증하지 않게 된다. 또한 `cat`이 `RETIRED_PET_IDS`에 들어가면서 "사용자 커스터마이즈 보존" 테스트의 전제(`cat` 패키지가 살아남음)가 깨져 `tabby` 기준으로 옮겨야 했다.

### 3. 환경 수리 — linux 네이티브 바이너리 추가 설치

**WHAT**: `node_modules/.pnpm/` 아래에 `@rollup/rollup-linux-x64-gnu@4.62.2`와 `@esbuild/linux-x64@0.28.1`을 npm 레지스트리에서 받아 배치했다.

**WHY**: `node_modules`가 Windows에서 설치되어 win32 바이너리만 있었고, WSL에서 vitest/svelte-check가 기동조차 못 했다. `pnpm install`은 "node_modules를 전부 지우고 재설치" 확인을 요구했고 — 실행했다면 **사용자의 Windows 개발 환경이 깨졌을 것**이라 취소했다.

**영향**: `package.json`과 `pnpm-lock.yaml` **미변경**. 추가 전용이므로 Windows 쪽 워크플로에 영향 없음. `node_modules`는 gitignore 대상이라 커밋되지 않는다.

## Issues Encountered

| 문제 | 해결 |
|---|---|
| `cargo` PATH 부재 | `$HOME/.cargo/bin` 추가 |
| Tauri 크레이트 빌드 불가 (pkg-config/GTK 부재) | 순수 모듈 격리 크레이트로 우회, 미검증 범위 명시 |
| Windows cargo interop 불가 (`Exec format error`) | 상동 |
| rollup/esbuild linux 바이너리 부재 | 레지스트리에서 추가 설치 (비파괴적) |
| numpy/scipy 부재로 아트 재생성 위험 | 개명 방식으로 전환 (편차 1) |
| 비-JSON 번들 manifest 픽스처 | 테스트 재작성 (편차 2) |
| `Provider` import 고아화 | 제거 (`PROVIDER_PET` 삭제의 결과) |

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src-tauri/src/store/tests.rs` | +5 | `list()` 정렬·손상 패키지 스킵·빈 디렉터리·루트 부재, `cat`→`tabby` 마이그레이션, V2+`cat` 재진입 |
| `src-tauri/src/lib.rs` | +2 | 번들 디렉터리 전수 설치(3종), 은퇴 패키지 제거 |
| `src-tauri/src/window/tests.rs` | +2 assertions | `ListPetPackages` panel 허용 / overlay 거부 |
| `src/App.test.ts` | +2 | Settings에서 펫 변경 시 primary 불변, 열거 실패 시 폴백 |
| `src/lib/components/SettingsPanel.test.ts` | +1 assertion | 펫 선택이 primary를 건드리지 않음 |
| `src/App.test.ts` (수정) | 1 | primary 변경 시 펫 **유지** — 기존 강제 교체 단언 반전 |

## Next Steps

- [ ] **Windows 또는 CI에서 `cargo test`/`cargo clippy` 실행** (필수 — 위 환경 제약 참조)
- [ ] `pnpm tauri dev`로 수동 검증 — 특히 업그레이드 시나리오(`settings.json`에 `"selected_pet_id": "cat"` + `pets/cat/` 존재 상태에서 기동)
- [ ] `/code-review`로 변경 검토
- [ ] `/prp-pr`로 PR 생성 (base: `develop`)
