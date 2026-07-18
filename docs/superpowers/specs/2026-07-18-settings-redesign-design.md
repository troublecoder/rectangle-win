# Settings 화면 전면 재설계

**날짜:** 2026-07-18
**상태:** 승인 대기 → 구현 대기
**관련 커밋:** `28e47bb` (입력/오버레이 전면 교체 이후 프론트엔드 설정 UI)

## 배경 및 동기

현재 설정 화면은 Nuxt UI의 dashboard 컴포넌트를 전혀 사용하지 않고 손수 div를
조립했고, 다음 문제가 있다.

1. **헤더 텍스트 3중 중복** — 사이드바 라벨 = 페이지 헤더 = 내부 `PageHeader`
   타이틀이 모두 같은 값.
2. **언어 동기화 버그** — `i18n.locale`(기본 `'en'`)과
   `config.general.language`(기본 `'ko'`)이 별개 소스. 사이드바
   `ULocaleSelect`는 `locale`만, General 페이지 `USelect`는
   `general.language`만 보고 있어 화면 표시와 셀렉트 값이 어긋남.
3. **Trigger modifier UX 부적절** — `USelectMenu` 콤보박스로 편집하지만
   핫키 캡처 방식이 자연스러움. 백엔드는 이미 임의 조합을 지원함에도
   UI만 뒤떨어져 있음.
4. **Snap Editor 불필요 기능** — 프리셋 셀렉트, Import/Export 버튼, Sector
   Mapping 탭이 있으나 실사용 관점에서 의미가 없거나 다른 페이지에 귀속됨.
5. **Snap Preview 색상 단일** — throw와 long throw가 같은 색상을 공유하여
   시각적 구분이 안 됨.
6. **미사용 설정 존재** — `keyboard.cycle_timeout_ms`가 Rust에 정의만
   있고 사용처 없음. `overlay.sector_count`는 8 기준 동작.

## 목표

- Nuxt UI dashboard 컴포넌트(`UDashboardGroup`/`UDashboardSidebar`/
  `UDashboardPanel`/`UDashboardNavbar`)를 Vue+Vite 환경에서 직접 사용.
- 페이지 수를 6개에서 5개로 축소.
- 헤더/타이틀 중복 제거 (단일 진실).
- 언어 단일 진실 공급원 확립.
- 핫키 캡처 UX 도입.
- Snap Editor를 `UTable` expandable rows로 단순화.
- Snap Preview 색상 throw/long throw 분리 (백엔드 포함).

## 비목표

- 백엔드 LL hook 동작 자체 변경 없음.
- `trigger_modifiers` 임의 조합 지원은 이미 되어 있으므로 백엔드 로직
  수정 최소화.
- i18n 메시지 카탐로그 대폭 확장은 하지 않음 (기존 키 정리 + 신규 키
  최소 추가만).

## 테마 정책 (기본 테마로 회귀)

Catppuccin 커스텀 테마를 완전히 제거하고 Nuxt UI 기본 테마를 사용한다.
**Pretendard 폰트 설정만 유지**한다.

### 제거 대상

| 파일 / 설정 | 작업 |
|---|---|
| `src/assets/css/_catppuccin.css` | **삭제** |
| `src/assets/css/main.css` 의 `@import "./_catppuccin.css";` | **삭제** |
| `scripts/gen-catppuccin-theme.js` (및 관련 npm script) | **삭제** |
| `vite.config.ts` 의 `ui({ ui: { colors: { ... } } })` block | **제거** (7개 색상 매핑 전부) |

### 유지 대상

- `src/assets/css/main.css`의 `@import "tailwindcss";`, `@import "@nuxt/ui";`
- Pretendard `@font-face` 4종 (Regular/Medium/SemiBold/Bold) 및
  `@theme static { --font-sans: "Pretendard", ... }`
- `src/assets/fonts/Pretendard-*.woff2` 파일들

### 결과

