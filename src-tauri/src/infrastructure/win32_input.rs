#![cfg(windows)]

//! RegisterHotKey + GetAsyncKeyState 폴링 기반 입력 리스너.
//!
//! LL(Low-Level) 키보드/마우스 훅 없이 전역 핫키(키보드 snap)와
//! modifier 조합 감지(마우스 throw)를 처리한다.
//!
//! - **키보드 snap**: `RegisterHotKey` 로 Ctrl+Alt+방향키(설정 가능)를 등록한다.
//!   `MOD_NOREPEAT` 로 자동 반복 스톰을 방지한다. 점유된 조합은 로그만 남기고 스킵.
//! - **마우스 throw**: `GetAsyncKeyState` 를 폴링 주기(≈16ms)로 호출하여
//!   throw modifier 조합(기본 Win+Alt)이 모두 눌렸는지 검사한다.
//!   Idle→Held 전이 시 origin 캡처 + `on_modifier_pressed`,
//!   Held 유지 시 delta 계산 + `on_mouse_moved`,
//!   Held→Idle 전이 시 `on_modifier_released(cancel=false)` 를 호출한다.
//!
//! 메시지 루프는 message-only 창에서 `MsgWaitForMultipleObjects` 타임아웃을
//! 폴링 틱으로 활용한다 — 메시지가 도착하면 즉시 깨어나고, 타임아웃이면
//! 폴링을 수행한다. PowerToys 등 다른 전역 핫키 등록 앱과 충돌하지 않는다.
//!
//! 단위 테스트는 실제 OS 입력 상호작용이 필요하므로 작성하지 않는다
//! (기존 win32_window/win32_monitor/win32_overlay 패턴과 동일).

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
    MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_DOWN, VK_LEFT, VK_LWIN, VK_MENU, VK_RIGHT,
    VK_RWIN, VK_SHIFT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, MsgWaitForMultipleObjects,
    PeekMessageW, PostThreadMessageW, RegisterClassExW, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    HWND_MESSAGE, MSG, PM_REMOVE, QS_ALLINPUT, WINDOW_EX_STYLE, WINDOW_STYLE, WM_HOTKEY, WM_QUIT,
    WNDCLASSEXW,
};

use crate::application::keyboard_service::KeyboardService;
use crate::application::ports::ConfigStore;
use crate::application::snap_service::SnapService;
use crate::domain::model::Direction;
use crate::infrastructure::win32_monitor::Win32MonitorProvider;

/// 핫키 ID — 방향키 매핑. RegisterHotKey 의 id 파라미터로 사용.
const HOTKEY_LEFT: i32 = 1;
const HOTKEY_RIGHT: i32 = 2;
const HOTKEY_UP: i32 = 3;
const HOTKEY_DOWN: i32 = 4;

/// 폴링 주기 (마우스 throw modifier 감지). 약 60fps.
/// MsgWaitForMultipleObjects 의 dwmilliseconds(u32) 에 직접 전달.
const POLL_INTERVAL_MS: u32 = 16;

/// WM_DISPLAYCHANGE — 모니터 연결/해제/DPI 변경 시 브로드캐스트 (winuser.h).
/// 0x007E. 수신 시 MonitorProvider 캐시를 무효화한다.
const WM_DISPLAYCHANGE: u32 = 0x007E;

/// 입력 리스너. `start()` 로 전용 스레드를 시작한다.
///
/// 스레드 종료를 위해 thread id 를 보관하며, `stop()` 이 WM_QUIT 을 게시하면
/// 메시지 루프가 종료된다.
pub struct Win32InputListener {
    thread_id: u32,
}

/// 스레드 간 공유 입력 상태 (마우스 throw 추적용).
struct InputState {
    /// throw 활성 시 origin 커서 좌표. Idle 상태에서는 None.
    origin: Option<(i32, i32)>,
    /// throw modifier 조합이 현재 눌려 있는지.
    throw_active: bool,
}

