#![cfg(windows)]

//! FancyZones 방식 저수준(LL) 키보드/마우스 훅 입력 리스너.
//!
//! `WH_KEYBOARD_LL` + `WH_MOUSE_LL` 글로벌 훅을 설치해
//! RegisterHotKey 없이도 키 입력을 삼키고 throw modifier 조합을 감지한다.
//!
//! - **키보드 snap**: throw modifier 조합(기본 Win+Alt)이 모두 눌린 상태에서
//!   방향키 DOWN 이 들어오면 `KeyboardService::on_direction_key` 호출 후
//!   `LRESULT(1)` 반환으로 키를 삼킨다. UP 은 그대로 통과.
//!   `general.override_win_snap` 활성 시 Win+방향키(Alt 없이)도 삼킨다.
//! - **마우스 throw**: throw modifier 조합이 Idle→Held 로 전이되면 origin 캡처 +
//!   `on_modifier_pressed`. WM_MOUSEMOVE 로 delta 계산 + `on_mouse_moved`.
//!   Held→Idle 전이(또는 우/중 버튼 DOWN) 시 `on_modifier_released`.
//! - **Config 캐싱**: 콜백 안에서 디스크 I/O 를 하지 않도록 설정값을 static
//!   `AtomicBool` 로 캐시한다. `update_config` 로 갱신 (startup + 저장 시).
//!
//! LL 훅 콜백이 발화하려면 설치 스레드에서 `GetMessageW` 메시지 루프가
//! 돌아야 한다. 전용 "win32-input" 스레드 하나가 루프를 소유한다.
//!
//! 단위 테스트는 실제 OS 입력 상호작용이 필요하므로 작성하지 않는다
//! (기존 win32_window/win32_monitor/win32_overlay 패턴과 동일).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_DOWN, VK_LEFT, VK_LWIN, VK_MENU, VK_RIGHT, VK_RWIN, VK_SHIFT,
    VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetCursorPos,
    HHOOK, HC_ACTION, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, MSG, PostMessageW, PostThreadMessageW,
    RegisterClassExW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_DISPLAYCHANGE, WM_KEYDOWN, WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_QUIT,
    WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_USER,
};

use crate::application::keyboard_service::KeyboardService;
use crate::application::ports::ConfigStore;
use crate::application::snap_service::SnapService;
use crate::domain::model::{Config, Direction};
use crate::infrastructure::win32_monitor::Win32MonitorProvider;

// ────────────────────────────────────────────────────────────────────
// Config 캐시 (static AtomicBool) — 콜백 안에서 디스크 I/O 금지
// ────────────────────────────────────────────────────────────────────

/// throw trigger 조합에 Win 키가 포함되어 있는지 (LWin/RWin 어느 쪽이든).
static CACHED_WIN: AtomicBool = AtomicBool::new(true);
/// throw trigger 조합에 Alt 가 포함되어 있는지.
static CACHED_ALT: AtomicBool = AtomicBool::new(true);
/// throw trigger 조합에 Ctrl 이 포함되어 있는지.
static CACHED_CTRL: AtomicBool = AtomicBool::new(false);
/// throw trigger 조합에 Shift 가 포함되어 있는지.
static CACHED_SHIFT: AtomicBool = AtomicBool::new(false);
/// `general.override_win_snap` — Win+방향키(Alt 없이)도 우리 snap 으로 삼킬지.
static CACHED_OVERRIDE_WIN_SNAP: AtomicBool = AtomicBool::new(false);
/// `keyboard.enabled` — 키보드 snap 기능 활성화 여부.
static CACHED_KB_ENABLED: AtomicBool = AtomicBool::new(true);

