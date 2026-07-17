/**
 * Tauri IPC 래퍼 — invoke 호출 + Zod 파싱을 캡슐화.
 *
 * 모든 호출은 백엔드 commands.rs 의 6개 명령에 대응한다.
 * 런타임 검증 실패시 ZodError가 그대로 throw된다.
 */
import { invoke } from '@tauri-apps/api/core'
import {
  configSchema,
  monitorInfoSchema,
  type Config,
  type MonitorInfo,
  type SnapPresetName,
} from '@/entities/config'

/** CommandError (commands.rs) */
export interface CommandError {
  message: string
  code: string
}

export async function getConfig(): Promise<Config> {
  const raw = await invoke<unknown>('get_config')
  return configSchema.parse(raw)
}

export async function saveConfig(config: Config): Promise<void> {
  // Vue reactive 객체를 plain object로 변환 — invoke가 structuredClone을
  // 사용하기 때문에 reactive proxy는 복제할 수 없다.
  const plain = JSON.parse(JSON.stringify(config))
  await invoke('save_config', { config: plain })
}

export async function getConfigPath(): Promise<string> {
  return invoke<string>('get_config_path')
}

export async function applyPreset(presetName: SnapPresetName): Promise<Config> {
  const raw = await invoke<unknown>('apply_preset', { presetName })
  return configSchema.parse(raw)
}

export async function getMonitors(): Promise<MonitorInfo[]> {
  const raw = await invoke<unknown>('get_monitors')
  return monitorInfoSchema.array().parse(raw)
}

export async function testSnapToSector(
  sector: number,
  cursorX: number,
  cursorY: number,
): Promise<void> {
  await invoke('test_snap_to_sector', {
    sector,
    cursorX,
    cursorY,
  })
}