impl Win32InputListener {
    /// 입력 리스너 스레드 시작.
    ///
    /// 전용 "win32-input" 스레드에서 message-only 창을 생성하고 핫키를 등록한 뒤
    /// 메시지 루프 + 폴링을 실행한다. SnapService/KeyboardService/config 는
    /// 스레드로 이동(move)한다.
    pub fn start(
        snap_service: Arc<SnapService>,
        keyboard_service: Arc<KeyboardService>,
        config_store: Arc<dyn ConfigStore>,
        monitor_provider: Arc<Win32MonitorProvider>,
    ) -> Self {
        // 스레드 종료를 위해 thread id 를 부모로 반환.
        // thread::Builder 에서 spawn 직전에 thread id 를 알 수 없으므로,
        // 채널이나 Once 로 전달해야 한다. 여기서는 간단히 Mutex 로 전달.
        let thread_id_slot: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let slot_for_thread = thread_id_slot.clone();

        thread::Builder::new()
            .name("win32-input".into())
            .spawn(move || {
                // 자신의 thread id 를 부모에게 알림.
                let tid = unsafe {
                    windows::Win32::System::Threading::GetCurrentThreadId()
                };
                {
                    let mut slot = slot_for_thread.lock().unwrap();
                    *slot = Some(tid);
                }

                let state = Arc::new(Mutex::new(InputState {
                    origin: None,
                    throw_active: false,
                }));
                if let Err(e) = run_message_loop(
                    snap_service,
                    keyboard_service,
                    config_store,
                    monitor_provider,
                    state,
                ) {
                    eprintln!("입력 리스너 오류: {e}");
                }
            })
            .expect("입력 리스너 스레드 시작 실패");

        // 스레드가 thread id 를 기록할 때까지 잠시 대기 (스핀).
        // start() 는 동기적으로 Win32InputListener 를 반환해야 하므로,
        // thread id 가 채워질 때까지 짧게 대기한다.
        let thread_id = loop {
            if let Some(tid) = *thread_id_slot.lock().unwrap() {
                break tid;
            }
            thread::sleep(Duration::from_millis(1));
        };

        Win32InputListener { thread_id }
    }

    /// 입력 리스너 정지 — 스레드에 WM_QUIT 게시.
    ///
    /// 메시지 루프가 WM_QUIT 을 받으면 핫키 해제 후 종료된다.
    /// 스레드 자체의 join 은 수행하지 않는다 (best-effort).
    #[allow(dead_code)]
    pub fn stop(&self) {
        // SAFETY: PostThreadMessageW 는 thread id 가 유효하면 안전.
        // WM_QUIT 은 GetMessage/PeekMessage 루프에서 감지된다.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// 메시지 루프
// ────────────────────────────────────────────────────────────────────

/// message-only 창에서 GetMessage 루프 + 폴링.
fn run_message_loop(
    snap_service: Arc<SnapService>,
    keyboard_service: Arc<KeyboardService>,
    config_store: Arc<dyn ConfigStore>,
    monitor_provider: Arc<Win32MonitorProvider>,
    state: Arc<Mutex<InputState>>,
) -> windows::core::Result<()> {
    // message-only 창 생성.
    let hwnd = create_message_window()?;

    // RegisterHotKey — config 의 keyboard trigger_modifiers + 방향키.
    register_hotkeys(&hwnd, &config_store);

    // 메시지 + 폴링 루프.
    let mut msg = MSG::default();
    loop {
        // MsgWaitForMultipleObjects 타임아웃 대기.
        // 메시지가 오면 즉시 깨어나고, 아니면 POLL_INTERVAL_MS 후 타임아웃(폴링 틱).
        // SAFETY: 인자가 단순 값/상수. 핸들 슬라이스는 None (카운트 0).
        let wait = unsafe {
            MsgWaitForMultipleObjects(None, false, POLL_INTERVAL_MS, QS_ALLINPUT)
        };

        // 메시지 큐를 모두 비울 때까지 처리.
        // SAFETY: msg 는 로컬 스택 버퍼. hwnd=None 은 "모든 창의 메시지 + 스레드 메시지".
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                unregister_hotkeys(&hwnd);
                return Ok(());
            }
            // WM_DISPLAYCHANGE — 모니터 연결/해제/DPI 변경. MonitorProvider 캐시를
            // 무효화하여 다음 snap 이 최신 모니터 정보를 사용하도록 한다.
            // 캐시만 갱신하고 메시지 처리는 계속 진행 (DefWindowProcW 로도 전달됨).
            if msg.message == WM_DISPLAYCHANGE {
                monitor_provider.invalidate();
            }
            // WM_HOTKEY 처리 — 방향키 snap.
            if msg.message == WM_HOTKEY {
                handle_hotkey(msg.wParam.0 as i32, &keyboard_service);
            }
            // WM_DESTROY 등은 기본 처리만 (DefWindowProcW 가 PostQuitMessage 호출 안 함).
            // SAFETY: msg 는 방금 PeekMessageW 로 채운 유효 메시지.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // wait == WAIT_TIMEOUT(258) 이면 타임아웃 → 폴링 틱. WAIT_OBJECT_0(0) 이면
        // 메시지 도착 → 위에서 이미 처리. 어느 쪽이든 폴링(비용 작음, 정확도 향상).
        let _ = wait;
        poll_throw(&snap_service, &config_store, &state);
    }
}

/// message-only 창 생성.
///
/// `HWND_MESSAGE` 를 부모로 `CreateWindowExW` 호출 → 보이지 않는 메시지 전용 창.
/// WndProc 은 `DefWindowProcW` 로 모든 것을 위임 (직접 처리는 GetMessage 루프에서).
fn create_message_window() -> windows::core::Result<HWND> {
    let class_name = w!("RectangleWinInput");

    // SAFETY: WNDCLASSEXW 는 zero-init 후 필요 필드만 채운다.
    // RegisterClassExW 는 중복 등록 시 0 을 반환하지만 CreateWindowExW 가
    // 클래스를 찾지 못하면 에러를 반환하므로 여기서는 별도 처리하지 않는다.
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(input_wndproc),
        hInstance: HINSTANCE::default(),
        lpszClassName: class_name,
        ..Default::default()
    };
    let _atom = unsafe { RegisterClassExW(&wc) };