/// 캐시된 config 값을 갱신한다 (자유 함수 — impl 메서드와 분리).
///
/// - `start()` 시작 시 1회 (config 로드 후)
/// - `save_config` 명령으로 저장될 때마다 (commands.rs 에서 `Win32InputListener::update_config` 경유)
///
/// 콜백 내 디스크 I/O 를 피하기 위해 미리 atomics 로 복사해 둔다.
fn update_config_static(config: &Config) {
    let mods = &config.throw.trigger_modifiers;
    CACHED_WIN.store(mods.iter().any(|m| m == "Win"), Ordering::Relaxed);
    CACHED_ALT.store(mods.iter().any(|m| m == "Alt"), Ordering::Relaxed);
    CACHED_CTRL.store(mods.iter().any(|m| m == "Ctrl"), Ordering::Relaxed);
    CACHED_SHIFT.store(mods.iter().any(|m| m == "Shift"), Ordering::Relaxed);
    CACHED_OVERRIDE_WIN_SNAP.store(config.general.override_win_snap, Ordering::Relaxed);
    CACHED_KB_ENABLED.store(config.keyboard.enabled, Ordering::Relaxed);
}

// ────────────────────────────────────────────────────────────────────
// 훅 컨텍스트 — 콜백이 접근하는 서비스 참조
// ────────────────────────────────────────────────────────────────────

/// LL 훅 콜백이 접근하는 공유 컨텍스트.
struct HookContext {
    snap_service: Arc<SnapService>,
    keyboard_service: Arc<KeyboardService>,
    monitor_provider: Arc<Win32MonitorProvider>,
    /// message-only 창 핸들 — PostMessage 타겟. AtomicPtr 로 스레드 간 공유.
    hwnd: std::sync::atomic::AtomicPtr<std::ffi::c_void>,
    /// throw origin / 활성 상태 추적 (콜백 스레드에서만 접근 → Mutex 불필요).
    origin: std::cell::UnsafeCell<Option<(i32, i32)>>,
    throw_active: AtomicBool,
}

// 콜백은 단일 스레드(메시지 루프 스레드)에서만 호출되므로 UnsafeCell 접근 안전.
unsafe impl Sync for HookContext {}

static HOOK_CTX: OnceLock<HookContext> = OnceLock::new();

/// 현재 throw 가 활성인지 (콜백 스레드에서만).
fn throw_active() -> bool {
    HOOK_CTX.get().map_or(false, |c| c.throw_active.load(Ordering::Relaxed))
}

/// throw origin 설정/해제 (콜백 스레드에서만).
fn set_throw(active: bool, origin: Option<(i32, i32)>) {
    if let Some(ctx) = HOOK_CTX.get() {
        ctx.throw_active.store(active, Ordering::Relaxed);
        // SAFETY: 메시지 루프 스레드에서만 접근 (LL 훅은 동일 스레드에서 직렬 호출).
        unsafe {
            *ctx.origin.get() = origin;
        }
    }
}

/// throw origin 조회 (콜백 스레드에서만).
fn throw_origin() -> Option<(i32, i32)> {
    // SAFETY: 메시지 루프 스레드에서만 접근.
    HOOK_CTX.get().map(|c| unsafe { *c.origin.get() }).unwrap_or(None)
}

// ────────────────────────────────────────────────────────────────────
// Win32InputListener
// ────────────────────────────────────────────────────────────────────

/// 입력 리스너. `start()` 로 전용 스레드를 시작한다.
///
/// 스레드 종료를 위해 thread id 를 보관하며, `stop()` 이 WM_QUIT 을 게시하면
/// 메시지 루프가 종료되며 훅을 해제한다.
pub struct Win32InputListener {
    thread_id: u32,
}

