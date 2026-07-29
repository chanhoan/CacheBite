# Code Review: 로컬 미커밋 변경분 (user-selectable-pet)

**리뷰 일자**: 2026-07-28
**브랜치**: `feat/user-selectable-pet`
**베이스**: `f730708` (Merge pull request #8 from chanhoan/fix/panel-anchor-work-area)
**범위**: 수정 21개 파일 (+1314 / −150, 계획 문서 828줄 제외 시 +486 / −150), 리소스 `pets/cat` → `pets/tabby` 리네임(매니페스트 + 프레임 16개), 신규 IPC 커맨드 1개(`list_pet_packages`)
**결정**: **REQUEST CHANGES** — HIGH 1건, MEDIUM 5건

---

## Summary

펫 선택을 primary provider에서 분리하고 설치된 패키지를 네이티브에서 열거해 채우는 변경. 핵심 설계 판단은 대체로 정확하다.

- `install_bundled_pet_packages`가 하드코딩 `["cat","corgi"]` 대신 번들 디렉터리를 순회하도록 바뀌어, 펫 추가가 폴더 추가만으로 끝난다.
- `should_preserve_installed`의 `match id { "cat" => "Cat", ... }` 하드코딩을 번들 매니페스트에서 읽도록 옮긴 것은 같은 방향의 올바른 제거다.
- `list_pet_packages`를 `panel` 윈도우에만 인가하고(`window/mod.rs:90`), overlay 거부/panel 허용을 양쪽 다 테스트한 점(`window/tests.rs:324-325`)이 기존 인가 정책과 일관된다.
- `PetSummary`는 `id` + `displayName`만 노출해 자격 증명 경로·계정 식별자 유출이 없다 — 프라이버시 계약 유지.
- V1/V2 마이그레이션 후 `load_locked()` 재진입으로 구 파일의 `"cat"` 값까지 복구하는 방식은 중복 로직 없이 문제를 해결했고, 재진입이 최신 스키마 경로를 타므로 무한 루프가 아니다.
- 렌더러 폴백(열거 실패 시 활성 펫만 표시)과 그 테스트(`App.test.ts:492-518`)가 잘 짜여 있다.

다만 **은퇴 패키지 정리가 사용자 데이터를 무기한·무조건 삭제**하는 부분이 README의 "user-supplied packages" 지원 선언과 정면으로 충돌하고, 설치 경로 전반이 **all-or-nothing + silent skip**으로 되어 있어 번들 구성 실수가 런타임에 조용히 사라진다. 문서 3곳도 `cat` 기준으로 남아 있다.

---

## Findings

### CRITICAL

없음.

- 하드코딩된 자격 증명·토큰·API 키 없음
- 신규 DTO(`PetSummary`)·신규 IPC(`list_pet_packages`)에 authorization 값, 원본 provider 응답, 계정 식별자, 자격 증명 경로 노출 없음
- 신규 `console.log` / 디버그 출력 없음, `TODO`/`FIXME` 없음
- 경로 조작 방어 유지: `list()`가 직접 파일을 읽지 않고 `load()`를 경유하므로 `validate_id` → `canonicalize` → `starts_with(root)` → 에셋 경로 검사가 그대로 적용됨(`store/pets.rs:119-134` → `:68-112`)
- `displayName`은 Svelte 기본 이스케이프 경로로 렌더링되어 XSS 없음(`SettingsPanel.svelte:83`)

---

### HIGH

#### H-1. 은퇴 패키지 정리가 보존 규칙을 우회해 사용자 패키지를 영구 삭제하며, README의 지원 선언과 충돌

**위치**: `src-tauri/src/lib.rs:207`, `:217-224`

```rust
const RETIRED_PET_IDS: &[&str] = &["cat"];
...
for retired in RETIRED_PET_IDS {
    let stale = installed_pets.join(retired);
    if stale.exists() {
        fs::remove_dir_all(&stale)?;   // ← 무조건 삭제
    }
}
```

이 삭제는 `should_preserve_installed`를 거치지 않는다. 같은 변경에서 그 함수는 명시적으로 다음을 보장한다(`store/pets.rs:140-143`):

```rust
// A renamed pet is a user customization — never clobber it.
if package.manifest.display_name != bundled.display_name {
    return true;
}
```

즉 `tabby`를 사용자가 개명하면 절대 덮어쓰지 않지만, `cat`은 개명 여부와 무관하게 지워진다. 그리고 이 목록은 만료 시점이 없어 **매 실행마다 영구히** 동작한다.

`README.md:37`은 사용자 공급 패키지를 지원한다고 명시한다:

> Additional user-supplied packages can follow the same manifest contract

따라서 사용자가 자신의 패키지 id를 `cat`으로 지으면, 앱을 켤 때마다 경고 없이 사라진다. 복구 수단은 없다.

플랜(`.claude/PRPs/plans/completed/user-selectable-pet.plan.md:494`)은 "커스텀 펫 미지원 결정에 따른 의도된 동작"이라고 기록하고 있으나, README는 반대로 말하고 있다. **둘 중 하나는 고쳐야 한다.**

**수정안 (택 1)**

1. 코드를 보수적으로 — 재고품일 때만 삭제. 비용이 거의 없다:

```rust
for retired in RETIRED_PET_IDS {
    let stale = installed_pets.join(retired);
    if !stale.exists() {
        continue;
    }
    // 개명된 사본은 사용자 커스터마이즈 — should_preserve_installed와 같은 규칙.
    let is_stock = repository
        .load(retired)
        .map(|package| package.manifest.display_name == "Cat")
        .unwrap_or(true); // 로드 불가 = 깨진 재고 잔여물, 정리 대상
    if is_stock {
        let _ = fs::remove_dir_all(&stale);
    }
}
```
단, 이 순서로 옮기려면 `repository` 생성(`lib.rs:225`)을 은퇴 루프 앞으로 이동해야 한다.

2. 또는 README를 수정해 사용자 공급 패키지 미지원을 명시하고, 예약 id 목록을 문서화한다.

---

### MEDIUM

#### M-1. 번들 디렉터리명을 pet id로 검증하지 않아, 잘못 이름 붙인 패키지가 조용히 설치되고 영원히 로드되지 않음

**위치**: `src-tauri/src/lib.rs:231-254`

```rust
let Ok(package_id) = entry.file_name().into_string() else { continue };
...
let destination = installed_pets.join(&package_id);   // ← is_valid_pet_id 검사 없음
```

`is_valid_pet_id`(`domain.rs:4-13`)는 소문자·숫자·하이픈만 허용하고 밑줄과 대문자를 거부한다. 번들에 `Tabby`, `my_pet`, `shiba_inu` 같은 폴더를 두면:

1. `should_preserve_installed` → `load()` → `validate_id` 실패 → `false` 반환 → **설치 진행**
2. 설치는 성공하지만 이후 `load()` / `list()`에서 모두 거부 → 피커에 안 뜨고 선택도 불가
3. 오류 로그 없음

이 변경의 목적이 "폴더만 추가하면 새 펫이 나간다"인데, 정확히 그 작업에서 가장 흔한 실수가 무증상으로 넘어간다. 최소한 스킵 + 로그가 필요하다:

```rust
if !crate::domain::is_valid_pet_id(&package_id) {
    eprintln!("skipping bundled pet with invalid id: {package_id}");
    continue;
}
```

#### M-2. 번들 매니페스트 파싱 실패가 조용한 스킵이라, 릴리스에서 펫이 통째로 빠져도 아무 신호가 없음

**위치**: `src-tauri/src/lib.rs:235-237`, `src-tauri/src/store/pets.rs:167-181`

```rust
let Some(bundled) = store::bundled_manifest_info(&source.join("manifest.json")) else {
    continue;
};
```

`bundled_manifest_info`는 이제 `display_name`을 **필수**로 요구한다(`pets.rs:172`, `#[serde(default)]` 없음). 번들 매니페스트에 `displayName`이 빠지거나 JSON이 깨지면 `None` → 그 펫은 설치되지 않고 로그도 남지 않는다.

M-1과 달리 이건 앱 자체 리소스라 CI에서 잡아야 하는데, 현재 그 방어가 없다(M-5 참조). 최소한 `eprintln!`을 추가하고, 가능하면 M-5의 테스트로 승격하는 편이 낫다.

#### M-3. 설치가 all-or-nothing이라, 삭제 1건 실패로 번들 펫 전부가 미설치됨

**위치**: `src-tauri/src/lib.rs:222`, `:226`, `:243`, `:247`

`fs::remove_dir_all(&stale)?` / `fs::read_dir(&bundled_pets)?`가 모두 `?`로 전파되고, 호출부는 로그만 찍고 계속 진행한다(`lib.rs:41-44`):

```rust
if let Err(error) = install_bundled_pet_packages(...) {
    eprintln!("failed to install bundled pet packages: {error}");
}
```

결과적으로 다음 시나리오가 성립한다.

1. Windows에서 `pets/cat` 하위 PNG에 파일 잠금이 걸림 → `remove_dir_all` 실패
2. `?`로 즉시 반환 → **`tabby`도 `corgi`도 설치되지 않음**
3. settings 마이그레이션은 이미 `cat` → `tabby`로 값을 바꿔 놓음(`settings.rs:148`)
4. 사용자는 존재하지 않는 `tabby`를 가리킨 채 `petPackageError` 상태로 앱을 마주함

`list()`가 "one bad directory must not empty the picker"(`pets.rs:116-117`) 원칙을 이미 채택했으므로, 설치 루프도 같은 원칙을 따르는 게 일관적이다. 은퇴 삭제는 `let _ = fs::remove_dir_all(...)`로 best-effort 처리하고, 개별 패키지 실패는 로그 후 `continue`를 권한다.

#### M-4. 권위 문서 3곳이 `cat` 기준으로 남아 있음

**위치**: `CLAUDE.md:35`, `CLAUDE.md:52`, `README.md:34`, `README.md:37`, `docs/ui-contract.md:196`

| 파일 | 현재 | 실제 |
|---|---|---|
| `CLAUDE.md:35` | "regenerate bundled cat/corgi pet packages" | `tabby`/`corgi` |
| `CLAUDE.md:52` | "Bundled cat/corgi pet packages install into..." | `tabby`/`corgi` |
| `README.md:34` | "rebuild bundled cat/corgi packages" | `tabby`/`corgi` |
| `README.md:37` | "bundles generated cat and corgi packages" | `tabby`/`corgi` (+ H-1 충돌) |
| `docs/ui-contract.md:196` | "설정 항목 (MVP): 주 provider 선택, 말풍선 켜기/끄기, 로그인 시 시작." | **펫 선택 항목 누락** |

`CLAUDE.md`가 `docs/ui-contract.md`를 "presentation contracts의 source of truth"로 지정하고 있으므로, 새 설정 항목이 추가된 이상 `:196` 갱신은 선택이 아니다. (`Appearance` 테마 항목도 이미 빠져 있어 이번에 함께 정리하는 게 좋다 — 기존 드리프트.)

#### M-5. `manifest.test.ts`가 펫 2개를 하드코딩해, "폴더만 추가" 설계의 검증 구멍이 됨

**위치**: `src/lib/assets/manifest.test.ts:2-3`, `:22-26`

```ts
import tabbyManifest from '../../../src-tauri/resources/pets/tabby/manifest.json';
import corgiManifest from '../../../src-tauri/resources/pets/corgi/manifest.json';
...
it.each([
  ['tabby', tabbyManifest],
  ['corgi', corgiManifest],
])
```

네이티브는 디렉터리 순회로 바뀌었는데 테스트는 여전히 명시적 2건이다. 세 번째 펫을 추가하면 매니페스트 검증 없이 릴리스에 들어가고, 깨져 있으면 M-2 경로로 조용히 사라진다. `import.meta.glob('../../../src-tauri/resources/pets/*/manifest.json', { eager: true })` 등으로 디렉터리를 열거하도록 바꾸면 설계 의도와 테스트가 다시 맞물린다.

---

### LOW

#### L-1. `changeSettings`의 overlay 분기는 도달 불가 (dead code)

**위치**: `src/App.svelte:528-530`

```ts
if (petChanged && windowLabel === 'overlay') {
  await loadPetPackage();
}
```

`changeSettings`의 호출 지점은 `:605`(SettingsPanel)와 `:623`(주 provider 버튼) 두 곳뿐이고, 둘 다 `{:else if windowLabel === 'panel'}` 블록(`:593`) 안에 있다. 게다가 `NativeCommand::UpdateSettings`는 panel에만 인가된다(`window/mod.rs:96`). 커버리지 리포트도 이를 확인해 준다 — App.svelte 미커버 라인에 `529-530`이 그대로 잡힌다.

실제 overlay 갱신은 `:296-300`의 settings 리스너가 담당하며 이쪽은 정상 동작한다. `:528-530`은 제거 가능하다. (기존 `primaryChanged` 시절부터 있던 죽은 분기를 그대로 옮긴 것이므로 이번 변경이 만든 문제는 아니다.)

#### L-2. `should_preserve_installed`의 `displayName` 검사가 `version` 검사보다 앞서, 향후 개명 시 강제 재설치 경로가 막힘

**위치**: `src-tauri/src/store/pets.rs:140-149`

```rust
if package.manifest.display_name != bundled.display_name {
    return true;    // ← early return
}
if package.manifest.version < bundled.version {
    return false;   // ← displayName이 바뀌면 도달 불가
}
```

`version`은 "번들 아트가 갱신됐을 때 강제 재설치"를 위해 존재하는데, `displayName`이 함께 바뀌면 그 검사에 도달하지 못한다. 이번 `cat` → `tabby`는 **id**까지 바뀌어 `RETIRED_PET_IDS`라는 별도 우회로로 처리됐지만, 다음에 id는 유지한 채 `displayName`만 바꾸는 경우(예: `"Tabby"` → `"Tabby Cat"`)에는 `version`을 올려도 기존 설치본이 영원히 갱신되지 않는다.

현재 재현되는 결함은 아니다. 순서를 뒤집거나(`version` 먼저), displayName 불일치를 `version >= bundled.version`일 때만 커스터마이즈로 간주하면 해소된다. 관련 테스트도 없다.

#### L-3. V1/V2 마이그레이션 경로만 write 전 validate를 생략

**위치**: `src-tauri/src/store/settings.rs:162-191` vs `:193-205`

Legacy 경로는 `validate(&migrated)?` 후 write하지만(`:202-203`), V1/V2는 검증 없이 write하고 `load_locked()`로 재진입한다. 재진입 시 `validate`가 실패하면 어느 하위 스키마로도 파싱되지 않아 `quarantine()` + 전체 기본값 리셋으로 떨어진다(`:206-207`) — 위치·알림 설정까지 함께 날아간다.

`save()`가 항상 검증하므로 앱이 스스로 쓴 파일에서는 발생하지 않고, 손상되거나 손으로 편집된 파일에서만 성립한다. 영향은 작지만 세 경로의 처리 방식이 다른 점은 정리할 가치가 있다.

#### L-4. `petOptions`가 startup 1회만 로드됨

**위치**: `src/App.svelte:368-373`

panel 윈도우는 `show_panel`/`hide_panel`로 표시만 토글되고 remount되지 않으므로, 앱 실행 중 설치된 패키지는 재시작 전까지 피커에 반영되지 않는다. 현재 앱 내 패키지 추가 UI가 없어 실사용 영향은 없다.

#### L-5. `displayName` 길이 상한 없음

**위치**: `src-tauri/src/store/pets.rs:38`

매니페스트 전체가 64KB로 제한되므로(`pets.rs:79-84`) 무한하지는 않지만, 수 KB 길이의 `displayName`이 피커와 `ResizeObserver` 기반 패널 자동 리사이즈(`App.svelte:407-428`)를 깨뜨릴 수 있다. XSS는 없다.

---

## Validation Results

| Check | Result | 비고 |
|---|---|---|
| Type check (`pnpm check`) | **Pass** | svelte-check: 0 errors, 0 warnings |
| Lint (`pnpm lint`) | **Pass** | eslint + prettier --check 모두 통과 |
| Tests (`pnpm vitest run`) | **Pass** | 23 files / 230 tests 전부 통과 |
| Coverage | **Pass** | Stmts 95.29 / Branch 90.18 / Funcs 91.91 / Lines 95.29 (게이트 80) |
| Build (`pnpm build`) | **Pass** | vite 빌드 성공, 7.80s |
| Rust (`cargo test --all-features`) | **Skipped** | 이 WSL 환경에 `pkg-config` / WebKitGTK 3.0 미설치로 `gdk-sys` 빌드 실패. **네이티브 변경분이 이번 변경의 절반 이상이므로 CI 또는 의존성 설치 환경에서 반드시 재확인 필요** |
| Rust lint (`cargo clippy`) | **Skipped** | 동일 사유 |

> Rust 테스트를 돌리지 못했으므로, `store::tests`(신규 5건)와 `lib::tests`(신규 2건 + 수정 4건)의 실제 통과 여부는 이 리뷰에서 확인되지 않았다.

---

## Files Reviewed

**네이티브 (Rust)**

| 파일 | 변경 |
|---|---|
| `src-tauri/src/lib.rs` | Modified — 번들 디렉터리 순회 설치, `RETIRED_PET_IDS`, 테스트 픽스처 헬퍼 |
| `src-tauri/src/store/pets.rs` | Modified — `PetSummary`, `BundledPetInfo`, `list()`, `bundled_manifest_info()` |
| `src-tauri/src/store/settings.rs` | Modified — 기본값 `tabby`, `cat`/`idle` 값 마이그레이션, 재진입 로드 |
| `src-tauri/src/store/mod.rs` | Modified — re-export 갱신 |
| `src-tauri/src/refresh/ipc.rs` | Modified — `list_pet_packages` 커맨드 |
| `src-tauri/src/window/mod.rs` | Modified — `NativeCommand::ListPetPackages` + panel 인가 |
| `src-tauri/src/store/tests.rs` | Modified — 마이그레이션 2건 + 열거 3건 추가 |
| `src-tauri/src/window/tests.rs` | Modified — 인가 검증 2줄 추가 |

**렌더러 (TypeScript / Svelte)**

| 파일 | 변경 |
|---|---|
| `src/App.svelte` | Modified — `PROVIDER_PET` 제거, `loadPetOptions`, `petChanged` 전환 |
| `src/lib/api/gateway.ts` | Modified — `PetSummaryModel`, `listPetPackages()` |
| `src/lib/api/fixtureGateway.ts` | Modified — 픽스처 메서드 추가 |
| `src/lib/components/SettingsPanel.svelte` | Modified — Pet `<select>` + 폴백 `petOptions` |
| `src/lib/state/presentation.ts` | Modified — `selectedPetId` 투영 추가 |
| `src/lib/stores/settings.ts` | Modified — `SettingsState.selectedPetId` |
| `src/App.test.ts` | Modified — 신규 2건 포함 |
| `src/lib/components/SettingsPanel.test.ts` | Modified |
| `src/lib/state/presentation.test.ts` | Modified |
| `src/lib/stores/settings.test.ts` | Modified |
| `src/lib/assets/manifest.test.ts` | Modified — import 경로 `cat` → `tabby` |

**리소스 / 스크립트**

| 파일 | 변경 |
|---|---|
| `src-tauri/resources/pets/cat/` → `tabby/` | Renamed — manifest.json + 프레임 16개 |
| `scripts/build-pet-packages.py` | Modified — `PET_SOURCES = {"tabby": "cat", ...}` |
| `.claude/PRPs/plans/completed/user-selectable-pet.plan.md` | Added |

---

## 권장 조치 순서

1. **H-1** — README와 코드 중 하나를 맞춘다 (코드 보수화 권장, 수정량 ~10줄)
2. **M-3** — 설치 루프를 best-effort로 전환 (`?` → 로그 + `continue`)
3. **M-1 / M-2** — 번들 id 검증 + 스킵 로그 추가
4. **M-4** — `CLAUDE.md`, `README.md`, `docs/ui-contract.md:196` 갱신
5. **M-5** — `manifest.test.ts`를 디렉터리 열거 방식으로 전환
6. **L-1** — 죽은 분기 제거
7. **L-2** — `should_preserve_installed` 검사 순서 정리 + 회귀 테스트
8. 의존성이 설치된 환경에서 `cargo test --all-features` 및 `cargo clippy -- -D warnings` 재확인
