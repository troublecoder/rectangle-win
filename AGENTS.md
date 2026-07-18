# AGENTS.md

에이전트가 이 프로젝트에서 작업할 때 참조하는 가이드입니다.

## 문서 참조

- [아키텍처 개요](docs/architecture.md) — 헥사고날 구조, 데이터 흐름, 스레드 모델
- [개발 가이드](docs/development.md) — 프로젝트 구조, 코딩 규칙, 트러블슈팅
- [README.md](README.md) — 프로젝트 소개, 기능, 기술 스택

## Skills (우선순위별)

작업 유형에 따라 다음 skills를 참조하세요:

### Tauri 백엔드 개발 (Rust)
1. **[tauri-v2](.agents/skills/tauri-v2/SKILL.md)** — Tauri v2 명령, 이벤트, 플러그인, 윈도우 관리, 권한 설정. Tauri IPC 추가/수정 시 필수.
2. **[rust-best-practices](.agents/skills/rust-best-practices/SKILL.md)** — Rust 코딩 베스트 프랙티스 (error handling, async, lifetime, trait 설계). `src-tauri/src/` 작업 시 참조.

### 프론트엔드 개발 (Vue + TypeScript)
3. **[nuxt-ui](.agents/skills/nuxt-ui/SKILL.md)** — Nuxt UI v4 컴포넌트 문서, 컨벤션, 디자인 시스템. UI 컴포넌트 추가/수정 시 필수.
4. **[vue-typescript](.agents/skills/vue-typescript/SKILL.md)** — Vue 3 + TypeScript 패턴, Composition API, 타입 안전성. `src/` 작업 시 참조.

## 핵심 파일 (자주 수정하는 곳)

### Rust 백엔드
| 파일 | 용도 | 수정 시 주의사항 |
|------|------|------------------|
| `src-tauri/src/domain/model.rs` | Config, SnapTarget 등 데이터 모델 | 필드 변경 시 `src/entities/config.ts` + `default-config.ts` 동기화 필수 |
| `src-tauri/src/application/snap_service.rs` | 마우스 throw snap 로직 | `inner` lock을 overlay 호출 전에 drop (데드락 방지) |
| `src-tauri/src/application/keyboard_service.rs` | 키보드 snap 로직 | ←/→ throw mapping 순회, ↑/↓ 액션 순환 |
| `src-tauri/src/infrastructure/win32_input.rs` | LL hook + 채널 작업 스레드 | 콜백 내 무거운 작업 금지 → 채널로 위임 |
| `src-tauri/src/infrastructure/win32_overlay.rs` | Direct2D Layered 창 | draw_scene을 ShowWindow보다 먼저 실행 |
| `src-tauri/src/infrastructure/win32_window.rs` | 창 이동/크기/액션 | SetWindowPos에 SWP_FRAMECHANGED 포함 |
| `src-tauri/src/presentation/commands.rs` | Tauri IPC 명령 | save_config 후 update_config 호출 |

### 프론트엔드
| 파일 | 용도 | 수정 시 주의사항 |
|------|------|------------------|
| `src/entities/config.ts` | Zod 스키마 | Rust model.rs와 1:1 대응 유지 |
| `src/features/api.ts` | Tauri IPC 래퍼 | Vue reactive → JSON.parse/stringify 변환 |
| `src/features/config-store.ts` | Pinia 설정 스토어 | structuredClone 사용 금지 |
| `src/components/SnapCanvas.vue` | snap 영역 편집 캔버스 | 단일 v-rect 재사용 패턴 |
| `src/pages/SnapEditor.vue` | snap 에디터 페이지 | 영역 선택 → 캔버스 갱신 |

## 중요 규칙

### 절대 하면 안 되는 것
1. **LL hook 콜백에서 config 로드/디스크 I/O** — 300ms 타임아웃으로 훅 자동 해제
2. **`structuredClone` 사용** — Vue reactive 객체 복제 불가 (DataCloneError)
3. **Rust 필드 추가 후 프론트엔드 스키마 미갱신** — IPC 역직렬화 실패 (missing field)
4. **오버레이 창을 show/hide 반복** — 깜빡임 발생, draw-before-show 패턴 유지

### 반드시 해야 하는 것
1. **config 변경 시 양쪽 업데이트** — `model.rs` + `config.ts` + `default-config.ts`
2. **unsafe 블록에 SAFETY 주석** — 모든 `unsafe`에 위험성 설명
3. **`log::error!` 사용** — `eprintln!` 대신 (tauri-plugin_log 연동)
4. **채널로 무거운 작업 위임** — LL hook → input-worker 스레드

## 빌드 & 테스트

```bash
# 개발 실행
npm run tauri dev

# Rust 테스트 (68 passed, 1 pre-existing failure)
cd src-tauri && cargo test

# 빌드
npm run tauri build
```

## 설정 파일

`%APPDATA%\rectangle-win\config.toml` — 삭제 시 기본값(Full 프리셋)으로 재생성