- `vite.config.ts` 의 `ui()` 호출은 인자 없이 `ui()` 만 사용.
- 모든 7개 semantic 색상이 Nuxt UI 기본값(green/blue/slate 계열)로 동작.
- 다크/라이트 모드는 `UColorModeButton`으로 전환, 기본 `dark` 유지.
- 컴포넌트 색상 지정 시 semantic 색상(`primary`, `neutral`, `error` 등)만
  사용 — raw palette(`text-gray-500`) 금지.

### 패키지 정리

`gen-catppuccin-theme.js` 가 사용하던 의존성(있다면) `package.json`에서
제거 검토. `package.json`의 `scripts` 항목 중 테마 생성 관련 제거.

## 시각적 완성도 가이드라인

"엄청 이쁘게" 보여야 한다는 요구에 대한 구체적 지침. 모두 Nuxt UI 컴포넌트
기반으로 달성.

### 핵심 원칙: flat + 소제목 (card overuse 금지)

Nuxt UI component-selection 가이드라인을 따른다:
> "Don't overuse `UCard` — plain content with spacing is often better
> than wrapping everything in cards"

**`UPageCard`는 랜딩 피처 카드용이므로 설정 화면에서 절대 사용 금지.**
기존 `USection.vue`(UCard 래퍼)도 제거한다.

설정값 섹션은 다음 패턴으로 통일:
```
UFormField 들의 flat 세로 나열
└─ 섹션 구분 = 소제목 + USeparator
```

### 레이아웃 구조 (페이지별)

```vue
<UDashboardPanel>
  <template #header>
    <UDashboardNavbar title="일반">
      <template #right><SaveActions .../></template>
    </UDashboardNavbar>
  </template>
  <template #body>
    <UContainer class="max-w-3xl py-8 space-y-10">
      <!-- 섹션 1: 시작 -->
      <section class="space-y-4">
        <div class="flex items-center gap-2">
          <UIcon name="i-lucide-power" class="size-4 text-primary" />
          <h3 class="text-sm font-medium text-muted">시작</h3>
        </div>
        <USeparator />
        <UFormField label="..." description="...">
          <USwitch ... />
        </UFormField>
      </section>
      <!-- 섹션 2, 3, ... -->
    </UContainer>
  </template>
</UDashboardPanel>
```

### 섹션 시각 요소

- **소제목**: `UIcon`(size-4, semantic 색상) + `h3 text-sm font-medium
  text-muted` 가로 정렬.
- **구분선**: `USeparator` (소제목 바로 아래).
- **섹션 간격**: `space-y-10` (넉넉한 whitespace).
- **필드 간격**: 섹션 내 `space-y-4`.

### 예외: 카드 사용이 타당한 영역

독립된 데이터 영역(테이블/매핑)만 `UCard`로 감싼다:
- SnapEditor의 `UTable` 영역.
- Throw의 Sector 매핑 표.
- About의 앱 정보 영역.

단순 폼 필드 그룹(시작/트레이/언어/레티클/커서/핫키)은 flat.

### 사이드바

- `UDashboardSidebar` 고정 폭 (약 240px).
- `#header`: 앱 아이콘(`size-5 text-primary`) + 이름 +
  `UColorModeButton` (우측 끝).
- `#default`: `UNavigationMenu orientation="vertical"` 아이콘 + 라벨.
- `#footer`: `USeparator` 아래 활성화 토글 행 + 종료 버튼 행
  (`space-y-3`).

### 폼 필드 컨벤션

- `UFormField`의 `label` / `description` / `#hint` 구조 일관 유지.
- **USwitch**: `UFormField` 안에서 label 행에 가로 배치 —
  `<div class="flex items-center justify-between">` 로 label 좌, switch 우.
- **USlider**: 항상 `#hint` slot으로 현재값 표시 (예: `400px`, `50%`).
- **UColorPicker**: 작은 미리보기 색상 칩과 함께 배치.
- **USelect/UInput**: `class="w-full"` 또는 적절한 폭.

### 마이크로인터랙션

- `UButton` `loading` 상태 활용 (저장 중 스피너).
- `UTable` expandable row 애니메이션 = Nuxt UI 기본값.
- 다호/라이트 전환 = 사이드바 헤더 `UColorModeButton` (기본 dark).

