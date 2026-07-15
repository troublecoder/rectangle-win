/**
 * Config Pinia store — 백엔드 설정의 단일 진실 공급원.
 *
 * - load(): 초기 로드
 * - update(path, value): 드래프트 수정 (dirty 상태)
 * - save(): 드래프트를 백엔드에 저장
 * - reset(): 드래프트를 마지막 저장본으로 되돌림
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as api from './api'
import type { Config } from '@/entities/config'
import { defaultConfig } from '@/entities/default-config'

/** Tauri IPC가 없는 순수 브라우저 환경 감지 */
const isBrowserOnly = typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window)

export const useConfigStore = defineStore('config', () => {
  // 백엔드에 저장된 최신 설정
  const saved = ref<Config | null>(null)
  // UI에서 편집 중인 드래프트
  const draft = ref<Config | null>(null)
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<string | null>(null)

  const isDirty = computed(() => {
    if (!saved.value || !draft.value) return false
    return JSON.stringify(saved.value) !== JSON.stringify(draft.value)
  })

  async function load() {
    loading.value = true
    error.value = null
    try {
      if (isBrowserOnly) {
        // 개발 환경: Tauri 없이 mock 데이터 사용
        saved.value = structuredClone(defaultConfig)
        draft.value = structuredClone(defaultConfig)
      } else {
        const config = await api.getConfig()
        saved.value = config
        draft.value = structuredClone(config)
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function save() {
    if (!draft.value) return
    saving.value = true
    error.value = null
    try {
      if (!isBrowserOnly) {
        await api.saveConfig(draft.value)
      }
      saved.value = structuredClone(draft.value)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      saving.value = false
    }
  }

  function reset() {
    if (saved.value) {
      draft.value = structuredClone(saved.value)
    }
  }

  return {
    saved,
    draft,
    loading,
    saving,
    error,
    isDirty,
    load,
    save,
    reset,
  }
})
