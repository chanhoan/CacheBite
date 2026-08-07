# CacheBite UI 상태 계약 (UI Contract)

이 문서는 CacheBite 펫 UI의 **구현 가능한 상태 계약**을 정의한다.
시각 디자인(색상 값, 폰트, 정확한 픽셀 치수)은 다루지 않는다.
[architecture.md](architecture.md)의 정규화 스냅샷 모델과 새로고침/실패 정책을 전제로 한다.

## 0. 핵심 결정 요약

| 결정 | 내용 |
| --- | --- |
| 펫 상태 기준 | **주 provider 선택제.** 펫(오버레이, 말풍선, 무드)은 사용자가 선택한 주 provider 하나만 반영한다. 나머지 provider는 클릭 패널에서 열람한다. |
| 상태 구조 | **이중 축.** 5시간 창과 주간 창이 각각 독립적인 심각도를 가지며, 펫 오버레이에 둘 다 상시 표시한다. |
| 조합 폭발 통제 | GIF 선택과 말풍선 톤은 두 심각도의 최댓값인 **파생 무드(pet_mood)** 하나만 사용한다. |
| MVP 표현 | GIF는 `idle` 하나만 필수. 사용량 상태는 UI 오버레이(분할 링 + 배지)로 표현한다. 상태별 GIF는 이후 버전의 선택적 에셋이다. |

## 1. 상태 모델

### 1.1 계층 구조

펫의 표시 상태는 세 값의 조합으로 완전히 결정된다.

```text
PetUiState
  system:  auth_required | unavailable | error | offline | loading | active
  stale:   boolean                       (active일 때만 의미 있음)
  session_sev: ok | warn | critical | exhausted | unknown   (5시간 창)
  weekly_sev:  ok | warn | critical | exhausted | unknown   (주간 창)
  pet_mood:    ok | warn | critical | exhausted             (파생값)
```

- `system`이 `active`가 아니면 심각도 링 대신 시스템 배지를 표시한다.
- `session_sev`, `weekly_sev`는 주 provider 스냅샷에서만 산출한다.
- `pet_mood = max(session_sev, weekly_sev)` (unknown은 제외, 둘 다 unknown이면 `ok`).

### 1.2 시스템 상태 목록과 의미

우선순위가 높은 순서대로 나열한다. 조건이 여럿 겹치면 위의 것이 이긴다.

| 상태 | 의미 | 진입 조건 |
| --- | --- | --- |
| `auth_required` | 주 provider 자격 증명이 없거나 무효 | 수집기가 `CREDS_MISSING` 보고 |
| `unavailable` | 주 provider CLI 미설치 등 수집 경로 부재 | 수집기가 `CLI_MISSING` 보고 (오류가 아님 — 아키텍처 원칙) |
| `error` | 스냅샷 없음 + 마지막 실패가 provider/파싱/내부 오류 | 스냅샷 만료 또는 부재 상태에서 `FETCH_FAIL(class != network)` |
| `offline` | 스냅샷 없음 + 마지막 실패가 네트워크 계층 | 스냅샷 만료 또는 부재 상태에서 `FETCH_FAIL(class == network)` |
| `loading` | 최초 조회 진행 중, 캐시 스냅샷 없음 | 앱 시작 직후에만. 백그라운드 갱신은 loading으로 전환하지 않는다 |
| `active` | 표시 가능한 스냅샷 보유 | `FETCH_OK` 또는 유효한 캐시 스냅샷 존재 |

`stale`은 독립 상태가 아니라 `active`의 데코레이터 플래그다:

- `fresh`: 스냅샷 나이 ≤ `FRESH_MAX_AGE_MIN`
- `stale`: `FRESH_MAX_AGE_MIN` < 나이 ≤ `SNAPSHOT_TTL_MIN` — 데이터는 계속 표시하되 캡처 시각을 노출
- 만료: 나이 > `SNAPSHOT_TTL_MIN` — 스냅샷을 버리고 마지막 실패 원인에 따라 `error`/`offline`으로 강등

### 1.3 사용량 심각도 (창별, 이중 축)

`used_percent`(0–100 클램프)만으로 결정되는 순수 함수다.

| 심각도 | 범위 | 의미 |
| --- | --- | --- |
| `ok` | 0 ≤ p < 70 | 여유 |
| `warn` | 70 ≤ p < 90 | 주의 — 소진이 가시권 |
| `critical` | 90 ≤ p < 100 | 임박 — 곧 차단됨 |
| `exhausted` | p ≥ 100 | 소진 — 리셋 대기 |
| `unknown` | 창 데이터 없음 (`session`/`weekly`가 optional) | 중립 표시, 무드 계산에서 제외 |