impl Win32InputListener {
    /// 입력 리스너 스레드 시작.
    ///
    /// 전용 "win32-input" 스레드에서 message-only 창을 생성하고 LL 훅을 설치한 뒤
    /// GetMessageW 루프를 돈다. SnapService/KeyboardService/MonitorProvider 는
    /// `OnceLock<HookContext>` static 으로 이동되어 콜백이 접근한다.
    pub fn start(
        snap_service: Arc<SnapService>,
        keyboard_service: Arc<KeyboardService>,
        config_store: Arc<dyn ConfigStore>,
        monitor_provider: Arc<Win32MonitorProvider>,
    ) -> Self {
        // Config 캐시 1차 갱신 — 디스크에서 로드 가능하면 캐시에 반영.
        // 훅 설치 전에 수행하여 콜백이 시작과 동시에 올바른 config 를 본다.
        if let Ok(cfg) = config_store.load() {
            update_config_static(&cfg);
        }

        // OnceLock 에 컨텍스트 등록 — hwnd 없이 먼저 세팅.
        // hwnd 는 run_message_loop 안에서 생성 후 업데이트.
        let _ = HOOK_CTX.set(HookContext {
            snap_service: snap_service.clone(),
            keyboard_service: keyboard_service.clone(),
            monitor_provider: monitor_provider.clone(),
            hwnd: std::sync::atomic::AtomicPtr::new(std::ptr::null_mut()),
            origin: std::cell::UnsafeCell::new(None),
            throw_active: AtomicBool::new(false),
        });

        // thread id 를 부모에게 전달하기 위한 슬롯.
        let thread_id_slot: Arc<std::sync::Mutex<Option<u32>>> =
            Arc::new(std::sync::Mutex::new(None));
        let slot_for_thread = thread_id_slot.clone();

        thread::Builder::new()
            .name("win32-input".into())
            .spawn(move || {
                // 자신의 thread id 를 부모에게 알림.
                let tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
                {
                    let mut slot = slot_for_thread.lock().unwrap();
                    *slot = Some(tid);
                }

                if let Err(e) = run_message_loop() {
                    eprintln!("입력 리스너 오류: {e}");
                }
            })
            .expect("입력 리스너 스레드 시작 실패");

        // 스레드가 thread id 를 기록할 때까지 대기 (start() 는 동기적으로 반환해야 함).
        let thread_id = loop {
            if let Some(tid) = *thread_id_slot.lock().unwrap() {
                break tid;
            }
            thread::sleep(Duration::from_millis(1));
        };

        Win32InputListener { thread_id }
    }

    /// Config 갱신 — 외부(save_config)에서 호출. static atomics 에 반영.
    pub fn update_config(config: &Config) {
        // 주의: 이 impl 메서드와 동일한 이름의 자유 함수를 호출하므로 전체 경로 필요.
        update_config_static(config);
    }

