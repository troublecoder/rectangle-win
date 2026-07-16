# 입력/오버레이 전면 교체 설계

**날짜:** 2026-07-16
**상태:** 승인됨 → 구현 계획 대기

## 배경 및 동기

현재 구현은 `WH_KEYBOARD_LL`/`WH_MOUSE_LL` 저수준 후킹 + Tauri 웹뷰 오버레이 창 방식이다. 이 방식의 문제점:

1. **PowerToys KeyboardManager와 충돌** — LL hook 체인에서 상호 간섭. KeyboardManager가 쓰는 LL hook과 우리 hook이 충돌하여 키 입력이 꼬임.
2. **`LowLevelHooksTimeout` 자동 해제** — LL hook 콜백이 300ms(기본값) 초과 시 Windows가 자동 해제. 우리 콜백 내에서 config 로드(디스크 I/O), Mutex lock, Tauri emit 수행 → 타임아웃 위반 위험.
3. **Tauri 웹뷰 오버레이의 포커스/클릭 간섭** — always-on-top 웹뷰 창이 다른 창의 클릭/포커스를 차단. snap 대상 창의 포커스를 빼앗아 foreground window를 잃게 함.
4. **깜빡임** — 마우스 이동마다 emitter가 show/hide 반복 + 웹뷰 리렌더링 → 조준경이 깜빡임.

## 목표

- **LL hook 완전 제거** → PowerToys 충돌/시스템 입력 문제 근본 해결
- **부드러운 오버레이** → DirectComposition + Direct2D로 GPU 직접 합성 (깜빡임 없음, 안티앨리어싱)
- **멀티 모니터 동적 감지** → 모니터 연결/해제 실시간 대응
- **핵심 도메인 로직 보존** → SnapService/KeyboardService/도메인/순수 로직 변경 없음

## 결정된 요구사항

- 마우스 throw 방식 유지 (Win+Alt 누른 채 마우스 이동 → snap). 타이틀바 드래그 snap은 제외.
- OverrideOs 모드 포기 (Win+방향키 OS snap 삼키기). LL hook 불필요.
- 오버레이 렌더링: DirectComposition + DXGI swap chain + Direct2D (접근법 A)
- 멀티 모니터: WM_DISPLAYCHANGE로 동적 감지

## 아키텍처

헥사고날(ports-and-adapters) 구조 유지. 도메인/애플리케이션 계층은 변경 없이, 인프라 어댑터만 교체.

```
[Domain: SnapService / KeyboardService / 도메인 모델]  — 변경 없음
        ↑ 호출
[Application: ports.rs (OverlayController/WindowMover/MonitorProvider/ConfigStore)]  — 변경 없음
        ↑ 구현
[Infrastructure: 새/교체 어댑터]
  ├─ win32_input.rs       (신규) RegisterHotKey + GetAsyncKeyState 폴링
  ├─ win32_overlay.rs     (신규) DirectComposition/Direct2D 오버레이
  ├─ win32_window.rs      (기존) WindowMover + SW_RESTORE 추가
  └─ win32_monitor.rs     (기존) MonitorProvider + WM_DISPLAYCHANGE 처리
```

삭제 파일:
- `src-tauri/src/infrastructure/win32_input_hook.rs` (LL hook)
- `overlay.html`, `src/overlay.ts` (Tauri 웹뷰 오버레이 진입점)
- tauri.conf.json의 overlay 창 정의

## 컴포넌트 상세

### 1. win32_overlay.rs — DirectComposition 오버레이 (신규)

`OverlayController` trait의 두 번째 구현체. TauriOverlay(웹뷰)를 대체.

**창 생성:**
- 확장 스타일: `WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`
  - `WS_EX_NOREDIRECTIONBITMAP`: 리다이렉션 비트맵 할당 안 함 → DirectComposition이 직접 합성 (최고 효율)
  - `WS_EX_TRANSPARENT`: 클릭스루 (입력이 아래 창으로 통과)
  - `WS_EX_TOPMOST`: 항상 위
  - `WS_EX_NOACTIVATE`: 포커스 획득 안 함
- 전체 가상 데스크톱 크기(SM_XVIRTUALSCREEN, SM_CXVIRTUALSCREEN 등)의 투명 창
- 창은 앱 시작 시 한 번 생성 후 계속 유지. show/hide는 visible 플래그로만 제어 (깜빡임 원인인 show/hide 반복 제거)

**렌더링 파이프라인:**
1. D3D11 device 생성
2. DXGI swap chain 생성(`DXGI_ALPHA_MODE_PREMULTIPLIED`, 창에 연결)
3. Direct2D factory → D3D11 백버퍼에서 Direct2D 렌더타겟(`ID2D1DeviceContext`) 생성
4. DirectComposition device + target + visual 생성
5. visual에 swap chain 연결 → target의 root로 설정