히스테리시스는 두지 않는다. 창 내 사용량은 단조 증가하고, 하락은 곧 리셋이므로 플래핑이 구조적으로 발생하지 않는다.

## 2. 상태 전환 기준

### 2.1 창별 심각도 전환

`SNAPSHOT_UPDATED` 이벤트마다 심각도를 재계산한다. 전환 자체는 순수 함수이고, **레벨이 변한 순간에만** 아래 이벤트를 방출한다.

| 방향 | 조건 | 방출 이벤트 |
| --- | --- | --- |
| 상승 (ok→warn, warn→critical, critical→exhausted, 건너뛰기 포함) | 새 p가 상위 구간 진입 | `SEV_RAISED(window, new_sev)` |
| 하락 (임의 레벨 → 하위 레벨) | 새 p가 하위 구간, 즉 창 리셋 발생 | `WINDOW_RESET(window)` |
| `resets_at` 도달 | 현재 시각 ≥ `resets_at` (스냅샷 갱신 전이라도) | `WINDOW_RESET(window)` — 표시만 낙관적으로 `ok`/`unknown` 처리, 다음 조회로 확정 |

### 2.2 시스템 상태 전환표

이벤트 정의:

| 이벤트 | 발생원 |
| --- | --- |
| `APP_START` | 앱 기동 |
| `FETCH_OK(snapshot)` | 수집 성공 |
| `FETCH_FAIL(class)` | 수집 실패. `class: network \| provider \| parse \| internal` |
| `CREDS_MISSING` | 자격 증명 브로커가 자격 증명 부재/무효 보고 |
| `CLI_MISSING` | 수집 경로 자체가 없음 (CLI 미설치 등) |
| `SNAPSHOT_EXPIRED` | 보유 스냅샷 나이 > `SNAPSHOT_TTL_MIN` |
| `MANUAL_REFRESH` | 사용자 새로고침 (디바운스 적용) |
| `PRIMARY_SWITCHED` | 주 provider 변경 |

전환표 (행 = 현재 상태, 첫 매치 우선):

| 현재 \ 이벤트 | FETCH_OK | FETCH_FAIL(network) | FETCH_FAIL(기타) | CREDS_MISSING | CLI_MISSING | SNAPSHOT_EXPIRED |
| --- | --- | --- | --- | --- | --- | --- |
| `loading` | `active(fresh)` | `offline` | `error` | `auth_required` | `unavailable` | — |
| `active` | `active(fresh)` | `active` 유지 (stale은 나이로 결정) | `active` 유지 | `auth_required` | `unavailable` | 마지막 실패 class가 network면 `offline`, 아니면 `error` |
| `offline` | `active(fresh)` | `offline` | `error` | `auth_required` | `unavailable` | — |
| `error` | `active(fresh)` | `offline` | `error` | `auth_required` | `unavailable` | — |
| `auth_required` | `active(fresh)` | `auth_required` 유지 | `auth_required` 유지 | `auth_required` | `unavailable` | — |
| `unavailable` | `active(fresh)` | `unavailable` 유지 | `unavailable` 유지 | `auth_required` | `unavailable` | — |

부가 규칙:

- `APP_START`: 유효한 캐시 스냅샷이 있으면 `active`(stale 여부는 나이로), 없으면 `loading` 후 즉시 조회.
- `MANUAL_REFRESH`: 상태를 바꾸지 않고 조회만 촉발한다. 결과는 위 표의 `FETCH_*`로 처리된다.
- `PRIMARY_SWITCHED`: 대상 provider의 독립 상태(아키텍처의 provider 독립 원칙)로 `PetUiState` 전체를 재도출한다. 재조회는 불필요하다.
- 실패 반복 시 재시도 간격은 아키텍처의 지수 백오프 정책을 따른다. UI는 백오프를 표시하지 않고 상태만 반영한다.

### 2.3 수집기 계약 확장 (UI-facing DTO)

이 계약이 성립하려면 정규화 서비스가 renderer에 다음을 추가로 제공해야 한다.
(스냅샷 원본 모델은 변경하지 않고, UI 전달 DTO에만 얹는다.)

```text
ProviderUiSnapshot = ProviderUsageSnapshot +
  unavailable_reason: optional (not_installed | not_signed_in)
  failure_class: optional (network | provider | parse | internal)
```

## 3. 5시간/주간 표시 우선순위