    /// 입력 리스너 정지 — 스레드에 WM_QUIT 게시.
    ///
    /// 메시지 루프가 WM_QUIT 을 받으면 훅 해제 후 종료된다.
    /// 스레드 자체의 join 은 수행하지 않는다 (best-effort).
    #[allow(dead_code)]
    pub fn stop(&self) {
        // SAFETY: PostThreadMessageW 는 thread id 가 유효하면 안전.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// 메시지 루프
// ────────────────────────────────────────────────────────────────────

/// message-only 창 + LL 훅 설치 후 GetMessageW 루프.
///
/// LL 훅 콜백이 발화하려면 GetMessageW 처럼 스레드 메시지 큐에서
/// 블로킹 대기하는 루프가 필요하다.
fn run_message_loop() -> windows::core::Result<()> {
    // message-only 창 생성 (WM_DISPLAYCHANGE 수신용).
    let hwnd = create_message_window()?;

    // HookContext 의 hwnd 업데이트 (start() 에서 null 로 세팅됨).
    if let Some(ctx) = HOOK_CTX.get() {
        ctx.hwnd.store(hwnd.0, Ordering::Relaxed);
    }

    // LL 훅 설치 — 반드시 메시지 루프 스레드에서.
    // SAFETY: HMODULE(None) — LL 훅은 DLL 이 아닌 자체 프로세스에서 설치 가능
    // (콜백 함수 주소는 프로세스 주소 공간에 있으므로). dwthreadid=0 은
    // "현재 스레드" 를 의미 (LL 훅은 필수 — 다른 스레드의 입력 큐를 후크 불가).
    // SetWindowsHookExW 의 hmod 파라미터는 Param<HINSTANCE> 를 받으나 LL 훅은
    // 무시되므로 None 안전.
    let kb_hook: HHOOK =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0)? };
    let mouse_hook: HHOOK =
        unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0)? };
    eprintln!("LL 훅 설치 완료 (keyboard + mouse)");

    // GetMessageW 루프 — LL 훅 콜백 발화 조건.
    let mut msg = MSG::default();
    loop {
        // SAFETY: msg 는 로컬 스택 버퍼. hwnd=None(0) → 모든 창 + 스레드 메시지.
        // GetMessageW 는 WM_QUIT 수신 시 BOOL(0) 반환 → 루프 종료.
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !ret.as_bool() {
            // WM_QUIT 또는 오류.
            break;
        }

        // WM_DISPLAYCHANGE 처리 (큐에서 꺼낸 후).
        // LL 훅과 무관하게 디스플레이 변경 시 MonitorProvider 캐시를 무효화.
        if msg.message == WM_DISPLAYCHANGE {
            if let Some(ctx) = HOOK_CTX.get() {
                ctx.monitor_provider.invalidate();
            }
        }

        // throw modifier 전이 처리 — LL 훅 콜백에서 PostMessage 로 위임된 작업.
        // 콜백 내에서 직접 호출하면 config 로드/D2D 렌더링으로 LowLevelHooksTimeout 초과.
        if msg.message == WM_USER + 1 {
            // throw pressed
            if let Some(ctx) = HOOK_CTX.get() {
                let (cx, cy) = current_cursor();
                if let Err(e) = ctx.snap_service.on_modifier_pressed(cx, cy) {
                    eprintln!("throw on_modifier_pressed 오류: {e}");
                }
            }
        } else if msg.message == WM_USER + 2 {
            // throw released
            if let Some(ctx) = HOOK_CTX.get() {
                let (cx, cy) = current_cursor();
                if let Err(e) = ctx.snap_service.on_modifier_released(false, cx, cy) {
                    eprintln!("throw on_modifier_released 오류: {e}");
                }
            }
        }

        // SAFETY: msg 는 GetMessageW 로 채운 유효 메시지.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // 훅 해제.
    // SAFETY: kb_hook/mouse_hook 은 설치 시 얻은 유효한 HHOOK.
    unsafe {
        let _ = UnhookWindowsHookEx(kb_hook);
        let _ = UnhookWindowsHookEx(mouse_hook);
    }
    eprintln!("LL 훅 해제 완료");
    Ok(())
}

/// message-only 창 생성 (DefWindowProcW 만 호출하는 최소 창).
fn create_message_window() -> windows::core::Result<HWND> {
    use windows::core::w;
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW,
    };

    let class_name = w!("RectangleWinInput");

    // SAFETY: WNDCLASSEXW zero-init 후 필요 필드만 채운다.
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
/// WM_DISPLAYCHANGE 처리는 GetMessageW 루프에서 직접 수행한다.
unsafe extern "system" fn input_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

// ────────────────────────────────────────────────────────────────────
// LL 키보드 훅 콜백
// ────────────────────────────────────────────────────────────────────

