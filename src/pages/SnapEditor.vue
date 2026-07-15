<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import * as api from '@/features/api'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'
import SnapCanvas from '@/components/SnapCanvas.vue'
import SnapProperties from '@/components/SnapProperties.vue'
import SectorMapping from '@/components/SectorMapping.vue'
import ChainEditor from '@/components/ChainEditor.vue'
import type { SnapTarget, SnapPresetName } from '@/entities/config'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const selectedId = ref<string | null>(null)
const activeTab = ref(0)

const presetItems = computed(() => [
  { label: t('snapEditor.presetMinimal'), value: 'minimal' },
  { label: t('snapEditor.presetStandard'), value: 'standard' },
  { label: t('snapEditor.presetExtended'), value: 'extended' },
  { label: t('snapEditor.presetFull'), value: 'full' },
  { label: t('snapEditor.presetPortrait'), value: 'portrait' },
])

const tabItems = computed(() => [
  { label: t('snapEditor.tabs.areas'), slot: 'areas', icon: 'i-lucide-layout-grid' },
  { label: t('snapEditor.tabs.sectorMapping'), slot: 'mapping', icon: 'i-lucide-pie-chart' },
  { label: t('snapEditor.tabs.chains'), slot: 'chains', icon: 'i-lucide-link' },
])

const areas = computed(() =>
  store.draft?.snap.areas.filter((a): a is Extract<SnapTarget, { kind: 'area' }> => a.kind === 'area') ?? [],
)

const selectedTarget = computed(() =>
  store.draft?.snap.areas.find(a => a.id === selectedId.value) ?? null,
)

function selectTarget(id: string) {
  selectedId.value = id
}

function updateTarget(id: string, patch: Partial<SnapTarget>) {
  if (!store.draft) return
  const idx = store.draft.snap.areas.findIndex(a => a.id === id)
  if (idx >= 0) {
    store.draft.snap.areas[idx] = { ...store.draft.snap.areas[idx], ...patch } as SnapTarget
  }
}

function deleteTarget(id: string) {
  if (!store.draft) return
  store.draft.snap.areas = store.draft.snap.areas.filter(a => a.id !== id)
  // 매핑에서도 제거
  for (const mapKey of ['mapping', 'long_throw_mapping'] as const) {
    const map = store.draft.throw[mapKey]
    for (const [sector, targetId] of Object.entries(map)) {
      if (targetId === id) delete map[sector]
    }
  }
  selectedId.value = null
}

function addTarget(kind: 'area' | 'action') {
  if (!store.draft) return
  const id = kind === 'area' ? `area-${Date.now()}` : `action-${Date.now()}`
  const name = kind === 'area' ? 'New Area' : 'New Action'
  const target: SnapTarget = kind === 'area'
    ? { kind: 'area', id, name, x_ratio: 0.1, y_ratio: 0.1, w_ratio: 0.3, h_ratio: 0.3 }
    : { kind: 'action', id, name, action: 'Maximize' }
  store.draft.snap.areas.push(target)
  selectedId.value = id
}

async function applyPreset(presetName: string) {
  if (!presetName) return
  try {
    const config = await api.applyPreset(presetName as SnapPresetName)
    if (store.saved) {
      store.saved = config
      // draft도 snap 부분만 갱신
      if (store.draft) {
        store.draft.snap = config.snap
      }
    }
  } catch {
    // 브라우저 환경에서는 무시
  }
}
</script>

<template>
  <div class="max-w-5xl space-y-6">
    <div class="flex items-start justify-between">
      <PageHeader :title="t('snapEditor.title')" :description="t('snapEditor.description')" />
      <!-- Preset + Import/Export -->
      <div class="flex items-center gap-2">
        <USelect
          :model-value="store.draft?.snap.active_preset"
          :items="presetItems"
          value-key="value"
          size="sm"
          :placeholder="t('snapEditor.preset')"
          @update:model-value="applyPreset($event as string)"
        />
        <UButton icon="i-lucide-download" size="sm" color="neutral" variant="ghost" :label="t('snapEditor.import')" />
        <UButton icon="i-lucide-upload" size="sm" color="neutral" variant="ghost" :label="t('snapEditor.export')" />
      </div>
    </div>

    <template v-if="store.draft">
      <UTabs v-model="activeTab" :items="tabItems" class="w-full">

        <!-- Tab 1: Snap Areas (3패널) -->
        <template #areas>
          <div class="grid grid-cols-[200px_1fr_240px] gap-4 py-4">
            <!-- 좌측: 타겟 목록 -->
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <h4 class="text-sm font-medium">{{ t('snapEditor.targets') }}</h4>
                <UDropdownMenu :items="[
                  { label: t('snapEditor.area'), icon: 'i-lucide-square', onSelect: () => addTarget('area') },
                  { label: t('snapEditor.action'), icon: 'i-lucide-zap', onSelect: () => addTarget('action') },
                ]">
                  <UButton icon="i-lucide-plus" size="xs" color="primary" variant="soft" />
                </UDropdownMenu>
              </div>
              <div class="space-y-1">
                <button
                  v-for="target in store.draft.snap.areas"
                  :key="target.id"
                  class="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors"
                  :class="selectedId === target.id
                    ? 'bg-primary/10 text-primary'
                    : 'hover:bg-elevated/50'"
                  @click="selectTarget(target.id)"
                >
                  <UIcon
                    :name="target.kind === 'area' ? 'i-lucide-square' : 'i-lucide-zap'"
                    class="size-4 shrink-0"
                  />
                  <span class="truncate">{{ target.name }}</span>
                </button>
              </div>
            </div>

            <!-- 중앙: vue-konva 캔버스 -->
            <SnapCanvas
              :areas="areas"
              :selected-id="selectedId"
              @select="selectTarget"
              @update="(id, patch) => updateTarget(id, patch)"
            />

            <!-- 우측: 속성 패널 -->
            <UCard variant="subtle">
              <SnapProperties
                :target="selectedTarget"
                @update="(patch) => selectedId && updateTarget(selectedId, patch)"
                @delete="selectedId && deleteTarget(selectedId)"
              />
            </UCard>
          </div>
        </template>

        <!-- Tab 2: Sector Mapping -->
        <template #mapping>
          <div class="py-4">
            <SectorMapping
              :sector-count="store.draft.overlay.sector_count"
              :targets="store.draft.snap.areas"
              :mapping="store.draft.throw.mapping"
              :long-throw-mapping="store.draft.throw.long_throw_mapping"
              @update:mapping="store.draft!.throw.mapping = $event"
              @update:long-throw-mapping="store.draft!.throw.long_throw_mapping = $event"
            />
          </div>
        </template>

        <!-- Tab 3: Keyboard Chains -->
        <template #chains>
          <div class="py-4">
            <ChainEditor
              :chains="store.draft.keyboard.chains"
              :targets="store.draft.snap.areas"
              @update:chains="store.draft!.keyboard.chains = $event"
            />
          </div>
        </template>
      </UTabs>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
  </div>
</template>