### 타이포그래피

- 페이지 타이틀: `UDashboardNavbar` `title` prop (자동 스타일링).
- 섹션 소제목: `h3 text-sm font-medium text-muted`.
- 필드 label: `UFormField` 기본값.
- 본문/설명: `text-default` / `text-muted`.

### 색상 사용 규칙

- 저장: `color="primary"`, `variant="solid"`.
- 초기화: `color="neutral"`, `variant="ghost"`.
- 삭제: `color="error"`, `variant="ghost"`.
- 배지(area/action): `color="primary"` / `color="info"`, `variant="soft"`.
- 섹션 아이콘: `text-primary` / `text-muted` (semantic).

## 전체 구조

### 컴포넌트 트리

```
UApp
└── UDashboardGroup
    ├── UDashboardSidebar (고정형, collapsible 비활성화)
    │   ├── #header: 앱 아이콘 + 이름 + UColorModeButton
    │   ├── #default: UNavigationMenu vertical (5개 항목)
    │   └── #footer: 스냅 활성화 토글 + 종료 버튼
    └── <RouterView/> → 각 페이지
        └── UDashboardPanel
            ├── #header: UDashboardNavbar title=페이지명
            │   └── #right: 저장/초기화 버튼 (+ 페이지별 추가 액션)
            └── #body: UContainer > flat 섹션들 (소제목 + USeparator + UFormField)
```

### 사이드바 footer 구성

- **스냅 활성화 토글**: `keyboard.enabled` 에 바인딩. `USwitch` + 라벨
  ("스냅 활성화" / "Enable snap"). 앱 전체 스냅 on/off. 즉시 저장
  (`store.save()` 호출, dirty 대기 없이) — 활성화 토글은 설정 드래프트와
  별개로 즉시 반영되는 게 자연스러움.
- **종료 버튼**: `UButton` ("종료" / 아이콘 `i-lucide-power`),
  `color="error"`, `variant="ghost"`.
- 기존 "일시정지" 버튼은 제거 (활성화 토글이 동일 역할 수행).

> footer는 단일 `UPageCard` `variant="subtle"` 또는 단순 div로 감싸며,
> 활성화 토글 행과 종료 버튼 행을 `space-y-3`으로 분리.

### 라우트 (5개로 축소)

| 경로 | 페이지 | 비고 |
|---|---|---|
| `/general` | General | 시작/트레이/언어 |
| `/throw` | Throw | 핫키 캡처 + Long Throw + Sector 매핑 |
| `/snap-editor` | SnapEditor | Snap Areas만 (UTable expandable) |
| `/display` | Display | 레티클 + 커서 + Snap Preview 색상 |
| `/about` | About | 현행 유지 (Navbar로만 마이그레이션) |

**삭제**: `/keyboard` 라우트 및 `Keyboard.vue`.

### 파일 변경 요약

| 작업 | 파일 |
|---|---|
| **삭제** | `src/components/SettingsLayout.vue`, `src/components/PageHeader.vue`, `src/components/SaveBar.vue`, `src/components/USection.vue`, `src/components/SectorMapping.vue`, `src/pages/Keyboard.vue` |
| **신규** | `src/layouts/SettingsLayout.vue` (UDashboard 기반, App.vue가 사용), `src/components/SaveActions.vue` (UDashboardNavbar `#right`용), `src/components/UHotkeyInput.vue`, `src/components/ColorLockField.vue` |
| **재작성** | `src/components/SnapCanvas.vue` (축소형), `src/components/SnapProperties.vue` (UTable 확장용) |
| **수정** | `src/App.vue`, `src/pages/General.vue`, `src/pages/Throw.vue`, `src/pages/SnapEditor.vue`, `src/pages/Display.vue`, `src/pages/About.vue`, `src/main.ts` (라우트), `src/i18n/index.ts`, `src/i18n/locales/{en,ko}.ts`, `src/entities/config.ts`, `src/entities/default-config.ts`, `src/features/config-store.ts` |

