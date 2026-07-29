<p align="center">
  <img src="docs/assets/logo.png" alt="CacheBite 로고" width="160" />
</p>

<h1 align="center">CacheBite</h1>

<p align="center">
  Claude와 Codex 사용량을 바탕화면 위에서 보여주는 작은 데스크톱 펫입니다. 자격 증명은 로컬에만 남고, 베타 릴리스로 배포됩니다.
</p>

<p align="center">
  <a href="README.en.md">English</a> · 한국어
</p>

<p align="center">
  <img alt="Beta" src="https://img.shields.io/badge/status-beta-orange" />
  <img alt="Local only" src="https://img.shields.io/badge/privacy-local--only-success" />
  <img alt="Windows" src="https://img.shields.io/badge/platform-Windows-blue" />
  <img alt="macOS" src="https://img.shields.io/badge/platform-macOS-lightgrey" />
  <img alt="Linux" src="https://img.shields.io/badge/platform-Linux-yellow" />
  <img alt="Tauri 2" src="https://img.shields.io/badge/runtime-Tauri%202-24C8DB" />
  <img alt="문서 언어" src="https://img.shields.io/badge/docs-Korean%20%2B%20English-111827" />
</p>

<p align="center">
  <a href="#배포와-설치">배포와 설치</a> ·
  <a href="#사용법">사용법</a> ·
  <a href="#동작-방식">동작 방식</a> ·
  <a href="#주요-기능">주요 기능</a> ·
  <a href="#개인정보">개인정보</a> ·
  <a href="#베타-상태와-제한">베타 상태</a> ·
  <a href="#릴리스-운영">릴리스 운영</a>
</p>

<p align="center">
  <img
    src="docs/assets/screenshots/hero.png"
    alt="바탕화면 위의 CacheBite 펫과 열린 사용량 패널을 함께 보여주는 히어로 이미지"
    width="960"
  />
</p>

---

## 배포와 설치

> **서명되지 않은 베타 안내:** CacheBite 빌드는 아직 코드 서명되지 않았습니다. Windows에서는 SmartScreen 경고가 뜰 수 있고, macOS에서는 quarantine를 풀기 전까지 Gatekeeper가 차단합니다. Linux는 AppImage를 사용하며 호스트에 WebKitGTK 4.1이 필요합니다.

