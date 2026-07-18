# 개발 가이드

## 시작하기

```bash
# 의존성 설치
npm install

# 개발 모드 실행 (Tauri + Vite 핫 리로드)
npm run tauri dev

# 프로덕션 빌드
npm run tauri build

# Rust 테스트
cd src-tauri && cargo test

# 프론트엔드 타입 체크
npm run build
```

## 프로젝트 구조

```
rectangle-win/
├── src/                          # 프론트엔드 (Vue 3)
│   ├── App.vue                   # 루트 컴포넌트
│   ├── main.ts                   # 엔트리 포인트 (라우터, Pinia, i18n)
│   ├── components/               # 재사용 컴포넌트
│   │   ├── SnapCanvas.vue        # snap 영역 편집 캔버스 (vue-konva)
│   │   ├── SectorMapping.vue     # 8섹터 매핑 에디터
│   │   ├── SnapProperties.vue    # 영역 속성 패널
│   │   ├── SaveBar.vue           # 저장/리셋 바
│   │   └── ...
│   ├── pages/                    # 라우트 페이지
│   │   ├── General.vue           # 일반 설정
│   │   ├── Throw.vue             # Throw 스냅 설정
│   │   ├── SnapEditor.vue        # Snap 영역 에디터
│   │   ├── Keyboard.vue          # 키보드 스냅 설정
│   │   ├── Display.vue           # 디스플레이 설정
│   │   └── About.vue             # 정보
│   ├── entities/                 # Zod 스키마 + 타입 (Rust model.rs와 1:1)
│   │   ├── config.ts             # Zod 검증 스키마
│   │   └── default-config.ts     # 기본값
│   ├── features/                 # 비즈니스 로직
│   │   ├── api.ts                # Tauri IPC 래퍼
│   │   └── config-store.ts       # Pinia 설정 스토어
│   ├── i18n/                     # 다국어 (ko, en)
│   └── assets/                   # CSS, 폰트
│
├── src-tauri/                    # 백엔드 (Rust)
│   ├── src/
│   │   ├── lib.rs                # Tauri Builder + setup
│   │   ├── main.rs               # 엔트리 포인트
│   │   ├── domain/               # 순수 도메인 로직
│   │   │   ├── model.rs          # Config, SnapTarget, Direction, SectorMap
│   │   │   ├── geometry.rs       # 좌표 변환, sector 계산
│   │   │   ├── cursor_fsm.rs     # 커서 상태 머신 (Idle→Armed→Tracking)
│   │   │   ├── presets.rs        # 5가지 snap 프리셋
│   │   │   └── errors.rs         # 도메인 에러
│   │   ├── application/          # 애플리케이션 서비스
│   │   │   ├── snap_service.rs   # 마우스 throw 스냅
│   │   │   ├── keyboard_service.rs # 키보드 스냅
│   │   │   ├── ports.rs          # trait 정의 (4개)
│   │   │   ├── errors.rs         # 애플리케이션 에러
│   │   │   └── mock.rs           # 테스트용 mock
│   │   ├── infrastructure/       # Win32 어댑터
│   │   │   ├── win32_input.rs    # LL hook + 채널 작업 스레드
│   │   │   ├── win32_overlay.rs  # Direct2D Layered 창
│   │   │   ├── win32_window.rs   # 창 이동/크기/액션
│   │   │   ├── win32_monitor.rs  # 모니터 정보
│   │   │   └── toml_config.rs    # TOML 설정 저장소
│   │   └── presentation/         # Tauri IPC
│   │       ├── commands.rs       # 6개 명령 (get_config, save_config 등)
│   │       ├── state.rs          # AppState (서비스 조립)
│   │       └── tray.rs           # 시스템 트레이
│   ├── capabilities/             # Tauri 권한
│   ├── tauri.conf.json           # Tauri 설정
│   └── Cargo.toml                # Rust 의존성
│
├── docs/                         # 문서
│   ├── architecture.md           # 아키텍처 개요
│   └── development.md            # 이 파일
│
└── .agents/skills/               # Agent skills
```

## 코딩 규칙

### Rust
- 모든 unsafe 블록에 `// SAFETY:` 주석 필수
- `eprintln!` 대신 `log::error!` / `log::warn!` 사용 (tauri-plugin_log)
- 도메인 계층은 OS/IO 의존 금지 (純粋 로직만)
- trait(port)를 통해 의존성 주입
- `#[cfg(windows)]`로 Windows 전용 코드 격리

### TypeScript/Vue
- 모든 IPC 응답은 Zod로 런타임 검증 (`api.ts`)
- Vue reactive 객체를 `invoke`에 전달할 때 `JSON.parse(JSON.stringify(...))`로 변환
- `structuredClone` 사용 금지 (Vue reactive 호환 안 됨)
- Pinia store에서 `saved`/`draft` 패턴 사용

## 트러블슈팅

### 빌드 에러: EBUSY / resource busy
Vite가 `target/` 디렉토리를 감시하여 Cargo 빌드 중 `.exe` 파일 lock.
→ `vite.config.ts`의 `server.watch.ignored`에 `target/` 포함됨.

### Config 에러: missing field
프론트엔드 Zod 스키마와 Rust `model.rs`의 필드가 일치해야 함.
새 필드 추가 시 양쪽 모두 업데이트:
1. `src-tauri/src/domain/model.rs` — struct + Default
2. `src/entities/config.ts` — Zod schema
3. `src/entities/default-config.ts` — 기본값

### 오버레이 안 보임 / 깜빡임
- `draw_scene`이 `ShowWindow`보다 먼저 실행되어야 함 (DIB 갱신 후 표시)
- `show_reticle`에서 `redraw()` 호출하여 이전 상태 클리어
- config 캐시(`AtomicBool`)가 LL hook 콜백에서 올바른 값을 읽는지 확인

### LL hook 타임아웃 (입력 멈춤)
LL hook 콜백에서 무거운 작업(config 로드, D2D 렌더링)을 직접 수행하면
Windows가 300ms 후 훅을 자동 해제.
→ 모든 무거운 작업은 `mpsc::Sender`로 input-worker 스레드에 위임.
