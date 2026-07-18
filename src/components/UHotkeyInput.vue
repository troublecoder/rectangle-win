<script setup lang="ts">
/**
 * 핫키 캡처 입력 — modifier 키 조합을 잡아 string[]로 내보낸다.
 *
 * VS Code 식 동작:
 *  - 캡처 시작 후 modifier(Win/Ctrl/Alt/Shift)를 누르는 동안 조합이 화면에 누적.
 *  - modifier가 아닌 일반 키(예: 방향키, 문자)를 누르면 그 시점의 modifier 조합을 확정.
 *  - ESC: 캡처 취소.
 *  - blur: 캡처 취소.
 *  - 빈 조합은 확정하지 않는다 (백엔드 check_throw_modifiers가 거부).
 */
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: string[]
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()
const { t } = useI18n()

const capturing = ref(false)
// 캡처 도중 누적 중인 modifier 집합
const pendingMods = ref<Set<string>>(new Set())

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

const MODIFIER_TOKENS = new Set(['Win', 'Ctrl', 'Alt', 'Shift'])

const displayBadges = computed(() =>
  props.modelValue.length > 0 ? props.modelValue : [t('throw.noHotkey')],
)

const pendingBadges = computed(() =>
  Array.from(pendingMods.value).length > 0
    ? Array.from(pendingMods.value)
    : [t('throw.captureHint')],
)

function toggleCapture() {
  if (capturing.value) {
    // 이미 캡처 중이면 취소
    cancelCapture()
    return
  }
  pendingMods.value = new Set()
  capturing.value = true
}

function commitCapture() {
  if (pendingMods.value.size === 0) {
    cancelCapture()
    return
  }
  // 정해진 순서로 정렬 (Win, Ctrl, Alt, Shift)
  const order = ['Win', 'Ctrl', 'Alt', 'Shift']
  const sorted = order.filter((m) => pendingMods.value.has(m))
  emit('update:modelValue', sorted)
  capturing.value = false
  pendingMods.value = new Set()
}

function cancelCapture() {
  capturing.value = false
  pendingMods.value = new Set()
}

function onKeydown(e: KeyboardEvent) {
  if (!capturing.value) return
  e.preventDefault()
  e.stopPropagation()

  if (e.key === 'Escape') {
    cancelCapture()
    return
  }

  const token = CODE_TO_TOKEN[e.code]
  if (token) {
    // modifier 키 눌림 — 누적 (e.repeat 무시)
    if (!e.repeat) pendingMods.value.add(token)
    // modifier만 누르는 동안엔 확정하지 않고 대기
    return
  }

  // modifier가 아닌 키 — 현재 누적된 modifier 조합 확정.
  // modifier가 하나도 안 눌린 상태에서 일반 키만 온 경우엔 무시 (빈 조합 거부).
  if (pendingMods.value.size > 0) {
    commitCapture()
  }
}

function clearHotkey() {
  emit('update:modelValue', [])
}

function onBlur() {
  cancelCapture()
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center gap-2">
      <UButton
        :color="capturing ? 'primary' : 'neutral'"
        variant="outline"
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
        <template v-if="capturing">
          <UBadge
            v-for="mod in pendingBadges"
            :key="mod"
            :label="mod"
            color="primary"
            variant="subtle"
            size="sm"
          />
        </template>
        <template v-else>
          <UBadge
            v-for="mod in displayBadges"
            :key="mod"
            :label="mod"
            color="neutral"
            variant="subtle"
            size="sm"
          />
        </template>
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
      {{ capturing ? t('throw.captureHintMulti') : t('throw.captureHotkeyDesc') }}
    </p>
  </div>
</template>