## 페이지별 설계

### General 페이지

`UDashboardNavbar title="일반"`, `#right`에 `SaveActions`.

| 섹션 | 필드 | 컴포넌트 |
|---|---|---|
| 시작 | `launch_at_login`, `start_minimized` | USwitch |
| 시스템 트레이 | `show_in_tray` | USwitch |
| 언어 | `general.language` | USelect (English / 한국어) |

**언어 단일 진실 공급원 규칙**:
- `config.general.language`가 단일 소스.
- `changeLanguage(lang)`은 `store.draft.general.language = lang` 및
  `locale.value = lang` **둘 다** 갱신.
- 앱 시작 시 `store.load()` 이후 `locale.value = config.general.language`
  적용 (`config-store`의 `load()` 내 또는 watcher).
- `i18n/index.ts` 기본 `locale`을 `'ko'`로 변경 (default-config와 일치).
- 사이드바 `ULocaleSelect` 제거.

### Throw 페이지

`UDashboardNavbar title="스로우"`.

| 섹션 | 필드 | 컴포넌트 |
|---|---|---|
| 핫키 (Trigger) | `throw.trigger_modifiers: string[]` | `UHotkeyInput` (커스텀, flat) |
| Long Throw | `throw.long_throw.enabled` (USwitch), `throw.long_throw.distance` (USlider, 100~1000 step 50) | enabled일 때만 노출 |
| Sector 매핑 | `throw.mapping`, `throw.long_throw.mapping` | `UCard`로 감싼 매핑 표 (데이터 영역, 여기로 이동, 8섹터 고정) |

**UHotkeyInput 컴포넌트 동작**:
1. 비활성 상태: 현재 modifier 조합 표시 (예: `Win + Alt` 배지).
2. 클릭 시 캡처 모드 진입 — 다음 `keydown` 이벤트에서 modifier 키
   추출 (e.key / e.code 기반, Win/Alt/Ctrl/Shift 한정).
3. 일반 키 단독 입력은 무시 (modifier 없으면 거부). 빈 조합은 백엔드가
   거부하므로 클라이언트에서도 저장 막기.
4. ESC: 캡처 취소.
5. 선택된 modifier 배열을 `string[]` (`['Win','Alt']` 형태)로 emit.
6. 백엔드 `trigger_modifiers` Vec<String> 호환 — `check_throw_modifiers()`
   (`win32_input.rs:459`)가 Win/Alt/Ctrl/Shift 토큰을 추출해 검사하므로
   포맷 그대로 유지.

**SectorMapping UI** (Throw 페이지로 이동):
- 8 섹터(↘↓↙←↖↑↗→) × 2 세트(throw / long throw).
- 각 행: 섹터 화살표 라벨 + `USelect` (snap areas 목록에서 선택).
- `sector_count`는 8 고정(백엔드 기본값)이므로 셀렉트에서 선택 불가.

### SnapEditor 페이지

`UDashboardNavbar title="스냅 에디터"`. `#right`: [+ 영역] [+ 동작]
`UDropdownMenu` + `SaveActions`.

**구조**: `UTable` + expandable rows. 3패널 레이아웃 폐기.

| Column | 내용 |
|---|---|
| `expand` | chevron 버튼 → `row.toggleExpanded()` |
| `name` | 영역 이름 (텍스트 표시, 편집은 확장 시) |
| `kind` | UBadge (`area` / `action`) |
| `preview` | area만: 16:9 미니 rect 시각화 (색상 칩 또는 축소 도형) |
| `actions` | `UDropdownMenu` (복제 / 삭제) |

**`#expanded` 슬롯 콘텐츠**:
- `area`인 경우: `SnapProperties` 폼 (UInput name + USlider X/Y/W/H) +
  축소형 `SnapCanvas` (드래그/리사이즈로 비율 직관 편집).
- `action`인 경우: `SnapProperties` 폼 (UInput name + USelect
  WindowAction).

