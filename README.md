# Rectangle Win

Windows용 창 스냅(Window Snap) 유틸리티. [Rectangle](https://github.com/rxhanson/Rectangle)(macOS)에서 영감을 받아, 마우스 제스처와 키보드 단축키로 창을 화면 영역에 빠르게 배치합니다.

## 기능

### 마우스 Throw 스냅
- `Win + Alt` 키를 누른 채 마우스를 움직여 8방향 영역으로 창을 스냅
- 시작점에 커서 마커 표시, snap 대상 영역을 실시간 미리보기
- 색상, 크기, 투명도 설정 가능

### 키보드 스냅
- `Win + Alt + ←/→`: throw 매핑을 순회하며 영역 변경
- `Win + Alt + ↑/↓`: Maximize → Restore → Center → Minimize 순환

### Snap Editor
- 시각적으로 snap 영역 편집 (드래그/리사이즈)
- 섹터 매핑(8방향 → snap 타겟) 설정
- 5가지 프리셋(Minimal, Standard, Extended, Full, Portrait)

### 시스템
- 시스템 트레이 아이콘
- 자동 시작(로그인 시 실행)
- 멀티 모니터 동적 감지
- 한국어/영어 다국어 지원
- 자동 업데이트(GitHub Releases)

## 기술 스택

| 계층 | 기술 |
|------|------|
| 프론트엔드 | Vue 3 + TypeScript + Vite |
| UI 프레임워크 | Nuxt UI v4 + Tailwind CSS + Catppuccin 테마 |
| 상태 관리 | Pinia |
| 검증 | Zod (런타임 타입 검증) |
| 백엔드 | Rust + Tauri v2 |
| 오버레이 | Direct2D + WS_EX_LAYERED (Win32) |
| 입력 | WH_KEYBOARD_LL / WH_MOUSE_LL (FancyZones 방식) |

## 아키텍처

헥사고날(ports-and-adapters) 패턴:

```
Domain (순수 로직)
  ├── model.rs       — Config, SnapTarget, Direction, SectorMap
  ├── geometry.rs    — 좌표 변환, sector 계산
  ├── cursor_fsm.rs  — 스냅 상태 머신
  └── presets.rs     — snap 영역 프리셋
        ↑
Application (서비스)
  ├── snap_service.rs       — 마우스 throw 스냅 오케스트레이션
  ├── keyboard_service.rs   — 키보드 스냅 오케스트레이션
  └── ports.rs              — trait 정의 (WindowMover, MonitorProvider, ...)
        ↑
Infrastructure (Win32 어댑터)
  ├── win32_input.rs        — LL hook + 채널 작업 스레드
  ├── win32_overlay.rs      — Direct2D Layered 창 오버레이
  ├── win32_window.rs       — 창 이동/크기/액션
  ├── win32_monitor.rs      — 모니터 정보 + 동적 감지
  └── toml_config.rs        — TOML 설정 저장소
```

## 개발 환경

### 필수 요구사항
- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable)
- Windows 10/11

### 실행
```bash
npm install
npm run tauri dev
```

### 빌드
```bash
npm run tauri build
```

### 테스트
```bash
# Rust 단위 테스트
cd src-tauri && cargo test

# 프론트엔드 타입 체크
npm run build
```

## 설정

설정 파일 위치: `%APPDATA%\rectangle-win\config.toml`

| 섹션 | 설명 |
|------|------|
| `[general]` | 자동 시작, 트레이 표시, 언어, snap 여백 |
| `[snap]` | 활성 프리셋, snap 영역 목록 |
| `[throw]` | trigger modifier, 8섹터 매핑 |
| `[keyboard]` | 키보드 snap 활성화, 순회 타이아웃 |
| `[overlay]` | 색상, 크기, 투명도, 섹터 수 |
| `[update]` | 자동 업데이트 채널 |

## 라이선스

MIT