**그리기 (OverlayController 메서드):**
- `show_reticle(center_x, center_y, sector_count)`: 상태 저장 + 전체 재그리기 + 창 visible
  - 섹터 부채꼴(Pie) — Direct2D path geometry + 안티앨리어싱
  - 중심점 작은 원
- `highlight_sector(sector)`: 활성 섹터 채우기 색 변경 + 재그리기
- `show_snap_preview(x, y, w, h)`: 점선 사각형(D2D dashed stroke) + 재그리기
- `hide()`: 창 hidden (visible 플래그만)
- `update_cursor_indicator(x, y)`: 커서 위치 작은 원 (선택적)

**재그리기 전략:** 부분 갱신 없이 상태 변경 시마다 전체 재그리기. Direct2D는 충분히 빠르며 throw 중에는 초당 수십 번 갱신되어도 GPU 부하 미미.

**그리기 스레드:** OverlayController 메서드는 입력 스레드(폴링/핫키 처리 스레드)에서만 호출됨. SnapService가 내부적으로 overlay 메서드를 호출하므로, SnapService 호출이 입력 스레드에서 이루어지면 overlay 그리기도 동일 스레드에서 발생. D2D 렌더타겟은 단일 스레드 전용이지만, 입력 스레드에서 직렬 호출되므로 스레드 안전. 렌더타겟 접근은 동일 스레드 내에서만 일어나므로 추가 Mutex 불필요. 단, SnapService 자체는 여러 스레드에서 호출될 수 있으므로(Tauri command 등), overlay 렌더타겟 접근은 SnapService 내부 Mutex와 별도로 D2D 컨텍스트 보호가 필요할 수 있음 — 구현 시 검증.

**Graceful degradation:** D3D11/DComp 초기화 실패 시 — snap은 계속 작동, 오버레이만 비활성. 에러 로깅.

### 2. win32_input.rs — RegisterHotKey + 폴링 (신규)

`InputHookController`를 대체.

**구조체 `Win32InputListener`:**
- `Arc<SnapService>`, `Arc<KeyboardService>`, `Arc<dyn ConfigStore>` 보관
- origin 커서 좌표(Mutex<Option<(i32,i32)>>), throw 활성 상태(Mutex<bool>)

**시작 (`start()`):**
- 전용 스레드(`std::thread::spawn`)에서 message-only 창 생성 + GetMessage 루프
- 동일 스레드에서 키보드 핫키(WM_HOTKEY)와 마우스 폴링을 모두 처리

**키보드 snap — RegisterHotKey:**
- config의 `keyboard.trigger_modifiers` + 방향키(VK_LEFT/RIGHT/UP/DOWN)로 4개 핫키 등록
- 플래그: `MOD_NOREPEAT` (auto-repeat 방지) + modifier 플래그(MOD_CONTROL, MOD_ALT 등)
- WM_HOTKEY 수신 → 방향 판별 → `KeyboardService::on_direction_key` 호출
- config 변경 감지 시: 기존 핫키 해제(UnregisterHotKey) + 재등록. 설정 저장 시 Tauri command → 스레드에 PostMessage로 알림.

**마우스 throw — GetAsyncKeyState 폴링:**
- GetMessage 대기 중 `MsgWaitForMultipleObjects`로 타임아웃(~16ms) 설정 → 폴링 주기 확보
- 매 폴링마다:
  1. throw modifier 조합 모두 눌렸는지 GetAsyncKeyState로 확인
  2. Idle→Held 전환: `GetCursorPos`로 origin 캡처 + `snap_service.on_modifier_pressed(cx, cy)`
  3. Held 유지: `GetCursorPos`로 현재 좌표, delta = 현재 - origin, `snap_service.on_mouse_moved(cx, cy, dx, dy)`
  4. Held→Idle 전환: `snap_service.on_modifier_released(cancel=false, cx, cy)`
- throw trigger modifier → GetAsyncKeyState VK 코드 매핑 (Win→VK_LWIN/VK_RWIN, Alt→VK_MENU, Ctrl→VK_CONTROL, Shift→VK_SHIFT)

**RegisterHotKey 실패 처리:** 조합이 이미 점유된 경우 등록 실패. 해당 방향은 비활성, 로깅. throw는 계속 작동.

### 3. win32_monitor.rs 확장 — 멀티 모니터 동적 감지

- 입력 스레드의 message-only 창에서 `WM_DISPLAYCHANGE` 수신
- MonitorProvider의 캐시 무효화 + `EnumDisplayMonitors` 재호출
- MonitorProvider에 `invalidate_cache()` 메서드 추가 또는 내부 Mutex로 캐시 관리
- throw snap은 커서가 있는 모니터 기준이므로 항상 최신 모니터 정보 필요