    // SAFETY: 클래스는 위에서 등록했음 (또는 이미 등록됨). HWND_MESSAGE 부모 → message-only.
    // HMENU/HINSTANCE/lpparam = 0(없음). 창 크기/위치는 message-only 이므로 의미 없음(0).
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("RectangleWinInput"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            HINSTANCE::default(),
            None,
        )?
    };

    Ok(hwnd)
}

/// message-only 창의 window proc — 모든 메시지를 DefWindowProcW 로 위임.
///
/// 실제 입력 처리(WM_HOTKEY) 및 WM_DISPLAYCHANGE 캐시 무효화는
/// PeekMessageW 루프에서 직접 수행한다.
unsafe extern "system" fn input_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

// ────────────────────────────────────────────────────────────────────
// 핫키 등록 / 해제
// ────────────────────────────────────────────────────────────────────

/// config.keyboard.trigger_modifiers → MOD_* 플래그 조합.
///
/// 문자열 목록("Ctrl", "Alt", "Win", "Shift")을 `HOT_KEY_MODIFIERS` 비트 OR 로 변환.
/// 알 수 없는 문자열은 무시한다. 항상 `MOD_NOREPEAT` 를 추가한다.
fn modifiers_to_flags(mods: &[String]) -> HOT_KEY_MODIFIERS {
    let mut flags = MOD_NOREPEAT;
    for m in mods {
        let f = match m.as_str() {
            "Ctrl" => Some(MOD_CONTROL),
            "Alt" => Some(MOD_ALT),
            "Win" => Some(MOD_WIN),
            "Shift" => Some(MOD_SHIFT),
            _ => None,
        };
        if let Some(f) = f {
            flags |= f;
        }
    }
    flags
}

/// 방향키 4종 핫키 등록.
///
/// `config.keyboard.trigger_modifiers` (기본 Ctrl+Alt) 와 결합하여
/// VK_LEFT/RIGHT/UP/DOWN 으로 4개의 핫키를 RegisterHotKey 한다.
/// 실패(이미 점유된 조합) 시 eprintln 로깅 + 해당 핫키만 스킵.
fn register_hotkeys(hwnd: &HWND, config_store: &Arc<dyn ConfigStore>) {
    let modifiers = match config_store.load() {
        Ok(cfg) => modifiers_to_flags(&cfg.keyboard.trigger_modifiers),
        Err(e) => {
            eprintln!("핫키 등록: config 로드 실패 ({e}) — Ctrl+Alt 폴백 사용");
            modifiers_to_flags(&["Ctrl".to_string(), "Alt".to_string()])
        }
    };

    let keys: [(i32, u32, &str); 4] = [
        (HOTKEY_LEFT, VK_LEFT.0 as u32, "Left"),
        (HOTKEY_RIGHT, VK_RIGHT.0 as u32, "Right"),
        (HOTKEY_UP, VK_UP.0 as u32, "Up"),
        (HOTKEY_DOWN, VK_DOWN.0 as u32, "Down"),
    ];

    for (id, vk, name) in keys {
        // SAFETY: hwnd 는 방금 생성한 유효 창. modifiers/vk 는 상수.
        match unsafe { RegisterHotKey(*hwnd, id, modifiers, vk) } {
            Ok(()) => {}
            Err(e) => {
                eprintln!("핫키 등록 실패 ({name}, id={id}): {e} — 스킵");
            }
        }
    }
}

