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
      { kind: 'action', id: 'restore', name: 'Restore', action: 'Restore' },
    ],
  },
  throw: {
    trigger_modifiers: ['Win', 'Alt'],
    mapping: {
      '0': 'two-thirds-right',
      '1': 'sixth-br',
      '2': 'restore',
      '3': 'sixth-bl',
      '4': 'third-left',
      '5': 'sixth-tl',
      '6': 'maximize',
      '7': 'sixth-tr',
    },
    long_throw: {
      enabled: true,
      distance: 400,
      mapping: {},
    },
  },
  keyboard: {
    enabled: true,
  },
  overlay: {
    cursor: {
      indicator: true,
      radius: 18,
      color: '#E53935',
      opacity: 0.5,
    },
    snap_preview: {
      enabled: true,
      colors: {
        throw_color: '#3B82F6',
        long_throw_color: '#3B82F6',
      },
    },
  },
  update: {
    enabled: true,
    channel: 'stable',
    check_on_startup: true,
  },
}
