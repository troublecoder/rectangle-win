# 아키텍처 개요

## 헥사고날 아키텍처 (Ports & Adapters)

이 프로젝트는 헥사고날 패턴을 따릅니다. 도메인 로직은 외부 시스템에 의존하지 않고, 인프라 계층이 trait(port)를 구현하여 주입됩니다.

```
┌─────────────────────────────────────────────────────┐
│                    Presentation                      │
│  commands.rs · tray.rs · state.rs                    │
│  (Tauri IPC 명령, 시스템 트레이, 상태 관리)            │
├─────────────────────────────────────────────────────┤
│                   Application                        │
│  snap_service.rs · keyboard_service.rs · ports.rs   │
│  (스냅 오케스트레이션, trait 정의)                      │
├─────────────────────────────────────────────────────┤
│                     Domain                           │
│  model.rs · geometry.rs · cursor_fsm.rs · presets.rs │
│  (순수 로직 — OS/IO 의존 없음, 단위 테스트 가능)        │
├─────────────────────────────────────────────────────┤
│                  Infrastructure                      │
│  win32_input.rs · win32_overlay.rs · win32_window.rs │
│  win32_monitor.rs · toml_config.rs                   │
│  (Win32 API 어댑터 — Windows 타겟 전용)                │
└─────────────────────────────────────────────────────┘
```

## 핵심 데이터 흐름

### 마우스 Throw 스냅
```
Win+Alt 누름 (LL keyboard hook)
  → 채널(input-worker 스레드)로 ThrowPressed 메시지 전송
  → SnapService::on_modifier_pressed → OverlayController::show_reticle (origin 마커)
  → 마우스 이동 (LL mouse hook)
  → 채널로 MouseMoved 메시지 전송
  → SnapService::on_mouse_moved → sector 계산 → OverlayController::show_snap_preview
  → Win+Alt 뗌 (LL keyboard hook)
  → 채널로 ThrowReleased 메시지 전송
  → SnapService::on_modifier_released → WindowMover::apply_snap_target (창 이동)
```

### 키보드 스냅
```
Win+Alt+방향키 (LL keyboard hook)
  → 삼킴 (return LRESULT(1), CallNextHookEx 호출 안 함)
  → 채널로 DirectionKey 메시지 전송
  → KeyboardService::on_direction_key
    ←/→: throw.mapping 순회
    ↑/↓: [maximize, restore, center, minimize] 순환
  → WindowMover::apply_snap_target
```

## 스레드 구조

| 스레드 | 역할 |
|--------|------|
| **Tauri 메인** | 이벤트 루프, IPC, 설정 UI |
| **win32-input** | LL hook 설치 + GetMessageW 펌프 |
| **input-worker** | 채널에서 메시지 수신 → SnapService/KeyboardService 호출 |

LL hook 콜백은 반드시 가벼워야 합니다 (300ms `LowLevelHooksTimeout`). 모든 무거운 작업(D2D 렌더링, config 로드, 창 이동)은 input-worker 스레드에서 처리됩니다.

## 설정 (Config)

- 저장소: `%APPDATA%\rectangle-win\config.toml`
- 캐싱: `TomlConfigStore`가 Mutex로 캐싱, load()는 캐시에서 clone
- LL hook 콜백은 config를 디스크에서 읽지 않음 — `AtomicBool` 캐시(static)에서 읽음
- 설정 저장 시 `update_config()`로 캐시 갱신

## 오버레이 (Overlay)

- 창 스타일: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`
- 렌더링: Direct2D DC render target → 32bpp DIB → `UpdateLayeredWindow`
- 창은 전체 가상 데스크톱 크기, `visible` 플래그로 show/hide 제어
- draw_scene을 ShowWindow보다 먼저 실행하여 깜빡임 방지
