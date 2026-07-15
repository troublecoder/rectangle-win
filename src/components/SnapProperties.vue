<script setup lang="ts">
/**
 * Snap 타겟 속성 패널 — 선택된 타겟의 메타데이터와 ratio 값 편집.
 */
import { useI18n } from 'vue-i18n'
import type { SnapTarget, WindowAction } from '@/entities/config'

const props = defineProps<{
  target: SnapTarget | null
}>()

const emit = defineEmits<{
  update: [patch: Partial<SnapTarget>]
  delete: []
}>()

const { t } = useI18n()

const actionOptions: { label: string; value: WindowAction }[] = [
  { label: 'Maximize', value: 'Maximize' },
  { label: 'Minimize', value: 'Minimize' },
  { label: 'Restore', value: 'Restore' },
  { label: 'Center', value: 'Center' },
  { label: 'Almost Maximize', value: 'AlmostMaximize' },
  { label: 'Maximize Height', value: 'MaximizeHeight' },
  { label: 'Next Display', value: 'NextDisplay' },
  { label: 'Previous Display', value: 'PreviousDisplay' },
]

function updateName(name: string) {
  emit('update', { name } as Partial<SnapTarget>)
}

function updateArea(field: 'x_ratio' | 'y_ratio' | 'w_ratio' | 'h_ratio', value: number) {
  if (props.target?.kind === 'area') {
    emit('update', { [field]: value } as Partial<SnapTarget>)
  }
}

function updateAction(action: WindowAction) {
  if (props.target?.kind === 'action') {
    emit('update', { action } as Partial<SnapTarget>)
  }
}
</script>

<template>
  <div class="space-y-4">
    <div v-if="!target" class="py-8 text-center text-sm text-muted">
      <UIcon name="i-lucide-mouse-pointer-click" class="mb-2 size-6 opacity-50" />
      <p>{{ t('snapEditor.properties') }}</p>
    </div>

    <template v-else>
      <!-- 공통: 이름 -->
      <UFormField :label="t('snapEditor.name')">
        <UInput :model-value="target.name" @update:model-value="updateName($event as string)" />
      </UFormField>

      <USeparator />

      <!-- Area 타입 -->
      <template v-if="target.kind === 'area'">
        <UFormField :label="t('snapEditor.type')">
          <UBadge :label="t('snapEditor.area')" color="primary" variant="soft" />
        </UFormField>

        <UFormField label="X">
          <USlider :model-value="target.x_ratio" :min="0" :max="1" :step="0.01" class="w-full"
            @update:model-value="updateArea('x_ratio', $event)" />
          <template #hint>{{ (target.x_ratio * 100).toFixed(0) }}%</template>
        </UFormField>

        <UFormField label="Y">
          <USlider :model-value="target.y_ratio" :min="0" :max="1" :step="0.01" class="w-full"
            @update:model-value="updateArea('y_ratio', $event)" />
          <template #hint>{{ (target.y_ratio * 100).toFixed(0) }}%</template>
        </UFormField>

        <UFormField label="Width">
          <USlider :model-value="target.w_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
            @update:model-value="updateArea('w_ratio', $event)" />
          <template #hint>{{ (target.w_ratio * 100).toFixed(0) }}%</template>
        </UFormField>

        <UFormField label="Height">
          <USlider :model-value="target.h_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
            @update:model-value="updateArea('h_ratio', $event)" />
          <template #hint>{{ (target.h_ratio * 100).toFixed(0) }}%</template>
        </UFormField>
      </template>

      <!-- Action 타입 -->
      <template v-else>
        <UFormField :label="t('snapEditor.type')">
          <UBadge :label="t('snapEditor.action')" color="info" variant="soft" />
        </UFormField>

        <UFormField :label="t('snapEditor.action')">
          <USelect
            :model-value="target.action"
            :items="actionOptions"
            value-key="value"
            @update:model-value="updateAction($event as WindowAction)"
          />
        </UFormField>
      </template>

      <USeparator />

      <UButton
        :label="t('common.delete')"
        icon="i-lucide-trash-2"
        color="error"
        variant="ghost"
        block
        @click="emit('delete')"
      />
    </template>
  </div>
</template>
