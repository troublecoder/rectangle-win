/**
 * Config Zod 스키마 — Rust domain/model.rs 와 1:1 대칭.
 *
 * 백엔드에서 invoke() 응답으로 오는 JSON을 런타임 검증한다.
 * serde tag="kind" discriminated union → Zod discriminatedUnion('kind') 로 매핑.
 */
import { z } from 'zod'

// ─── WindowAction ───────────────────────────────────────────────────
export const windowActionSchema = z.enum([
  'Maximize',
  'Minimize',
  'Restore',
  'Center',
  'AlmostMaximize',
  'MaximizeHeight',
  'NextDisplay',
  'PreviousDisplay',
])
export type WindowAction = z.infer<typeof windowActionSchema>

// ─── SnapTarget (serde tag="kind") ──────────────────────────────────
export const snapTargetAreaSchema = z.object({
  kind: z.literal('area'),
  id: z.string(),
  name: z.string(),
  x_ratio: z.number(),
  y_ratio: z.number(),
  w_ratio: z.number(),
  h_ratio: z.number(),
})

export const snapTargetActionSchema = z.object({
  kind: z.literal('action'),
  id: z.string(),
  name: z.string(),
  action: windowActionSchema,
})

export const snapTargetSchema = z.discriminatedUnion('kind', [
  snapTargetAreaSchema,
  snapTargetActionSchema,
])
export type SnapTarget = z.infer<typeof snapTargetSchema>
export type SnapTargetArea = z.infer<typeof snapTargetAreaSchema>
export type SnapTargetAction = z.infer<typeof snapTargetActionSchema>

// ─── SectorMap (HashMap<u8, String> → Record) ───────────────────────
export const sectorMapSchema = z.record(z.string(), z.string())
export type SectorMap = z.infer<typeof sectorMapSchema>

// ─── Config 하위 스키마들 ───────────────────────────────────────────
export const generalConfigSchema = z.object({
  launch_at_login: z.boolean(),
  start_minimized: z.boolean(),
  show_in_tray: z.boolean(),
  language: z.string(),
  override_win_snap: z.boolean(),
  snap_margin: z.number(),
})
export type GeneralConfig = z.infer<typeof generalConfigSchema>

export const snapConfigSchema = z.object({
  active_preset: z.string(),
  areas: z.array(snapTargetSchema),
})
export type SnapConfig = z.infer<typeof snapConfigSchema>

export const throwConfigSchema = z.object({
  trigger_modifiers: z.array(z.string()),
  long_throw_enabled: z.boolean(),
  long_throw_distance: z.number().int(),
  mapping: sectorMapSchema,
  long_throw_mapping: sectorMapSchema,
})
export type ThrowConfig = z.infer<typeof throwConfigSchema>

export const keyboardConfigSchema = z.object({
  enabled: z.boolean(),
  cycle_timeout_ms: z.number().int(),
})
export type KeyboardConfig = z.infer<typeof keyboardConfigSchema>

export const overlayConfigSchema = z.object({
  reticle_style: z.string(),
  cursor_indicator: z.boolean(),
  cursor_radius: z.number().int(),
  cursor_color: z.string(),
  cursor_opacity: z.number(),
  sector_highlight_color: z.string(),
  sector_count: z.number().int(),
  snap_preview: z.boolean(),
})
export type OverlayConfig = z.infer<typeof overlayConfigSchema>

export const updateConfigSchema = z.object({
  enabled: z.boolean(),
  channel: z.string(),
  check_on_startup: z.boolean(),
})
export type UpdateConfig = z.infer<typeof updateConfigSchema>

// ─── Config (최상위) ────────────────────────────────────────────────
export const configSchema = z.object({
  general: generalConfigSchema,
  snap: snapConfigSchema,
  throw: throwConfigSchema,
  keyboard: keyboardConfigSchema,
  overlay: overlayConfigSchema,
  update: updateConfigSchema,
})
export type Config = z.infer<typeof configSchema>

// ─── MonitorInfo (commands.rs MonitorInfo DTO) ──────────────────────
export const monitorInfoSchema = z.object({
  x: z.number().int(),
  y: z.number().int(),
  width: z.number().int(),
  height: z.number().int(),
})
export type MonitorInfo = z.infer<typeof monitorInfoSchema>

// ─── SnapPreset 이름 ────────────────────────────────────────────────
export const snapPresetSchema = z.enum([
  'minimal',
  'standard',
  'extended',
  'full',
  'portrait',
])
export type SnapPresetName = z.infer<typeof snapPresetSchema>
