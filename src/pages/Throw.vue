<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import * as api from '@/features/api'
import SaveActions from '@/components/SaveActions.vue'
import UHotkeyInput from '@/components/UHotkeyInput.vue'
import SectorMappingTable from '@/components/SectorMappingTable.vue'
import type { SnapTarget } from '@/entities/config'

const { t } = useI18n()
const store = useConfigStore()
const builtinTargets = ref<SnapTarget[]>([])

onMounted(async () => {
  store.load()
  try {
    builtinTargets.value = await api.getBuiltinTargets()
  } catch {
    builtinTargets.value = []
  }
})

// 빌트인 + 커스텀 영역을 합친 전체 목록 (매핑 선택지용).
const allTargets = computed<SnapTarget[]>(() => {
  const custom = store.draft?.snap.areas ?? []
  return [...builtinTargets.value, ...custom]
})
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('throw.title')">
        <template #right>
          <SaveActions
            v-if="store.draft"
            :dirty="store.isDirty"
            :saving="store.saving"
            @save="store.save()"
            @reset="store.reset()"
          />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="max-w-3xl space-y-10 py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>
        <UAlert
          v-else-if="store.error"
          color="error"
          variant="soft"
          icon="i-lucide-alert-circle"
          :title="store.error"
        />

        <template v-else-if="store.draft">
          <!-- 핫키 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-keyboard" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.triggerModifiers') }}</h3>
            </div>
            <USeparator />
            <UFormField :description="t('throw.triggerModifiersDesc')">
              <UHotkeyInput v-model="store.draft.throw.trigger_modifiers" />
            </UFormField>
          </section>

          <!-- Long Throw -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-arrow-right" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.longThrow') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('throw.longThrowEnabled')" :description="t('throw.longThrowEnabledDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.throw.long_throw.enabled" />
              </div>
            </UFormField>
            <UFormField
              v-if="store.draft.throw.long_throw.enabled"
              :label="t('throw.longThrowDistance')"
              :description="t('throw.longThrowDistanceDesc')"
            >
              <USlider
                v-model="store.draft.throw.long_throw.distance"
                :min="100"
                :max="1000"
                :step="50"
                class="w-full"
              />
              <template #hint>{{ store.draft.throw.long_throw.distance }}px</template>
            </UFormField>
          </section>

          <!-- Sector 매핑 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-pie-chart" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('throw.sectorMapping') }}</h3>
            </div>
            <USeparator />
            <SectorMappingTable
              :targets="allTargets"
              :mapping="store.draft.throw.mapping"
              :long-throw-mapping="store.draft.throw.long_throw.mapping"
              @update:mapping="store.draft!.throw.mapping = $event"
              @update:long-throw-mapping="store.draft!.throw.long_throw.mapping = $event"
            />
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
