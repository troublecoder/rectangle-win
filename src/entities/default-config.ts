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
    areas: [], // 사용자 추가 영역만 (빌트인은 코드에서 제공)
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
