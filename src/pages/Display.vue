<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'
import ColorLockField from '@/components/ColorLockField.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('display.title')">
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
          <!-- 커서 표시기 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-mouse-pointer" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('display.cursorIndicator') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('display.cursorIndicator')" :description="t('display.cursorIndicatorDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.overlay.cursor.indicator" />
              </div>
            </UFormField>
            <template v-if="store.draft.overlay.cursor.indicator">
              <UFormField :label="t('display.cursorColor')">
                <UColorPicker v-model="store.draft.overlay.cursor.color" />
              </UFormField>
              <UFormField :label="t('display.cursorRadius')">
                <USlider
                  v-model="store.draft.overlay.cursor.radius"
                  :min="5"
                  :max="50"
                  :step="1"
                  class="w-full"
                />
                <template #hint>{{ store.draft.overlay.cursor.radius }}px</template>
              </UFormField>
              <UFormField :label="t('display.cursorOpacity')">
                <USlider
                  v-model="store.draft.overlay.cursor.opacity"
                  :min="0.1"
                  :max="1"
                  :step="0.05"
                  class="w-full"
                />
                <template #hint>{{ Math.round(store.draft.overlay.cursor.opacity * 100) }}%</template>
              </UFormField>
            </template>
          </section>

          <!-- Snap Preview -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-square-dashed" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('display.snapPreview') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('display.snapPreview')" :description="t('display.snapPreviewDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.overlay.snap_preview.enabled" />
              </div>
            </UFormField>
            <template v-if="store.draft.overlay.snap_preview.enabled">
              <ColorLockField v-model="store.draft.overlay.snap_preview.colors" />
            </template>
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