| 표면 | 규칙 |
| --- | --- |
| 오버레이 링 | 항상 둘 다 표시 (이중 축의 본질). 상반원 = 5시간, 하반원 = 주간 |
| 클릭 패널 | 둘 다 표시. 5시간 게이지를 위에 배치 |
| 한 줄 표면 (말풍선, 이후 버전의 OS 알림) | 심각도가 높은 창 우선 → 동률이면 **5시간 창 우선** (더 빨리 변하고 즉시 행동 가능하므로) |

## 4. 오버레이 명세 (MVP)

### 4.1 분할 링

- idle GIF를 둘러싸는 원형 링. **상반원 = 5시간 창, 하반원 = 주간 창.**
- 각 반원은 `used_percent`만큼 채워지고, 색은 해당 창의 심각도 시맨틱 토큰을 따른다:
  `sev.ok` / `sev.warn` / `sev.critical` / `sev.exhausted` (실제 색상 값은 시각 디자인 단계에서 결정).
- `unknown` 창은 중립 토큰(`sev.unknown`)의 비채움 트랙만 표시.
- `stale`이면 링 색을 유지한 채 불투명도를 낮춘다 (시맨틱: `overlay.stale-dim`).

확정 시각 토큰은 라이트/다크 순서로 `ok` `#22c55e`/`#4ade80`, `warn`
`#f59e0b`/`#fbbf24`, `critical` `#f97316`/`#fb923c`, `exhausted`
`#dc2626`/`#f87171`, `unknown` `#c3c8ce`/`#4b5563`이며,
`overlay.stale-dim`은 `0.42`다. 실제 사용처는 `src/lib/styles/tokens.css`의
`--sev-*`와 `--overlay-stale-dim` 변수를 단일 원본으로 삼는다.

### 4.2 시스템 배지

`system != active`일 때 링을 숨기고 펫 모서리에 단일 배지를 표시한다.

| 상태 | 배지 시맨틱 | 클릭 패널 안내 문구 (예시) |
| --- | --- | --- |
| `auth_required` | 자물쇠 | "Claude CLI에 로그인하세요: `claude login`" |
| `unavailable` | 슬래시 원 | "Codex CLI가 설치되어 있지 않습니다" |
| `error` | 경고 삼각형 | "사용량을 가져오지 못했습니다. 잠시 후 재시도합니다" |
| `offline` | 구름/오프라인 | "네트워크에 연결할 수 없습니다" |
| `loading` | 스피너 | "사용량을 불러오는 중" |

### 4.3 포인터 동작

- 이동 거리 < `DRAG_THRESHOLD_PX`인 더블클릭 → 패널 토글(`toggle_panel`). 단일 클릭은 아무것도 열지 않는다.
- 그 이상 이동 → 드래그. 드래그 중에는 상호작용 애니메이션과 말풍선을 중지한다(아키텍처의 드래그 정책).
- 우클릭 → 네이티브 컨텍스트 메뉴(`show_pet_menu`). 항목은 패널 표시/숨기기, 펫 숨기기(전역 단축키와 동일한 래치), CacheBite 종료. 렌더러는 팝업 요청만 보내고 항목 동작은 전부 Rust에서 처리한다. 드래그 제스처는 주 버튼으로만 열린다.
- 전체화면 앱 감지로 펫이 숨겨진 동안에도 수집과 상태 계산은 계속된다. 표시만 중단한다.

### 4.4 링 모드 (큰 원 / 작은 원)

`ring_mode: 'single' | 'double'`은 `Settings`에 저장되는 **순수 표시 선호**다(기본 `single`).
수집, 새로고침 일정, 알림 라우팅, 펫 무드 산출 중 어느 것에도 영향을 주지 않는다.

| 항목 | 규칙 |
| --- | --- |
| 큰 원 | **항상** `primary_provider`. 연결 상태에 따른 교체(스왑)나 자동 강등은 없다 |
| 큰 원 중앙 | 사용자가 고른 펫. 펫은 provider를 뜻하지 않으며 선택도 독립적이다 |
| 작은 원 | `ring_mode === 'double'`일 때만 렌더 |
| 작은 원 provider | `secondaryProvider(primary)` — primary의 반대편이 자동 배정된다. 사용자가 손으로 지정하는 UI는 없다 |
| 작은 원 중앙 | 해당 provider의 로고(`ProviderLogo.svelte`) |