/// 등록한 핫키 4종 모두 해제. 종료 시 호출.
fn unregister_hotkeys(hwnd: &HWND) {
    for id in [HOTKEY_LEFT, HOTKEY_RIGHT, HOTKEY_UP, HOTKEY_DOWN] {
        // SAFETY: hwnd 는 유효 창. id 는 등록 시 사용한 값.
        let _ = unsafe { UnregisterHotKey(*hwnd, id) };
    }
}

// ────────────────────────────────────────────────────────────────────
// WM_HOTKEY 핸들러 (키보드 snap)
// ────────────────────────────────────────────────────────────────────

/// WM_HOTKEY 처리 — 핫키 ID → Direction 매핑 후 KeyboardService 호출.
fn handle_hotkey(hotkey_id: i32, keyboard_service: &Arc<KeyboardService>) {
    let direction = match hotkey_id {
        HOTKEY_LEFT => Direction::Left,
        HOTKEY_RIGHT => Direction::Right,
        HOTKEY_UP => Direction::Up,
        HOTKEY_DOWN => Direction::Down,
        _ => return,
    };
    let (cx, cy) = current_cursor();
    if let Err(e) = keyboard_service.on_direction_key(direction, cx, cy) {
        eprintln!("키보드 snap 오류: {e}");
    }
}

/// 현재 커서 좌표 (GetCursorPos). 실패 시 (0, 0).
fn current_cursor() -> (i32, i32) {
    let mut pt = POINT::default();
    // SAFETY: pt 는 로컬 스택 버퍼.
    let _ = unsafe { GetCursorPos(&mut pt) };
    (pt.x, pt.y)
}

// ────────────────────────────────────────────────────────────────────
// 폴링 — throw modifier 감지 + SnapService 구동
// ────────────────────────────────────────────────────────────────────

/// throw modifier 폴링 — Idle/Held 상태 전이에 따라 SnapService 호출.
fn poll_throw(
    snap_service: &Arc<SnapService>,
    config_store: &Arc<dyn ConfigStore>,
    state: &Arc<Mutex<InputState>>,
) {
    let config = match config_store.load() {
        Ok(c) => c,
        Err(_) => return,
    };
    let modifiers_held = check_modifiers(&config.throw.trigger_modifiers);

    let mut st = state.lock().unwrap();
    if modifiers_held && !st.throw_active {
        // Idle → Held 전이.
        st.throw_active = true;
        let (cx, cy) = current_cursor();
        st.origin = Some((cx, cy));
        drop(st);
        if let Err(e) = snap_service.on_modifier_pressed(cx, cy) {
            eprintln!("throw on_modifier_pressed 오류: {e}");
        }
    } else if modifiers_held && st.throw_active {
        // Held 유지 — origin 기준 delta 계산.
        if let Some((ox, oy)) = st.origin {
            let (cx, cy) = current_cursor();
            let dx = (cx - ox) as f64;
            let dy = (cy - oy) as f64;
            drop(st);
            if let Err(e) = snap_service.on_mouse_moved(cx, cy, dx, dy) {
                eprintln!("throw on_mouse_moved 오류: {e}");
            }
        }
    } else if !modifiers_held && st.throw_active {
        // Held → Idle 전이. cancel=false (정상 release → snap 실행).
        st.throw_active = false;
        let (cx, cy) = current_cursor();
        st.origin = None;
        drop(st);
        if let Err(e) = snap_service.on_modifier_released(false, cx, cy) {
            eprintln!("throw on_modifier_released 오류: {e}");
        }
    }
}

/// modifier 문자열 목록이 모두 눌려 있는지 `GetAsyncKeyState` 로 확인.
///
/// Win 키는 좌/우 어느 쪽이든 활성으로 간주한다. 목록의 모든 modifier 가
/// 눌려 있어야 true 를 반환한다 (AND 결합). 빈 목록은 false (활성 없음).
fn check_modifiers(mods: &[String]) -> bool {
    if mods.is_empty() {
        return false;
    }
    for m in mods {
        // SAFETY: GetAsyncKeyState 는 읽기 전용 조회. vk 는 상수값.
        let held = match m.as_str() {
            "Win" => unsafe {
                GetAsyncKeyState(VK_LWIN.0 as i32) < 0
                    || GetAsyncKeyState(VK_RWIN.0 as i32) < 0
            },
            "Alt" => unsafe { GetAsyncKeyState(VK_MENU.0 as i32) < 0 },
            "Ctrl" => unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 },
            "Shift" => unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 },
            _ => false,
        };
        if !held {
            return false;
        }
    }
    true
}