가장 최근 베타는 [GitHub Releases](https://github.com/chanhoan/CacheBite/releases)에서 받을 수 있습니다.

| 플랫폼 | 산출물 | 참고 |
| --- | --- | --- |
| Windows | MSI, NSIS 설치 파일 | 새 베타는 기존 설치 위에 덮어씌워 설치하면 됩니다. |
| macOS | 서명 전 검증용 DMG | 현재는 검증용이며, notarization 배포본은 아닙니다. |
| Linux | AppImage | WebKitGTK 4.1과 데스크톱 의존성이 필요합니다. |

설치 전에는 `SHA256SUMS.txt`와 대조해서 다운로드를 확인하세요.
플랫폼별 설치 방법과 베타 리포트 규칙은 [docs/beta-testing.md](docs/beta-testing.md)에 정리되어 있습니다.

### 첫 실행 전

- Claude는 이 머신에서 Claude Code에 로그인되어 있어야 합니다.
- Codex는 `PATH`에 있는 `codex` CLI가 로그인된 상태여야 합니다.
- provider 하나만 있어도 괜찮습니다. 한쪽만 로그인되어 있으면 CacheBite는 다른 쪽을 데이터가 있는 척하지 않고 unavailable로 보여줍니다.

---

## 사용법

1. CacheBite를 설치하고 실행합니다.
2. 작은 펫이 바탕화면에 나타납니다. 원하는 곳으로 드래그하면 되고, 위치는 디스플레이별로 저장됩니다.
3. 펫을 더블클릭하거나 포커스가 있는 상태에서 Enter를 누르면 사용량 패널이 열립니다.
4. 패널은 Claude와 Codex를 탭으로 보여줍니다. 각 provider는 세션 창과 주간 창을 보여주며, 현재 지원하는 두 provider의 세션 창은 5시간 창으로 노출됩니다.
5. 탭을 바꿔 다른 provider를 확인합니다.
6. `Refresh now`로 최신 사용량을 다시 불러오거나, `Set as primary`로 펫의 링과 상태가 따라갈 기본 provider를 바꿉니다.
7. `Settings`에서 외형, primary provider, 펫, 말풍선, 알림, 보조 provider 알림, 로그인 시 시작을 설정할 수 있습니다.

선택한 펫은 primary provider와 별개입니다. 하나를 바꿔도 다른 하나는 바뀌지 않습니다.

링의 두 개 아크는 세션 창과 주간 창을 나타냅니다. 펫의 상태는 primary provider의 사용량을 따라가므로, 바탕화면 상태만 봐도 지금 중요한 provider의 상태를 읽을 수 있습니다.

앱에서 보이는 상태 의미:

- `Fresh`는 스냅샷이 최신 상태라는 뜻입니다.
- `Stale`는 아직 표시할 수는 있지만 최신 상태 기준 시간을 넘었다는 뜻입니다.
- provider에 로그인되어 있지 않거나 사용할 수 없으면, CacheBite는 그 상태를 그대로 보여주고 사용량이 있는 척하지 않습니다.
- 현재 빌드에서는 전체 화면 감지를 사용할 수 없어서, 프레젠테이션 중에도 펫이 자동으로 숨겨지지 않습니다. 그 상태를 그대로 보여줍니다.

---

## 동작 방식

1. CacheBite가 provider 사용량을 로컬 머신에서 수집합니다.
2. Rust 쪽 핵심 코드가 provider별 응답을 공통 사용량 모델로 정규화합니다.
3. Svelte 렌더러가 정규화된 상태를 바탕으로 펫, 링, 패널, 설정 화면을 보여줍니다.
4. 말풍선과 알림은 원격 서비스가 아니라 로컬 사용량 변화에 따라 동작합니다.

---

## 주요 기능

| 영역 | CacheBite가 하는 일 |
| --- | --- |
| 데스크톱 펫 | 화면 위에 펫을 띄우고, 디스플레이별 위치를 기억합니다. |
| 사용량 화면 | Claude와 Codex를 탭으로 보여주고, 세션 창과 주간 창을 함께 보여줍니다. |
| 링 상태 | primary provider의 사용량이 펫의 상태와 링을 움직입니다. |
| 설정 | 외형, provider 기준, 펫, 말풍선, 알림, 로그인 시 시작을 바꿀 수 있습니다. |

---

## 개인정보

> CacheBite는 provider 자격 증명을 로컬에만 둡니다. CacheBite 서버로 보내는 것은 없고, 렌더러에는 정규화된 상태만 전달됩니다.

- CacheBite 클라우드 서비스는 없습니다.
- 자격 증명 접근, 수집, 갱신 스케줄링, 영속성은 Rust 쪽 핵심 코드에 있습니다.
- Svelte 렌더러는 정규화된 상태만 받습니다.
- CacheBite는 브라우저 쿠키를 읽지 않습니다.
- provider의 원본 응답, 계정 식별자, 자격 증명 경로는 로그나 스크린샷에 남기면 안 됩니다.
- 자격 증명 파일은 읽기 전용으로 다룹니다.

---

## 베타 상태와 제한

- 베타 버전은 아직 서명되지 않았습니다.
- 릴리스 공개는 아직 사람이 직접 해야 합니다.
- 자동 업데이트는 아직 없습니다.
- 현재 빌드에서는 전체 화면 감지를 사용할 수 없어서, 프레젠테이션 중에도 펫이 자동으로 숨겨지지 않습니다.
- provider 한쪽만 로그인되어 있으면, 다른 쪽은 unavailable로 남습니다.

---

## 릴리스 운영

- 태그를 푸시하면 플랫폼별 설치 파일과 체크섬이 붙은 GitHub draft release가 만들어집니다.
- 릴리스 공개는 사람이 직접 합니다.
- 서명과 notarization이 된 macOS 빌드는 보호된 `production-macos-signing` 워크플로 입력으로만 생성됩니다.

---

## 라이선스

아직 프로젝트 라이선스는 정해지지 않았습니다. 추가되기 전까지는 모든 권리가 보유됩니다.