### 4. win32_window.rs — WindowMover (기존 + SW_RESTORE)

- 기존 구현 유지 + `is_own_window` 체크(앱 창 snap 제외)
- **SW_RESTORE 추가**: Area snap 전 `ShowWindow(hwnd, SW_RESTORE)` 호출 — maximize된 창은 SetWindowPos가 무시되므로 먼저 복원

### 5. presentation/state.rs — overlay 타입 변경

- `overlay: Arc<TauriOverlay>` → `Arc<dyn OverlayController>`
- Windows: `Win32LayeredOverlay`(DComp) 생성
- 비Windows(cfg(not(windows))): 기존 `TauriOverlay` 유지 (CI/컴파일용)

### 6. lib.rs — wiring 정리

- 기존 emitter/set_emitter/NOACTIVATE/overlay 창 제어 코드 전부 제거
- `Win32InputListener::start(snap, keyboard, config)` 호출 (Windows)
- overlay는 리스너가 직접 보유 (Tauri 창 아님)

### 7. tauri.conf.json / capabilities

- overlay 창 정의 제거 (main만)
- capabilities: `["main"]`

## 데이터 흐름

```
키보드 snap:
  OS → RegisterHotKey → WM_HOTKEY → Win32InputListener
  → KeyboardService::on_direction_key(dir, cx, cy)
  → WindowMover::apply_snap_target (체인 사이클)

마우스 throw:
  GetAsyncKeyState 폴링 → Win32InputListener
  → SnapService::on_modifier_pressed/moved/released
    → (내부) OverlayController 메서드 → DirectComposition 오버레이 갱신
    → (release 시) WindowMover::apply_snap_target

멀티 모니터:
  OS → WM_DISPLAYCHANGE → Win32InputListener → MonitorProvider 캐시 무효화
```

## 에러 처리

- RegisterHotKey 실패: 로깅 + 해당 핫키 비활성. 앱 계속 실행(throw만 작동)
- D3D11/DComp 초기화 실패: 오버레이 없이 snap만 작동(Graceful degradation). 로깅
- 폴링 중 GetAsyncKeyState 실패: 무시, 다음 폴링. 절대 패닉 X
- 창 snap 실패(foreground 없음 등): SnapService가 AppResult로 이미 처리. 로깅만
- config 저장 후 핫키 재등록 실패: 기존 핫키 유지 + 로깅

## 테스트 전략

- **순수 로직(기존 88개 테스트 유지):** classify, SnapService, KeyboardService, 도메인 FSM/체인/기하학. 변경 없음.
- **Win32 어댑터:** 실제 OS 상호작용이라 단위 테스트 제외(기존 win32_window/win32_monitor 패턴과 동일). 수동 테스트로 통합 검증.
- **회귀:** `cargo test`로 순수 로직 보호. 수동 테스트 체크리스트:
  - 설정 UI 정상 동작
  - Ctrl+Alt+방향키 → 체인 snap
  - Win+Alt + 마우스 → 부드러운 조준경/미리보기 + snap
  - maximize된 창 → 다시 snap 가능 (SW_RESTORE)
  - 모니터 연결/해제 → snap 기준 모니터 갱신
  - PowerToys 실행 중 → 충돌 없음

## 위험 및 완화

- **DirectComposition 복잡도**: windows-rs 0.58에 D3D11/D2D/DComp 바인딩 존재 확인. 별도 모듈 격리로 복잡도 관리. 초기화 실패 시 graceful degradation.
- **RegisterHotKey 점유 충돌**: 다른 앱이 같은 조합을 쓰면 등록 실패. 설정에서 대체 조합 허용(향후). 현재는 에러 로깅.
- **폴링 CPU 소비**: 16ms(60Hz) 폴링은 미소. GetAsyncKeyState는 가벼운 호출. 배터리 영향 미미.
- **Windows 버전 호환성**: DirectComposition은 Windows 8+ 필수. Windows 10/11(타겟)에서 문제 없음.

## 구현 순서 (writing-plans에서 상세화)

1. 정리: 기존 LL hook + Tauri 오버레이 제거, 설정 창 기준선 확보
2. win32_overlay.rs: DirectComposition/Direct2D 오버레이 (OverlayController 구현)
3. win32_input.rs: RegisterHotKey + 폴링 리스너
4. win32_monitor.rs: WM_DISPLAYCHANGE 동적 감지
5. win32_window.rs: SW_RESTORE (기존에 이미 추가됨)
6. wiring: state.rs/lib.rs/tauri.conf.json/capabilities
7. 검증: cargo check/test + npm run tauri dev 수동 테스트
