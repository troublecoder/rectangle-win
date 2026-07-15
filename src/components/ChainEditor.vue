<script setup lang="ts">
/**
 * 키보드 체인 편집 탭 — horizontal/vertical 체인의 snap 타겟 순서를 드래그로 편집.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SnapTarget, ChainConfig } from '@/entities/config'

const props = defineProps<{
  chains: ChainConfig
  targets: SnapTarget[]
}>()

const emit = defineEmits<{
  'update:chains': [value: ChainConfig]
}>()

const { t } = useI18n()

const targetNameById = computed(() => {
  const map: Record<string, string> = {}
  for (const target of props.targets) {
    map[target.id] = target.name
  }
  return map
})

const availableTargets = computed(() =>
  props.targets.map(t => ({ label: t.name, value: t.id })),
)

function removeFromChain(direction: 'horizontal' | 'vertical', index: number) {
  const list = [...props.chains[direction]]
  list.splice(index, 1)
  emit('update:chains', { ...props.chains, [direction]: list })
}

function addToChain(direction: 'horizontal' | 'vertical', targetId: string) {
  if (!targetId) return
  emit('update:chains', {
    ...props.chains,
    [direction]: [...props.chains[direction], targetId],
  })
}

function moveInChain(direction: 'horizontal' | 'vertical', index: number, delta: number) {
  const list = [...props.chains[direction]]
  const newIndex = index + delta
  if (newIndex < 0 || newIndex >= list.length) return
  ;[list[index], list[newIndex]] = [list[newIndex], list[index]]
  emit('update:chains', { ...props.chains, [direction]: list })
}
</script>

<template>
  <div class="grid grid-cols-2 gap-6">
    <!-- Horizontal Chain -->
    <div>
      <h4 class="mb-3 text-sm font-medium">{{ t('keyboard.horizontalChain') }}</h4>
      <div class="space-y-2">
        <div
          v-for="(id, index) in chains.horizontal"
          :key="`${id}-${index}`"
          class="flex items-center gap-2"
        >
          <div class="flex flex-col">
            <UButton
              icon="i-lucide-chevron-up" size="xs" color="neutral" variant="ghost"
              :disabled="index === 0"
              @click="moveInChain('horizontal', index, -1)"
            />
            <UButton
              icon="i-lucide-chevron-down" size="xs" color="neutral" variant="ghost"
              :disabled="index === chains.horizontal.length - 1"
              @click="moveInChain('horizontal', index, 1)"
            />
          </div>
          <span class="flex-1 rounded bg-elevated/50 px-2 py-1 text-sm">
            {{ targetNameById[id] ?? id }}
          </span>
          <UButton
            icon="i-lucide-x" size="xs" color="error" variant="ghost"
            @click="removeFromChain('horizontal', index)"
          />
        </div>
      </div>
      <USelectMenu
        :items="availableTargets"
        value-key="value"
        placeholder="+ Add"
        size="sm"
        class="mt-2 w-full"
        @update:model-value="addToChain('horizontal', $event as string)"
      />
    </div>

    <!-- Vertical Chain -->
    <div>
      <h4 class="mb-3 text-sm font-medium">{{ t('keyboard.verticalChain') }}</h4>
      <div class="space-y-2">
        <div
          v-for="(id, index) in chains.vertical"
          :key="`${id}-${index}`"
          class="flex items-center gap-2"
        >
          <div class="flex flex-col">
            <UButton
              icon="i-lucide-chevron-up" size="xs" color="neutral" variant="ghost"
              :disabled="index === 0"
              @click="moveInChain('vertical', index, -1)"
            />
            <UButton
              icon="i-lucide-chevron-down" size="xs" color="neutral" variant="ghost"
              :disabled="index === chains.vertical.length - 1"
              @click="moveInChain('vertical', index, 1)"
            />
          </div>
          <span class="flex-1 rounded bg-elevated/50 px-2 py-1 text-sm">
            {{ targetNameById[id] ?? id }}
          </span>
          <UButton
            icon="i-lucide-x" size="xs" color="error" variant="ghost"
            @click="removeFromChain('vertical', index)"
          />
        </div>
      </div>
      <USelectMenu
        :items="availableTargets"
        value-key="value"
        placeholder="+ Add"
        size="sm"
        class="mt-2 w-full"
        @update:model-value="addToChain('vertical', $event as string)"
      />
    </div>
  </div>
</template>
