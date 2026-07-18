<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SnapTarget, SectorMap } from '@/entities/config'

const props = defineProps<{
  targets: SnapTarget[]
  mapping: SectorMap
  longThrowMapping: SectorMap
}>()

const emit = defineEmits<{
  'update:mapping': [value: SectorMap]
  'update:longThrowMapping': [value: SectorMap]
}>()

const { t } = useI18n()

// 8 섹터 고정 (백엔드 기본값)
const SECTOR_COUNT = 8
const sectors = computed(() => Array.from({ length: SECTOR_COUNT }, (_, i) => i))

const targetOptions = computed(() =>
  props.targets.map((tgt) => ({ label: tgt.name, value: tgt.id })),
)

const sectorLabels: Record<number, string> = {
  0: '→', 1: '↘', 2: '↓', 3: '↙',
  4: '←', 5: '↖', 6: '↑', 7: '↗',
}

function getTarget(map: SectorMap, sector: number): string {
  return map[String(sector)] ?? ''
}

function setTarget(map: SectorMap, sector: number, targetId: string): SectorMap {
  const next = { ...map }
  if (targetId) next[String(sector)] = targetId
  else delete next[String(sector)]
  return next
}
</script>

<template>
  <UCard variant="subtle">
    <div class="grid grid-cols-2 gap-6">
      <!-- 기본 throw 매핑 -->
      <div>
        <h4 class="mb-3 text-sm font-medium text-muted">{{ t('throw.title') }}</h4>
        <div class="space-y-2">
          <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
            <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
            <USelect
              :model-value="getTarget(mapping, sector)"
              :items="targetOptions"
              value-key="value"
              size="sm"
              class="flex-1"
              @update:model-value="emit('update:mapping', setTarget(mapping, sector, $event as string))"
            />
          </div>
        </div>
      </div>
      <!-- Long throw 매핑 -->
      <div>
        <h4 class="mb-3 text-sm font-medium text-muted">{{ t('throw.longThrow') }}</h4>
        <div class="space-y-2">
          <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
            <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
            <USelect
              :model-value="getTarget(longThrowMapping, sector)"
              :items="targetOptions"
              value-key="value"
              size="sm"
              class="flex-1"
              @update:model-value="emit('update:longThrowMapping', setTarget(longThrowMapping, sector, $event as string))"
            />
          </div>
        </div>
      </div>
    </div>
  </UCard>
</template>