/// WH_KEYBOARD_LL 콜백.
///
/// - 방향키 + throw modifier (또는 override_win_snap 시 Win+방향키) DOWN →
///   `on_direction_key` 호출 + 삼킴(LRESULT(1)). UP 은 통과.
/// - throw modifier 조합 전이 감지 → SnapService::on_modifier_pressed/released.
/// - 그 외 → CallNextHookEx (통과).
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // HC_ACTION(0) 일 때만 의미 있는 키 이벤트.
    if code != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let ctx = match HOOK_CTX.get() {
        Some(c) => c,
        None => return unsafe { CallNextHookEx(None, code, wparam, lparam) },
    };

    // SAFETY: lparam 은 KBDLLHOOKSTRUCT 포인터 (LL 훅 계약). CopyType.
    let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let vk = kb.vkCode;

    let w = wparam.0 as u32;
    let is_down = w == WM_KEYDOWN || w == WM_SYSKEYDOWN;

    // 1) throw modifier 조합 전이 감지 (modifier 변화는 이 키 이벤트로 추론).
    update_throw_state(ctx);

    // 2) 방향키 처리 — 키보드 snap. DOWN 만 삼키고 UP 은 통과(명시 처리 없음).
    let direction = vk_to_direction(vk);
    if let Some(dir) = direction {
        if is_down && handle_direction_key(ctx, dir, vk) {
            // 삼킴 — 다음 훅으로 전달하지 않음.
            return LRESULT(1);
        }
        // UP 또는 처리하지 않은 DOWN 은 통과.
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// 방향키 vkCode → Direction. 방향키가 아니면 None.
fn vk_to_direction(vk: u32) -> Option<Direction> {
    Some(match vk {
        x if x == VK_LEFT.0 as u32 => Direction::Left,
        x if x == VK_RIGHT.0 as u32 => Direction::Right,
        x if x == VK_UP.0 as u32 => Direction::Up,
        x if x == VK_DOWN.0 as u32 => Direction::Down,
        _ => return None,
    })
}

/// 방향키 DOWN 처리. 삼킬지 여부 반환.
///
/// 삼키는 조건 (캐시된 config 기반):
/// - keyboard.enabled == true 이고
/// - (throw modifier 조합이 모두 눌림) OR
///   (override_win_snap && Win 눌림 && Alt 안 눌림)
fn handle_direction_key(ctx: &HookContext, dir: Direction, _vk: u32) -> bool {
    if !CACHED_KB_ENABLED.load(Ordering::Relaxed) {
        return false;
    }

    let mods_held = check_throw_modifiers();
    let override_held = CACHED_OVERRIDE_WIN_SNAP.load(Ordering::Relaxed)
        && win_pressed()
        && !alt_pressed();

    if !mods_held && !override_held {
        return false;
    }

    let (cx, cy) = current_cursor();
    if let Err(e) = ctx.keyboard_service.on_direction_key(dir, cx, cy) {
        eprintln!("키보드 snap 오류: {e}");
    }
    true
}

// ────────────────────────────────────────────────────────────────────
// throw modifier 상태 전이 (FancyZones 방식)
// ────────────────────────────────────────────────────────────────────

/// throw modifier 조합 전이를 검사해 메시지 펌프로 처리 위임.
///
/// LL 훅 콜백에서 직접 SnapService 호출(config 로드 + D2D 렌더링)을 하면
/// LowLevelHooksTimeout(300ms)에 걸려 지연/해제된다. 대신 PostMessage 로
/// 사용자 정의 메시지를 보내고 메시지 펌프에서 처리한다.
fn update_throw_state(ctx: &HookContext) {
    let held = check_throw_modifiers();
    let was_active = throw_active();

    if held && !was_active {
        // Idle → Held 전이.
        let (cx, cy) = current_cursor();
        set_throw(true, Some((cx, cy)));
        // 메시지 펌프로 위임 — WM_USER+1 = throw pressed.
        let hwnd = HWND(ctx.hwnd.load(Ordering::Relaxed));
        unsafe {
            PostMessageW(hwnd, WM_USER + 1, WPARAM(0), LPARAM(0));
        }
    } else if !held && was_active {
        // Held → Idle 전이. cancel=false (정상 release → snap 실행).
        set_throw(false, None);
        // 메시지 펌프로 위임 — WM_USER+2 = throw released.
        let hwnd = HWND(ctx.hwnd.load(Ordering::Relaxed));
        unsafe {
            PostMessageW(hwnd, WM_USER + 2, WPARAM(0), LPARAM(0));
        }
    }
}

/// 캐시된 throw modifier 조합이 모두 눌려 있는지.
///
/// 빈 조합(Win/Alt/Ctrl/Shift 모두 미포함)은 활성으로 간주하지 않는다.
fn check_throw_modifiers() -> bool {
    let want_win = CACHED_WIN.load(Ordering::Relaxed);
    let want_alt = CACHED_ALT.load(Ordering::Relaxed);
    let want_ctrl = CACHED_CTRL.load(Ordering::Relaxed);
    let want_shift = CACHED_SHIFT.load(Ordering::Relaxed);

    // 빈 조합이면 활성 없음.
    if !want_win && !want_alt && !want_ctrl && !want_shift {
        return false;
    }

    if want_win && !win_pressed() {
        return false;
    }
    if want_alt && !alt_pressed() {
        return false;
    }
    if want_ctrl && !ctrl_pressed() {
        return false;
    }
    if want_shift && !shift_pressed() {
        return false;
    }
    true
}

/// Win(LWin 또는 RWin) 이 눌려 있는지.
fn win_pressed() -> bool {
    // SAFETY: GetAsyncKeyState 는 읽기 전용 조회.
    unsafe {
        GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0
    }
}

/// Alt 눌림.
fn alt_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_MENU.0 as i32) < 0 }
}

