<script setup lang="ts">
/**
 * Snap 타겟 속성 폼 — SnapEditor 확장 행에서 사용.
 * 헤더/카드/삭제 버튼 없이 단순 폼만 제공 (삭제는 테이블 행 드롭다운에서).
 */
import { useI18n } from 'vue-i18n'
import type { SnapTarget, WindowAction } from '@/entities/config'

const props = defineProps<{
  target: SnapTarget | null
}>()

const emit = defineEmits<{
  update: [patch: Partial<SnapTarget>]
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

function updateArea(field: 'x_ratio' | 'y_ratio' | 'w_ratio' | 'h_ratio', value: number) {
  if (props.target?.kind === 'area') {
    emit('update', { [field]: value } as Partial<SnapTarget>)
  }
}
</script>

<template>
  <div v-if="target" class="grid gap-4 sm:grid-cols-2">
    <UFormField :label="t('snapEditor.name')" class="sm:col-span-2">
      <UInput
        :model-value="target.name"
        class="w-full"
        @update:model-value="emit('update', { name: $event as string } as Partial<SnapTarget>)"
      />
    </UFormField>

    <template v-if="target.kind === 'area'">
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
      <UFormField :label="t('snapEditor.width')">
        <USlider :model-value="target.w_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('w_ratio', $event)" />
        <template #hint>{{ (target.w_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
      <UFormField :label="t('snapEditor.height')">
        <USlider :model-value="target.h_ratio" :min="0.05" :max="1" :step="0.01" class="w-full"
          @update:model-value="updateArea('h_ratio', $event)" />
        <template #hint>{{ (target.h_ratio * 100).toFixed(0) }}%</template>
      </UFormField>
    </template>

    <UFormField v-else :label="t('snapEditor.action')" class="sm:col-span-2">
      <USelect
        :model-value="target.action"
        :items="actionOptions"
        value-key="value"
        class="w-full"
        @update:model-value="emit('update', { action: $event as WindowAction } as Partial<SnapTarget>)"
      />
    </UFormField>
  </div>
</template>
