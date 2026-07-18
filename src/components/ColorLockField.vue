<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { PreviewColors } from '@/entities/config'

const props = defineProps<{
  modelValue: PreviewColors
}>()
const emit = defineEmits<{ 'update:modelValue': [value: PreviewColors] }>()
const { t } = useI18n()

// 잠금 상태: 초기 두 색상이 같으면 잠금, 다르면 열림
const locked = ref(props.modelValue.throw_color === props.modelValue.long_throw_color)

// emit은 항상 새 객체 spread — Vue 반응성 안전
function patch(partial: Partial<PreviewColors>) {
  emit('update:modelValue', { ...props.modelValue, ...partial })
}

// throw 변경 — 잠금 시 long_throw 동기화
function updateThrow(value: string) {
  if (locked.value) {
    patch({ throw_color: value, long_throw_color: value })
  } else {
    patch({ throw_color: value })
  }
}

// long_throw 변경 — 잠금 시 throw 동기화 (양방향)
function updateLongThrow(value: string) {
  if (locked.value) {
    patch({ throw_color: value, long_throw_color: value })
  } else {
    patch({ long_throw_color: value })
  }
}

function toggleLock() {
  locked.value = !locked.value
  if (locked.value) {
    // 열림 → 잠금 전환 시 throw 기준으로 long_throw 동기화
    patch({ long_throw_color: props.modelValue.throw_color })
  }
}

// 외부 modelValue 변경(초기 로드/리셋/load 등) 시 잠금 상태 재판별.
// watch 안에서는 emit하지 않음 — 순환 참조 방지.
watch(
  () => props.modelValue,
  (val) => {
    locked.value = val.throw_color === val.long_throw_color
  },
)
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-end">
      <UButton
        :icon="locked ? 'i-lucide-lock' : 'i-lucide-lock-open'"
        :label="locked ? t('display.colorsLocked') : t('display.colorsUnlocked')"
        color="neutral"
        variant="ghost"
        size="xs"
        @click="toggleLock"
      />
    </div>
    <UFormField :label="t('display.throwColor')">
      <UColorPicker
        :model-value="modelValue.throw_color"
        @update:model-value="updateThrow($event as string)"
      />
    </UFormField>
    <UFormField :label="t('display.longThrowColor')">
      <UColorPicker
        :model-value="modelValue.long_throw_color"
        @update:model-value="updateLongThrow($event as string)"
      />
    </UFormField>
  </div>
</template>