/// Ctrl 눌림.
fn ctrl_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 }
}

/// Shift 눌림.
fn shift_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 }
}

// ────────────────────────────────────────────────────────────────────
// LL 마우스 훅 콜백
// ────────────────────────────────────────────────────────────────────

/// WH_MOUSE_LL 콜백.
///
/// - WM_MOUSEMOVE: throw 활성 시 origin 기준 delta 계산 → on_mouse_moved.
/// - WM_RBUTTONDOWN / WM_MBUTTONDOWN: throw 활성 시 snap 취소 (cancel=true).
/// - 그 외 → CallNextHookEx (통과). 마우스 이벤트는 삼키지 않는다.
unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != HC_ACTION as i32 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let ctx = match HOOK_CTX.get() {
        Some(c) => c,
        None => return unsafe { CallNextHookEx(None, code, wparam, lparam) },
    };

    // SAFETY: lparam 은 MSLLHOOKSTRUCT 포인터 (LL 훅 계약). CopyType.
    let ms = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
    let w = wparam.0 as u32;

    if throw_active() {
        match w {
            WM_MOUSEMOVE => {
                let (cx, cy) = (ms.pt.x, ms.pt.y);
                if let Some((ox, oy)) = throw_origin() {
                    let dx = (cx - ox) as f64;
                    let dy = (cy - oy) as f64;
                    if let Err(e) = ctx.snap_service.on_mouse_moved(cx, cy, dx, dy) {
                        eprintln!("throw on_mouse_moved 오류: {e}");
                    }
                }
            }
            WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                // 취소 — snap 실행 없이 throw 종료.
                let (cx, cy) = (ms.pt.x, ms.pt.y);
                set_throw(false, None);
                if let Err(e) = ctx.snap_service.on_modifier_released(true, cx, cy) {
                    eprintln!("throw on_modifier_released(cancel) 오류: {e}");
                }
            }
            _ => {}
        }
    }

    // 마우스 입력은 항상 통과 (삼키지 않음).
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

// ────────────────────────────────────────────────────────────────────
// 유틸
// ────────────────────────────────────────────────────────────────────

/// 현재 커서 좌표 (GetCursorPos). 실패 시 (0, 0).
fn current_cursor() -> (i32, i32) {
    let mut pt = POINT::default();
    // SAFETY: pt 는 로컬 스택 버퍼.
    let _ = unsafe { GetCursorPos(&mut pt) };
    (pt.x, pt.y)
}
