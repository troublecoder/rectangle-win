<script setup lang="ts">
/**
 * 섹터 매핑 탭 — 섹터 인덱스(0~N)에 snap 타겟 id를 연결.
 * config.throw.mapping 및 long_throw_mapping을 편집.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SnapTarget, SectorMap } from '@/entities/config'

const props = defineProps<{
  sectorCount: number
  targets: SnapTarget[]
  mapping: SectorMap
  longThrowMapping: SectorMap
}>()

const emit = defineEmits<{
  'update:mapping': [value: SectorMap]
  'update:longThrowMapping': [value: SectorMap]
}>()

const { t } = useI18n()

const sectors = computed(() =>
  Array.from({ length: props.sectorCount }, (_, i) => i),
)

const targetOptions = computed(() =>
  props.targets.map(t => ({ label: t.name, value: t.id })),
)

const sectorLabels: Record<number, string> = {
  0: '→', 1: '↘', 2: '↓', 3: '↙',
  4: '←', 5: '↖', 6: '↑', 7: '↗',
  8: '→→', 9: '↘↘', 10: '↓↓', 11: '↙↙',
}

function getTarget(map: SectorMap, sector: number): string {
  return map[String(sector)] ?? ''
}

function setTarget(map: SectorMap, sector: number, targetId: string) {
  const next = { ...map }
  if (targetId) {
    next[String(sector)] = targetId
  } else {
    delete next[String(sector)]
  }
  return next
}

function updateMapping(sector: number, targetId: string) {
  emit('update:mapping', setTarget(props.mapping, sector, targetId))
}

function updateLongThrow(sector: number, targetId: string) {
  emit('update:longThrowMapping', setTarget(props.longThrowMapping, sector, targetId))
}
</script>

<template>
  <div class="grid grid-cols-2 gap-6">
    <!-- 기본 매핑 -->
    <div>
      <h4 class="mb-3 text-sm font-medium">{{ t('throw.title') }}</h4>
      <div class="space-y-2">
        <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
          <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
          <USelect
            :model-value="getTarget(mapping, sector)"
            :items="targetOptions"
            value-key="value"
            size="sm"
            class="flex-1"
            @update:model-value="updateMapping(sector, $event as string)"
          />
        </div>
      </div>
    </div>

    <!-- Long Throw 매핑 -->
    <div>
      <h4 class="mb-3 text-sm font-medium">{{ t('throw.longThrow') }}</h4>
      <div class="space-y-2">
        <div v-for="sector in sectors" :key="sector" class="flex items-center gap-2">
          <span class="w-8 text-center text-lg">{{ sectorLabels[sector] ?? sector }}</span>
          <USelect
            :model-value="getTarget(longThrowMapping, sector)"
            :items="targetOptions"
            value-key="value"
            size="sm"
            class="flex-1"
            @update:model-value="updateLongThrow(sector, $event as string)"
          />
        </div>
      </div>
    </div>
  </div>
</template>