**삭제 대상**:
- 프리셋 셀렉트 (`active_preset`은 `'full'`로 고정, UI 숨김, 스키마 유지).
- Import/Export 버튼.
- Sector Mapping 탭 (Throw로 이동).

**`expanded` prop**: `v-model:expanded="expandedRows"` (string[] id 기반).
한 번에 여러 행 확장 허용 (기본 동작).

### Display 페이지

`UDashboardNavbar title="디스플레이"`.

| 섹션 | 필드 | 컴포넌트 |
|---|---|---|
| 커서 표시기 | `overlay.cursor.indicator` (USwitch), `overlay.cursor.color` (UColorPicker), `overlay.cursor.radius` (USlider 5~50), `overlay.cursor.opacity` (USlider 0.1~1) | enabled일 때만 하위 노출 |
| Snap Preview | `overlay.snap_preview.enabled` (USwitch) + `overlay.snap_preview.colors` (`ColorLockField`) | enabled일 때만 하위 노출 |

> `reticle_style` 섹션 제거 — 데드 필드(그리는 코드 없음)이므로 스키마와
> UI에서 모두 제거. Display 페이지는 위 2개 섹션만.

**Snap Preview 색상 UX (`ColorLockField` 컴포넌트)**:
```
Throw 색상       [UColorPicker] ┐
                              [🔒/🔓 lock 토글 버튼]
Long Throw 색상  [UColorPicker] ┘
```
- **잠금 상태 (기본, `i-lucide-lock`)**: 두 색상 모두 편집 가능. 어느
  한쪽을 변경하면 반대쪽도 **즉시 같은 값으로 동기화**.
  - throw_color 변경 → long_throw_color = throw_color
  - long_throw_color 변경 → throw_color = long_throw_color
  - 두 값은 항상 같게 유지됨 (사용자가 어느 쪽을 만지든).
  - 비활성화 없음 — 둘 다 활성 ColorPicker.
- **열림 상태 (`i-lucide-lock-open`)**: 두 색상 개별 편집. 한쪽 변경이
  반대쪽에 영향 없음.
- 잠금 상태는 UI 로컬 상태 (백엔드 필드 아님). 백엔드는 항상 두 색상을
  별도 저장. 잠금 상태에서 어느 쪽을 편집하든 `store.draft`의 두 필드
  모두 같은 값으로 기록됨.
- **초기 진입 동작**: 설정 로드 시 두 색상이 이미 같으면 잠금 상태로,
  다르면 열림 상태로 자동 판별.
- **기본값** (신규 설치): throw_color = long_throw_color = `#3B82F6`
  (잠금 상태에서 시작). 사용자가 열어서 분리 설정할 때 long_throw
  기본 제안값 `#A855F7` (보라).

**섹터 수 설정 제거**: `overlay.sector_count` 백엔드 8 고정. UI에서
선택 불가.

### About 페이지

`UDashboardNavbar title="정보"`. 내용은 현행 유지 (앱 정보 카드, 업데이트
섹션). 컨테이너만 UDashboardPanel로 마이그레이션.

## 백엔드 변경

### `OverlayConfig` 스키마 (`src-tauri/src/domain/model.rs`)

**제거**:
- `sector_count: u8` → 8 상수 고정.
- `sector_highlight_color: String`.
- `reticle_style: String` → **데드 필드** (그리는 코드 없음, UI에만 존재).
  win32_overlay draw_scene은 cursor 원 + snap_preview 사각형만 그림.

**계층화 원칙**: 강결합 필드(항상 함께 의미를 갖는 3개 이상)는 객체로 묶는다.
단일 필드나 독립적 필드는 평면 유지. legacy 호환성은 무시 (깔끔한 제거).

