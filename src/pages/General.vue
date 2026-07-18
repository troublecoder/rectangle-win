<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import { i18n, type SupportedLocale } from '@/i18n'
import SaveActions from '@/components/SaveActions.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

function changeLanguage(lang: string) {
  i18n.global.locale.value = lang as SupportedLocale
  if (store.draft) {
    store.draft.general.language = lang
  }
}

const languageItems = [
  { label: 'English', value: 'en' },
  { label: '한국어', value: 'ko' },
]
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('general.title')">
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
          <!-- 시작 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-power" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.startup') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.launchAtLogin')" :description="t('general.launchAtLoginDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.launch_at_login" />
              </div>
            </UFormField>
            <UFormField :label="t('general.startMinimized')" :description="t('general.startMinimizedDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.start_minimized" />
              </div>
            </UFormField>
          </section>

          <!-- 시스템 트레이 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-tray" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.tray') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.showInTray')" :description="t('general.showInTrayDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.general.show_in_tray" />
              </div>
            </UFormField>
          </section>

          <!-- 창 간격 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-move-horizontal" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.windowGap') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.snapMargin')" :description="t('general.snapMarginDesc')">
              <USlider
                v-model="store.draft.general.snap_margin"
                :min="0"
                :max="30"
                :step="1"
                class="w-full"
              />
              <template #hint>{{ store.draft.general.snap_margin }}px</template>
            </UFormField>
          </section>

          <!-- 언어 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-languages" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('general.language') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('general.language')" :description="t('general.languageDesc')">
              <USelect
                :model-value="store.draft.general.language"
                :items="languageItems"
                value-key="value"
                class="w-full"
                @update:model-value="changeLanguage($event as string)"
              />
            </UFormField>
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