작은 원 기하: 큰 원 한 변의 **40%**, 우하단에 약 ⅓ 겹치도록 배치하고 오버레이 상자 밖으로
우 4.3% · 하 5.5% 삐져나온다(오버레이 창 240px, 링 128px이므로 여백 안에 들어간다).
아크는 §4.1의 분할 링을 **스타일 변경 없이 그대로** 쓴다 — 획 두께, 트랙 불투명도, 심각도 색이
큰 원과 동일해야 두 링이 같은 것으로 읽힌다. 유일한 예외는 `5H`/`WK` 라벨을 숨기는 것으로,
이 크기에서는 약 4px로 렌더되고 좌표가 viewBox 밖이라 퍽을 벗어나 큰 원 위로 삐져나온다.

링 뒤에는 퍽(원판, `--satellite-puck`)을 깔아 큰 원과 겹치는 구간에서 별개 물체로 읽히게 한다.
퍽 색은 **테마와 무관하게 고정**이다 — 두 provider 마크가 어두운 바탕을 전제로 그려졌고
Codex 글리프가 knockout이라 라이트 모드에서 흰 퍽을 쓰면 프롬프트 글리프가 하얗게 뚫린다.

**링별 상태는 독립적이다.** 한쪽의 `stale`이나 비-`active` 상태가 다른 쪽을 지우지 않는다
(§5 provider 독립성). 미연결(`auth_required` / `unavailable`) provider도 작은 원을 유지하고
§4.2 배지로 표시한다 — 강등하지 않는 이유는 설정이 화면을 그대로 설명해야 하기 때문이다.
`double`을 골랐는데 원이 하나만 보이면 설정이 거짓말을 하는 셈이 된다.

접근명은 의도적으로 비대칭이다. 큰 원은 `"Provider usage: …"`로 남고 작은 원만
`"Codex usage: …"`처럼 provider명을 갖는다. 큰 원은 primary 설정이 가리키는 것이고 패널이
이미 이름을 붙여 주지만, 작은 원은 "다른 하나"라서 스스로를 밝혀야 한다.

포인터 표면은 큰 원 ∪ 작은 원이다. 작은 원 위에서도 드래그·더블클릭·우클릭이 §4.3과 동일하게
동작해야 하며, 작은 원의 표면은 `aria-hidden`이고 역할이 없다 — 이미 이름을 가진 컨트롤의
중복 타격 영역일 뿐이고, 같은 이름의 두 번째 컨트롤은 접근성 트리를 모호하게 만든다.
키보드 경로(`Enter`/`Space`)는 큰 원의 표면이 계속 담당한다.

### 4.5 궤도 워커 (primary provider 마크)

큰 원의 바깥 가장자리를 도는 provider 마크다. **두 링 모드 모두에서 항상 렌더된다** —
`ring_mode`에 반응하지 않으며, 반응하는 것은 궤도의 모양뿐이다.

존재 이유는 정보 비대칭 해소다. 펫은 사용자가 고르는 장식이라 provider를 뜻하지 않고,
작은 원만 로고를 달고 있어서 화면상 브랜드가 붙은 유일한 요소가 **secondary**가 된다.
워커가 primary의 정체를 다시 위로 올린다.

| 항목 | 규칙 |
| --- | --- |
| provider | **항상** `primary_provider`. 작은 원과 짝을 이루는 반대편이며 교체되지 않는다 |
| 기하 | `orbitPath.ts` 단일 출처. 마크 한 변 `WALKER_SIZE` = 오버레이의 14%, 중심선은 도는 원의 바깥 획에 발이 닿도록 놓인다 |
| 궤도 | `single` → 큰 원 한 바퀴. `double` → 두 원의 **합집합 아웃라인**(큰 원 → 교점에서 건너뜀 → 작은 원의 노출 구간 → 복귀) |
| 회전 | `offset-rotate: auto`. 아래쪽에서는 거꾸로 서며, 이는 바깥에서 본 구(球) 위의 보행자로 의도된 것이다 |
| 방향 | 실행마다 무작위(`orbitDirection(roll)` 순수 함수 + 호출부 난수). overlay 창에서만 난수를 소비한다 |
| 주기 | 26s linear infinite |
| 접근성 | `role="img"`, 이름 `"Primary provider: Claude"`. 패널이 이미 싣고 있는 정보의 장식적 중복이므로 `pointer-events: none` — 아래 드래그 표면을 절대 가로채지 않는다 |