**재구성**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorConfig {
    pub indicator: bool,
    pub radius: u32,
    pub color: String,
    pub opacity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewColors {
    pub throw_color: String,
    pub long_throw_color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPreviewConfig {
    pub enabled: bool,
    pub colors: PreviewColors,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub cursor: CursorConfig,            // 객체화 (4개 강결합 필드)
    pub snap_preview: SnapPreviewConfig, // 객체화 (enabled + colors 강결합)
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            cursor: CursorConfig {
                indicator: true,
                radius: 18,
                color: "#E53935".to_string(),
                opacity: 0.5,
            },
            snap_preview: SnapPreviewConfig {
                enabled: true,
                colors: PreviewColors {
                    throw_color: "#3B82F6".to_string(),
                    long_throw_color: "#3B82F6".to_string(), // 잠금 상태 기본 — 같은 색
                },
            },
        }
    }
}
```

**ThrowConfig 계층화** (long_throw 3개 필드 묶기):
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LongThrowConfig {
    pub enabled: bool,
    pub distance: u32,
    pub mapping: SectorMap,
}

pub struct ThrowConfig {
    pub trigger_modifiers: Vec<String>,
    pub mapping: SectorMap,            // 기본 throw 매핑 (평면 — 단일 필드)
    pub long_throw: LongThrowConfig,   // 객체화 (3개 강결합 필드)
}
```

> `GeneralConfig`는 평면 유지 — `launch_at_login`/`start_minimized`는
> 독립적(로그인 시작해도 최소화 아닐 수 있음), `show_in_tray`/`language`/
> `snap_margin`도 단일 필드. 강결합 그룹 없음.

### `win32_overlay.rs` `draw_scene` 분기

`snap_preview` 사각형 색상 선택 로직:
```rust
// 기존: active_sector.is_some() → sector_highlight_color, else cursor_color
// 신규: throw 거리 기반 색상 분기
let color_hex = if state.is_long_throw {
    &cfg.snap_preview.colors.long_throw_color
} else {
    &cfg.snap_preview.colors.throw_color
};
```

**cursor 컬러 참조도 업데이트**: `cfg.cursor_color` → `cfg.cursor.color`,
`cfg.cursor_radius` → `cfg.cursor.radius` 등. snap_service/keyboard_service
호출부 모두 새 경로 사용.

`OverlayDrawState`에 `is_long_throw: bool` 필드 추가 또는 snap_service가
선택된 색상을 overlay에 직접 전달. long throw 판단은 이미
`snap_service.rs:222-225`에서 수행되므로, 해당 시점에
`show_snap_preview` 호출로 색상 정보를 넘김.

### `KeyboardConfig` 변경 (`domain/model.rs`)

```rust
// 기존
pub struct KeyboardConfig {
    pub enabled: bool,
    pub cycle_timeout_ms: u64,  // 제거
}
// 신규
pub struct KeyboardConfig {
    pub enabled: bool,
}
```

`keyboard.enabled`는 앱 전체 스냅 활성화 토글. **사이드바 footer**에
배치 (아래 "사이드바 footer 구성" 참고). `KeyboardConfig` 구조체 자체는
유지하되 페이지만 제거.

### `commands.rs` / `toml_config.rs` / 테스트

- 직렬화/역직렬화 대응.
- 기존 테스트(`toml_config.rs`)의 `sector_highlight_color`,
  `sector_count`, `cycle_timeout_ms` 참조 제거.
- **하위 호환성/마이그레이션 없음**: 기존 사용자 설정 파일에 구 필드가
  남아있으면 serde 에러 또는 무시. 신규 기본값으로 덮어쓰는 것을 전제로
  진행. `#[serde(default)]` 조차 붙이지 않고 필드를 깔끔하게 제거.

## 프론트엔드 스키마 변경 (`src/entities/config.ts`)

