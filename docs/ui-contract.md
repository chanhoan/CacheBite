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

- 이동 거리 < `DRAG_THRESHOLD_PX`인 클릭(release) → 패널 토글.
- 그 이상 이동 → 드래그. 드래그 중에는 상호작용 애니메이션과 말풍선을 중지한다(아키텍처의 드래그 정책).
- 전체화면 앱 감지로 펫이 숨겨진 동안에도 수집과 상태 계산은 계속된다. 표시만 중단한다.

## 5. 클릭 패널 정보 구조

패널은 펫 근처에 앵커되고 디스플레이 경계 안으로 클램프된다. 3단 구조:

```text
┌──────────────────────────────────┐
│ 헤더                              │
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
│  [설정] [종료]                     │
└──────────────────────────────────┘
```

규칙:

- 두 provider 모두 **항상** 탭으로 열람 가능하다. 한쪽의 `auth_required`/`unavailable`이 다른 쪽 표시를 막지 않는다 (provider 독립 원칙).
- 탭별 본문은 해당 provider의 `PetUiState` 파생 규칙을 그대로 재사용한다 (주 provider 여부와 무관하게 동일한 도출 함수).
- "지금 새로고침"은 아키텍처의 디바운스 정책을 따르고, 디바운스 중에는 비활성화 표시한다.
- 패널 자체의 로딩 스켈레톤은 `loading`일 때만 사용한다. 백그라운드 갱신 중에는 기존 값 유지.
- 설정 항목 (MVP): 주 provider 선택, 말풍선 켜기/끄기, 로그인 시 시작.

## 6. GIF 에셋 계약 (상태별)

기존 에셋 계약("첫 구현은 idle 하나 필수")을 유지하면서 선택 상태 키를 표준화한다.

| 상태 키 | 필수 여부 | 재생 시점 |
| --- | --- | --- |
| `idle` | **필수** | 기본. 다른 키가 없을 때의 최종 폴백 |
| `idle_warn` | 선택 | `pet_mood == warn` |
| `idle_critical` | 선택 | `pet_mood == critical` |
| `idle_exhausted` | 선택 | `pet_mood == exhausted` |
| `sleep` | 선택 | `system ∈ {unavailable, offline}` |
| `dragging` | 선택 | 드래그 중 |

폴백 체인: **요청 상태 키 → `idle`.** 중간 단계 폴백(예: critical→warn)은 두지 않는다 — 제작자가 일부만 만들어도 결과가 예측 가능해야 한다.

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
