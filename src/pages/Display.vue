<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const reticleStyles = [
  { label: 'Pie', value: 'pie' },
  { label: 'Crosshair', value: 'crosshair' },
  { label: 'Minimal', value: 'minimal' },
]

const sectorOptions = [
  { label: '4', value: 4 },
  { label: '8', value: 8 },
  { label: '12', value: 12 },
]
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <PageHeader :title="t('display.title')" :description="t('display.description')" />

    <template v-if="store.draft">
      <!-- Reticle -->
      <USection :title="t('display.reticle')">
        <UFormField :label="t('display.reticleStyle')">
          <USelect
            v-model="store.draft.overlay.reticle_style"
            :items="reticleStyles"
            value-key="value"
          />
        </UFormField>
      </USection>

      <!-- Cursor Indicator -->
      <USection :title="t('display.cursorIndicator')">
        <UFormField :label="t('display.cursorIndicatorDesc')">
          <USwitch v-model="store.draft.overlay.cursor_indicator" />
        </UFormField>

        <template v-if="store.draft.overlay.cursor_indicator">
          <UFormField :label="t('display.cursorColor')">
            <UColorPicker v-model="store.draft.overlay.cursor_color" />
          </UFormField>
          <UFormField :label="t('display.cursorRadius')">
            <USlider
              v-model="store.draft.overlay.cursor_radius"
              :min="5"
              :max="50"
              :step="1"
              class="w-full"
            />
            <template #hint>{{ store.draft.overlay.cursor_radius }}px</template>
          </UFormField>
          <UFormField :label="t('display.cursorOpacity')">
            <USlider
              v-model="store.draft.overlay.cursor_opacity"
              :min="0.1"
              :max="1"
              :step="0.05"
              class="w-full"
            />
            <template #hint>{{ Math.round(store.draft.overlay.cursor_opacity * 100) }}%</template>
          </UFormField>
        </template>
      </USection>

      <!-- Sectors -->
      <USection :title="t('display.sectors')">
        <UFormField :label="t('display.sectorCount')" :description="t('display.sectorCountDesc')">
          <USelect
            v-model="store.draft.overlay.sector_count"
            :items="sectorOptions"
            value-key="value"
          />
        </UFormField>
        <UFormField :label="t('display.sectorHighlightColor')">
          <UColorPicker v-model="store.draft.overlay.sector_highlight_color" />
        </UFormField>
      </USection>

      <!-- Snap Preview -->
      <USection>
        <UFormField :label="t('display.snapPreview')" :description="t('display.snapPreviewDesc')">
          <USwitch v-model="store.draft.overlay.snap_preview" />
        </UFormField>
      </USection>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
  </div>
</template>
