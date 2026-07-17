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
      const config = await api.getConfig()
      saved.value = config
      draft.value = JSON.parse(JSON.stringify(config))
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
      // isBrowserOnly 체크 제거 — 항상 invoke 시도, 실패 시 catch.
      // Tauri 웹뷰에서 __TAURI_INTERNALS__ 체크가 신뢰할 수 없는 문제 해결.
      await api.saveConfig(draft.value)
      saved.value = JSON.parse(JSON.stringify(draft.value))
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      saving.value = false
    }
  }

  function reset() {
    if (saved.value) {
      draft.value = JSON.parse(JSON.stringify(saved.value))
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