```ts
// 제거: overlay.sector_count, overlay.sector_highlight_color, overlay.reticle_style
// 제거: keyboard.cycle_timeout_ms
// 계층화: cursor_*, snap_preview + preview_colors → 객체

export const cursorConfigSchema = z.object({
  indicator: z.boolean(),
  radius: z.number().int(),
  color: z.string(),
  opacity: z.number(),
})
export type CursorConfig = z.infer<typeof cursorConfigSchema>

export const previewColorsSchema = z.object({
  throw_color: z.string(),
  long_throw_color: z.string(),
})
export type PreviewColors = z.infer<typeof previewColorsSchema>

export const snapPreviewConfigSchema = z.object({
  enabled: z.boolean(),
  colors: previewColorsSchema,
})
export type SnapPreviewConfig = z.infer<typeof snapPreviewConfigSchema>

export const overlayConfigSchema = z.object({
  cursor: cursorConfigSchema,
  snap_preview: snapPreviewConfigSchema,
})
export type OverlayConfig = z.infer<typeof overlayConfigSchema>

export const longThrowConfigSchema = z.object({
  enabled: z.boolean(),
  distance: z.number().int(),
  mapping: sectorMapSchema,
})
export type LongThrowConfig = z.infer<typeof longThrowConfigSchema>

export const throwConfigSchema = z.object({
  trigger_modifiers: z.array(z.string()),
  mapping: sectorMapSchema,
  long_throw: longThrowConfigSchema,
})
export type ThrowConfig = z.infer<typeof throwConfigSchema>

export const keyboardConfigSchema = z.object({
  enabled: z.boolean(),
})
```

## i18n 변경 (`src/i18n/`)

- `index.ts`: 기본 `locale` `'ko'`로.
- `locales/{en,ko}.ts`:
  - `nav.keyboard` 제거.
- `nav.pause` 제거 (활성화 토글로 대체).
- `nav.enableSnap`, `nav.enableSnapDesc` 추가 (footer 토글용).
  - `keyboard.*` 키 제거 (또는 enabled만 남기고 nav에서 숨김).
  - `display.sectorCount`, `display.sectorHighlightColor`, `display.sectors`
    제거.
  - `display.previewColors`, `display.previewColorsUnified`,
    `display.previewColorsLocked`, `display.previewColorsUnlocked` 추가.
  - `snapEditor.preset*`, `snapEditor.import`, `snapEditor.export`,
    `snapEditor.tabs.*` 제거.
  - `throw.captureHotkey`, `throw.capturing`, `throw.captureHint` 추가.
  - `snapEditor.expand`, `snapEditor.collapse`, `snapEditor.duplicate`
    추가.

## 검증 계획

- **수동**: 각 페이지 렌더링, 언어 전환 시 즉시 UI 반영, 핫키 캡처 후
  저장 → 백엔드 반영, Snap Areas 확장/편집/삭제, Snap Preview 색상 잠금/
  열림 동작, cargo test 통과.
- **자동**:
  - `cargo test` (Rust 스키마/서비스 테스트 업데이트).
  - `pnpm build` (프론트 타입 체크).
  - SnapEditor expandable rows 동작 단위 테스트는 본 스펙 범위 밖
    (수동 검증).

## 리스크

1. **`UDashboardGroup`/`UDashboardPanel` Vue+Vite 호환**: Nuxt 모듈이
   아닌 순수 Vue 컴포넌트로 제공되는지 확인 필요 (`@nuxt/ui/vue-plugin`
   경유). 구현 첫 단계에서 검증.
2. **`trigger_modifiers` 포맷 호환**: 백엔드가 대소문자 구분 (`"Win"`,
   `"Alt"` 등). `UHotkeyInput` emit 포맷을 정확히 맞출 것.
3. **TanStack Table ColumnDef**: Vue용 `h()` 함수 사용 패턴 숙지 필요.
4. **기존 설정 파일 깨짐 (수용)**: 구 필드 제거로 기존 사용자의 설정
   파일이 serde 오류를 일으킬 수 있음. **마이그레이션/하위 호환은 본
   스펙에서 명시적으로 무시** — 사용자는 설정 파일 삭제 후 재생성으로
   대응. 개발 단계 앱이므로 수용 가능.

## 추후 작업 (본 스펙 범위 밖)

- `trigger_modifiers` 외 일반 키(예: 방향키)까지 캡처하여 핫키 스킴
  확장.
- Snap Areas 드래그 앤 드롭 정렬 (UTable `useSortable`).
- 설정 내보내기/가져오기 (다른 형태로 재도입 필요 시).
