<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: string[]
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()
const { t } = useI18n()

const capturing = ref(false)

// modifier 키 매핑 (e.code → 백엔드 토큰)
const CODE_TO_TOKEN: Record<string, string> = {
  ControlLeft: 'Ctrl',
  ControlRight: 'Ctrl',
  ShiftLeft: 'Shift',
  ShiftRight: 'Shift',
  AltLeft: 'Alt',
  AltRight: 'Alt',
  MetaLeft: 'Win',
  MetaRight: 'Win',
}

const displayBadges = computed(() =>
  props.modelValue.length > 0 ? props.modelValue : [t('throw.noHotkey')],
)

function toggleCapture() {
  capturing.value = !capturing.value
}

function onKeydown(e: KeyboardEvent) {
  if (!capturing.value) return
  e.preventDefault()
  e.stopPropagation()

  if (e.key === 'Escape') {
    capturing.value = false
    return
  }

  // 눌린 modifier들을 수집
  const mods = new Set<string>()
  // e.code 기반으로 모든 눌린 modifier 감지는 불가 (단일 이벤트만).
  // 따라서 현재 이벤트의 modifier 플래그 사용.
  if (e.ctrlKey) mods.add('Ctrl')
  if (e.shiftKey) mods.add('Shift')
  if (e.altKey) mods.add('Alt')
  if (e.metaKey) mods.add('Win')

  // modifier 키 자체를 누른 경우에도 해당 토큰 추가
  const token = CODE_TO_TOKEN[e.code]
  if (token) mods.add(token)

  // 빈 조합은 거부 (백엔드 check_throw_modifiers가 빈 조합 거부)
  if (mods.size === 0) return

  // 정해진 순서로 정렬 (Win, Ctrl, Alt, Shift)
  const order = ['Win', 'Ctrl', 'Alt', 'Shift']
  const sorted = order.filter((m) => mods.has(m))

  emit('update:modelValue', sorted)
  capturing.value = false
}

function onBlur() {
  capturing.value = false
}

function clearHotkey() {
  emit('update:modelValue', [])
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center gap-2">
      <UButton
        :color="capturing ? 'primary' : 'neutral'"
        :variant="capturing ? 'outline' : 'outline'"
        block
        :class="capturing ? 'ring-2 ring-primary/50' : ''"
        @click="toggleCapture"
        @keydown="onKeydown"
        @blur="onBlur"
      >
        <UIcon
          v-if="capturing"
          name="i-lucide-keyboard"
          class="size-4 animate-pulse"
        />
        <UBadge
          v-for="mod in displayBadges"
          :key="mod"
          :label="mod"
          color="neutral"
          variant="subtle"
          size="sm"
        />
        <span v-if="capturing" class="ml-auto text-xs text-muted">
          {{ t('throw.capturing') }}
        </span>
      </UButton>
      <UButton
        v-if="modelValue.length > 0"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="sm"
        @click="clearHotkey"
      />
    </div>
    <p class="text-xs text-muted">
      {{ capturing ? t('throw.captureHint') : t('throw.captureHotkeyDesc') }}
    </p>
  </div>
</template>