**오버레이 크기 상한은 이제 펫이 아니라 워커의 도달 범위가 결정한다.** `OVERLAY_BOUNDS_FACTOR`는
중심에서 가장 멀리 나가는 요소까지의 거리를 2배한 값(≈1.39)이고, `overlaySize`는
`OVERLAY_WINDOW_PX / OVERLAY_BOUNDS_FACTOR`로 클램프된다. 즉 240px 창에서 펫은 최대 약 172px다.
**두 링 모드에 동일하게 적용한다** — 모드 전환이 펫 크기를 바꾸면 표시 설정이 아니라 레이아웃
변경이 되기 때문이다. 상한을 정하는 것은 큰 원이 아니라 작은 원 궤도의 바깥쪽이다(작은 원이
이미 우하단으로 치우쳐 있다).

말풍선이 떠 있는 동안 펫은 152px로 좁아진다. 이 클램프는 **CSS가 아니라 `overlaySize`에**
있어야 한다. 워커의 `offset-path`는 오버레이 상자에 대한 절대 픽셀이고 CSS `path()`는 퍼센트를
받지 않으므로, 스타일시트만 아는 폭이 생기면 마크가 더 이상 존재하지 않는 원을 돌게 된다.

**감축 모션**: `prefers-reduced-motion: reduce`에서 애니메이션만 정지하고 마크는 궤도 위에 남는다.
**미지원 엔진**: `offset-path`가 없으면(WebKit 16 / WebKitGTK 2.38 미만) 마크는 멈추는 게 아니라
`top:0; left:0`에 박제되어 링 밖 모서리에 남는다. `@supports not (offset-path: …)`로 숨긴다 —
패널이 이미 담고 있는 정보라 사라지는 쪽이 정직한 퇴화다.

**테마 불변 토큰**: `--logo-claude`, `--logo-codex-from`, `--logo-codex-to`, `--satellite-puck`은
라이트/다크에서 **동일하다**. 두 provider 마크가 어두운 바탕을 전제로 그려졌고 Codex 글리프가
knockout이라, 테마별로 흔들면 라이트 모드에서 프롬프트 글리프가 하얗게 뚫린다.

**로고 아트 출처**: `docs/assets/provider-logos/`(`claude-code.png`, `codex.png`)는 **추적 대상
원본이며 런타임에 로드되지 않는다**. `ProviderLogo.svelte`가 좌표를 벡터로 재현해 인라인 SVG로
들고 있고, PNG는 그 좌표가 어디서 나왔는지 되짚기 위한 근거일 뿐이다. 펫 아트(`docs/UI-plan/`)가
`scripts/build-pet-packages.py`를 거쳐 패키지로 번들되는 것과 달리 이쪽은 빌드 경로가 없다 —
자산 프로토콜 스코프(`$APPDATA/pets/*/frames/*.png`)에도 해당하지 않으므로, 이미지를 직접
참조하도록 바꾸면 로드에 실패한다.

## 5. 클릭 패널 정보 구조

패널은 펫 근처에 앵커되고 디스플레이 경계 안으로 클램프된다. 3단 구조:

```text
┌──────────────────────────────────┐
│ 헤더                        (✕)   │  ✕ = 패널 닫기 (절대 위치, 흐름 밖)
│  [Claude ★] [Codex]  탭           │  ★ = 주 provider 표시
│  plan_type (있을 때만)             │
├──────────────────────────────────┤
│ 본문 (선택된 탭의 provider)         │
│  5시간 게이지  ▓▓▓▓░░ 68%          │
│    리셋까지 1시간 12분              │
│  주간 게이지   ▓▓░░░░ 31%          │
│    리셋: 월요일 09:00              │
│  캡처 시각 · source 라벨            │
│  상태 줄: stale/오류/인증 안내       │
├──────────────────────────────────┤
│ 푸터                              │
│  [지금 새로고침] [주 provider로 설정] │
│  [설정] [종료]                     │  종료 = CacheBite 프로세스 종료
└──────────────────────────────────┘
```

규칙:

