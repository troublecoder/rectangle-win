/**
 * 기본 Config — Rust domain/model.rs 의 Default impl 과 대칭.
 * Tauri IPC가 없는 순수 브라우저 개발환경에서 fallback으로 사용.
 */
import type { Config } from './config'

export const defaultConfig: Config = {
  general: {
    launch_at_login: true,
    start_minimized: true,
    show_in_tray: true,
    language: 'ko',
    snap_margin: 0,
  },
  snap: {
    active_preset: 'full',
    areas: [
      { kind: 'area', id: 'left-half', name: 'Left Half', x_ratio: 0, y_ratio: 0, w_ratio: 0.5, h_ratio: 1 },
      { kind: 'area', id: 'right-half', name: 'Right Half', x_ratio: 0.5, y_ratio: 0, w_ratio: 0.5, h_ratio: 1 },
      { kind: 'area', id: 'top-half', name: 'Top Half', x_ratio: 0, y_ratio: 0, w_ratio: 1, h_ratio: 0.5 },
      { kind: 'area', id: 'bottom-half', name: 'Bottom Half', x_ratio: 0, y_ratio: 0.5, w_ratio: 1, h_ratio: 0.5 },
      { kind: 'area', id: 'center', name: 'Center', x_ratio: 0.25, y_ratio: 0.25, w_ratio: 0.5, h_ratio: 0.5 },
      { kind: 'area', id: 'third-left', name: 'Third Left', x_ratio: 0, y_ratio: 0, w_ratio: 1 / 3, h_ratio: 1 },
      { kind: 'area', id: 'third-center', name: 'Third Center', x_ratio: 1 / 3, y_ratio: 0, w_ratio: 1 / 3, h_ratio: 1 },
      { kind: 'area', id: 'third-right', name: 'Third Right', x_ratio: 2 / 3, y_ratio: 0, w_ratio: 1 / 3, h_ratio: 1 },
      { kind: 'action', id: 'maximize', name: 'Maximize', action: 'Maximize' },
      { kind: 'action', id: 'almost-maximize', name: 'Almost Maximize', action: 'AlmostMaximize' },
      { kind: 'action', id: 'maximize-height', name: 'Maximize Height', action: 'MaximizeHeight' },
    ],
  },
  throw: {
    trigger_modifiers: ['Win', 'Alt'],
    long_throw_enabled: true,
    long_throw_distance: 400,
    mapping: { '0': 'right-half', '2': 'bottom-half', '4': 'left-half', '6': 'top-half' },
    long_throw_mapping: { '0': 'third-right', '2': 'maximize', '4': 'third-left', '6': 'maximize-height' },
  },
  keyboard: {
    enabled: true,
    cycle_timeout_ms: 1500,
  },
  overlay: {
    reticle_style: 'pie',
    cursor_indicator: true,
    cursor_radius: 18,
    cursor_color: '#E53935',
    cursor_opacity: 0.5,
    sector_highlight_color: '#3B82F6',
    sector_count: 8,
    snap_preview: true,
  },
  update: {
    enabled: true,
    channel: 'stable',
    check_on_startup: true,
  },
}