- 두 provider 모두 **항상** 탭으로 열람 가능하다. 한쪽의 `auth_required`/`unavailable`이 다른 쪽 표시를 막지 않는다 (provider 독립 원칙).
- 탭별 본문은 해당 provider의 `PetUiState` 파생 규칙을 그대로 재사용한다 (주 provider 여부와 무관하게 동일한 도출 함수).
- "지금 새로고침"은 아키텍처의 디바운스 정책을 따르고, 디바운스 중에는 비활성화 표시한다.
- 패널 자체의 로딩 스켈레톤은 `loading`일 때만 사용한다. 백그라운드 갱신 중에는 기존 값 유지.
- 설정 항목: 테마(시스템/라이트/다크), 네이티브 알림, 보조 provider 알림, 주 provider 선택, **펫 선택**, 말풍선 켜기/끄기, 로그인 시 시작.
- 주 provider 선택은 게이지에 표시할 사용량 출처만 정한다. 펫 선택은 그와 독립된 항목이며, 한쪽을 바꿔도 다른 쪽은 유지된다.
- 펫 목록은 설치된 패키지를 네이티브가 열거해 채운다(`list_pet_packages`). 열거가 실패하면 현재 선택된 펫만 표시해 활성 펫이 화면에서 사라지지 않게 한다.
- 패널은 **지원되는 플랫폼에서 항상 위**로 떠 있고 외부 클릭으로 닫히지 않는다. always-on-top을 거부하는 컴포지터에서는 패널이 뒤로 밀릴 수 있다. 닫기 수단은 두 가지이며 둘 다 명시적 제스처다: 헤더 우측 상단의 `✕`, 그리고 펫 더블클릭. 둘 다 패널만 숨긴다(프로세스는 계속 실행된다). 포커스 상실이나 외부 클릭으로는 닫히지 않는다.
- `✕`는 레이아웃 흐름 밖의 절대 위치 요소다. 추가·제거가 헤더·본문·푸터의 치수나 `resize_panel`에 보고되는 측정 높이를 바꾸지 않아야 한다.
- **`✕`가 두 번째 탭의 우측 상단을 덮는 것은 의도된 계약이다.** 312px 패널에서 겹침은 14×18px(두 번째 탭 면적의 약 4%)이고, 겹침 영역의 포인터 이벤트는 `✕`가 받는다. 탭 스트립에서 폭을 양보하지 않는 쪽을 선택한 결과다 — `✕`는 어떤 컴포넌트도 밀어내지 않고 그 위에 얹힌다. 겹침 상한은 e2e로 고정하며(`renderer.spec.ts`), 아이콘 크기나 헤더 패딩을 바꿔 이 값을 넘기면 테스트가 실패한다. 겹침은 탭 폭의 약 10%이고, 두 번째 탭은 폭의 89% 이상이 클릭 가능하게 남아야 한다.
- **`✕`는 사용량 화면에만 있다.** Settings 화면(`← Back`으로 복귀)과 시작 로딩·실패 화면에서는 `UsagePanel`이 마운트되지 않아 `✕`가 없다. 시작 실패 화면에서는 창의 작업표시줄 항목이 유일한 닫기 경로다 — 패널 창에 `skipTaskbar`를 설정하지 않는 이유가 여기에도 있다.
- 펫 더블클릭은 **토글**이다. 숨겨져 있으면 열고 포커스를 주며, 보이면 숨긴다. 판단은 네이티브가 패널의 실제 가시성으로 내린다(`panel_toggle`) — 렌더러는 패널 상태 사본을 갖지 않는다. 표시 요청 직후 렌더러 높이 측정을 기다리는 동안(`PANEL_LAYOUT_GRACE`)은 아직 화면에 없어도 "보이는 것"으로 취급한다. 그래야 연속 더블클릭의 두 번째가 방금 건 표시를 취소한다.
- always-on-top을 거부하는 컴포지터에서 패널이 다른 창 뒤로 밀린 경우, 더블클릭 한 번은 그것을 **숨긴다**(전면화가 아니다). 복구는 한 번 더 더블클릭하는 것이며, 이때 표시 경로가 전면화와 포커스를 함께 수행한다. 토글의 예측 가능성을 포커스 기반 분기보다 우선한 결과다.
- 푸터의 "종료"는 CacheBite 프로세스를 종료한다(`app.exit(0)`). 패널 숨김과 혼용하지 않는다.

## 5.1 업데이트 알림 (`UpdateNotice`)

새 릴리스가 있으면 탭 위, `UsagePanel` 앞에 배너가 붙는다. 배너 유무는 패널 높이를 바꾸므로 기존 `ResizeObserver` → `resize_panel` 경로가 그대로 처리한다. `resizePanel`을 직접 호출하지 않는다 — 그 명령은 패널 노출 게이트를 겸하므로 방금 닫은 패널을 다시 열 수 있다.

```text
┌──────────────────────────────────┐
│ ▲ Update available — 0.1.0-b5    │  ← role="status" aria-live="polite"
│   [Install and restart] [Later]  │
├──────────────────────────────────┤
│ [Claude ★] [Codex]  탭            │
│ ...                              │
└──────────────────────────────────┘
```

`updateViewModel(state, dismissedVersion)`가 두 화면(배너와 설정)에 필요한 모든 문자열을 만든다:

| status | headline | detail | primary (action) | dismissible | settingsLine |
|---|---|---|---|---|---|
| `idle` | — | — | — | — | `Not checked yet` |
| `checking` | — | — | — | — | `Checking…` |
| `up_to_date` | — | — | — | — | `Up to date` |
| `available` | `Update available — {version}` | 잘라낸 릴리스 노트 | `Install and restart` (`install`) | 예 | `Update available — {version}` |
| `downloading` | `Downloading update` | `{pct}%`, `total`이 null이면 `Downloading…` | `Install and restart` (`install`, 비활성) | 아니오 | `Downloading…` |
| `installing` | `Installing {version}…` | `CacheBite will restart.` | (`install`, 비활성) | 아니오 | `Installing…` |
| `failed` | `Update failed` | 사유별 한 문장 | `Try again` (**`check`**) | 예 | `Update failed` |

규칙:

- **`primaryAction`이 어느 명령을 보낼지 정한다.** `failed`의 `Try again`은 반드시 `check`다 — 네이티브 `UpdateService::install`은 상태가 `available`이 아니면 즉시 반환하므로, 재시도를 `installUpdate`에 연결하면 버튼이 아무 일도 하지 않는다.
- `visible`은 `idle`/`checking`/`up_to_date`에서 `false`다.
- **해제(dismiss)는 두 종류이며 서로 독립이다.** `available`은 버전으로 키를 잡으므로(`dismissedVersion === version`) 더 새 릴리스는 다시 제안된다. `failed`는 키로 삼을 버전이 없으므로 별도의 세션 플래그(`failureDismissed`)를 쓰고, 상태가 `failed`를 벗어나는 순간 호출자가 지운다. 따라서 재시도가 또 실패하면 배너는 다시 나타난다.
- `downloading`/`installing`은 **항상** 보인다. 어느 해제도 진행 중인 작업을 가릴 수 없다.
- 두 해제 모두 렌더러 세션 상태다. 재시작하면 다시 나타난다. 설정 화면은 언제나 참값을 보여주므로 영구히 숨겨지는 정보는 없다.
- 실패 문장에는 URL·호스트·경로가 들어가지 않는다. 네이티브가 사유를 타입으로 분류하므로 렌더러가 전송 계층 세부를 되풀이할 이유가 없다.
- 설정 화면에는 읽기 전용 `Version` 행과 `Updates` 상태 줄, `Check for updates` 버튼이 있다. 이 버튼만 자동 주기(15분 floor)를 무시한다.
- 오버레이(펫)에는 업데이트 표시가 없다. 말풍선도, OS 알림도 쓰지 않는다.

## 6. GIF 에셋 계약 (상태별)

기존 에셋 계약("첫 구현은 idle 하나 필수")을 유지하면서 선택 상태 키를 표준화한다.

| 상태 키 | 필수 여부 | 재생 시점 |
| --- | --- | --- |
| `idle` | **필수** | 기본. 다른 키가 없을 때의 최종 폴백 |
| `idle_warn` | 선택 | `pet_mood == warn` |
| `idle_critical` | 선택 | `pet_mood == critical` |
| `idle_exhausted` | 선택 | `pet_mood == exhausted` |
| `sleep` | 선택 | `system ∈ {unavailable, offline}` |
| `dragging` | 선택 | 드래그 중. 패키지가 이 키를 선언한 경우에만 요청한다 |

폴백 체인: **요청 상태 키 → `idle`.** 중간 단계 폴백(예: critical→warn)은 두지 않는다 — 제작자가 일부만 만들어도 결과가 예측 가능해야 한다.

`dragging`은 예외적으로 요청 단계에서 걸러진다. 선언되지 않은 `dragging`을 요청하면 드래그가 끝날 때까지 무드 신호가 기본 `idle`로 덮여 사라지므로, 선언되지 않았다면 애초에 요청하지 않고 사용량 기반 키를 유지한다. 폴백 체인 자체는 그대로다.

표에 없는 시스템 상태(`auth_required`, `error`, `loading`)는 항상 `idle`을 요청한다. 이 상태들의 시각 신호는 GIF가 아니라 §4.2의 시스템 배지가 담당한다.

매니페스트는 기존 계약(식별자, 표시 이름, 기본 크기, 애니메이션 소스, 프레임 타이밍) 위에 `states` 맵으로 위 키를 선언한다. 선언되지 않은 키는 존재하지 않는 것으로 취급한다. 이 확장은 수집기/윈도 인터페이스를 변경하지 않는다.

## 7. 말풍선과 알림 규칙

### 7.1 말풍선 (MVP)

말풍선은 **상태 교차 이벤트에서만** 발화한다. 상시 표시 요소가 아니다.

| 트리거 | 예시 문구 톤 |
| --- | --- |
| `SEV_RAISED(window, warn)` | "5시간 창 70% 사용했어요" |
| `SEV_RAISED(window, critical)` | "주간 한도가 거의 다 찼어요" |
| `SEV_RAISED(window, exhausted)` | "5시간 한도 소진 — 1시간 12분 후 리셋" |
| `WINDOW_RESET(window)` | "5시간 창이 리셋됐어요" |
| `auth_required` 진입 | "로그인이 필요해요" |
| `error/offline → active` 복구 | "다시 연결됐어요" |

규칙:

1. 주 provider의 이벤트만 발화한다 (선택제와 일관).
2. 중복 억제 키는 `(provider, window, severity)`. 같은 키는 해당 창이 리셋되기 전까지 재발화하지 않는다.
3. 동시 표시는 최대 1개. 새 이벤트가 오면 기존 말풍선을 교체한다. 교체 우선순위는 §3의 한 줄 표면 규칙을 따른다.
4. `BUBBLE_DISMISS_SEC` 후 자동 소멸. 클릭하면 즉시 소멸하고 패널을 연다.
5. 드래그 중, 전체화면 숨김 중에는 발화하지 않는다. **큐잉 없이 폐기한다** — 뒤늦은 알림은 오정보다.
6. 설정에서 말풍선 전체를 끌 수 있다.

### 7.2 OS 알림 (v1.1 — 규칙만 선정의)

- 대상: `SEV_RAISED(*, critical)`, `SEV_RAISED(*, exhausted)`, `auth_required` 진입만.
- 중복 억제와 주 provider 한정 규칙은 말풍선과 동일.
- 기본값 꺼짐. 설정에서 켠다.

## 8. 구현 상수

모든 수치는 이름 있는 상수로 관리하고, 첫 릴리스에서는 사용자에게 노출하지 않는다.

| 상수 | 기본값 | 근거 |
| --- | --- | --- |
| `SEV_WARN_PCT` | 70 | §1.3 |
| `SEV_CRITICAL_PCT` | 90 | §1.3 |
| `FRESH_MAX_AGE_MIN` | 20 | 폴링 15분 + 유예 5분 |
| `SNAPSHOT_TTL_MIN` | 30 | 아키텍처의 30분 보존 정책 |
| `BUBBLE_DISMISS_SEC` | 8 | §7.1 |
| `DRAG_THRESHOLD_PX` | 4 | 클릭/드래그 판별 |

## 9. MVP와 이후 버전 범위

### MVP (v1.0)

- `idle` GIF 1종 렌더링 (에셋 계약 필수 항목 그대로)
- 이중 축 분할 링 + 시스템 배지 오버레이
- 시스템 상태 6종 + stale 데코레이터, §2 전환표 전부
- 클릭 패널 (양 provider 탭, 게이지, 리셋 시각, 수동 새로고침)
- 말풍선 (§7.1 규칙 전부)
- 주 provider 선택, 드래그 + 위치 복원 (윈도 컨트롤러 담당)

### v1.1+

- 무드별 GIF (`idle_warn`/`idle_critical`/`idle_exhausted`), `sleep`, `dragging` 에셋 지원
- OS 알림 (§7.2)
- 패널 사용량 히스토리 그래프
- 보조 provider 이벤트 알림 옵션

### 비목표 (이 계약의 범위 밖)

- 데스크톱 배회 (펫은 고정 위치에서 애니메이션만)
- 다중 펫 동시 실행
- CacheBite 클라우드 동기화, 계정 시스템
- 토큰 집계 기반 사용량 추정 (provider 계산값만 신뢰 — 아키텍처 원칙)

## 10. 검증 기준

이 계약이 구현되었다고 판정하는 최소 테스트 표면:

- §1.3 심각도 함수: 경계값 (69/70, 89/90, 99/100) 단위 테스트
- §2.2 전환표: 모든 (상태 × 이벤트) 셀에 대한 표 기반 단위 테스트
- 스냅샷 나이 경계 (20분/30분)에서 fresh→stale→만료 전이 테스트
- 말풍선 중복 억제: 같은 심각도 재진입 시 미발화, 리셋 후 재발화 테스트
- GIF 폴백: 선언되지 않은 상태 키 요청 시 `idle` 재생 테스트
- provider 독립성: 한쪽 `auth_required` 상태에서 다른 쪽 패널 탭 정상 표시 테스트
