<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'

const { t, locale } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

function changeLanguage(lang: string) {
  locale.value = lang
  if (store.draft) {
    store.draft.general.language = lang
  }
}
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <PageHeader :title="t('general.title')" :description="t('general.description')" />

    <template v-if="store.draft">
      <!-- Startup -->
      <USection :title="t('general.startup')">
        <UFormField :label="t('general.launchAtLogin')" :description="t('general.launchAtLoginDesc')">
          <USwitch v-model="store.draft.general.launch_at_login" />
        </UFormField>
        <UFormField :label="t('general.startMinimized')" :description="t('general.startMinimizedDesc')">
          <USwitch v-model="store.draft.general.start_minimized" />
        </UFormField>
      </USection>

      <!-- Tray -->
      <USection :title="t('general.tray')">
        <UFormField :label="t('general.showInTray')" :description="t('general.showInTrayDesc')">
          <USwitch v-model="store.draft.general.show_in_tray" />
        </UFormField>
      </USection>

      <!-- Language -->
      <USection :title="t('general.language')">
        <UFormField :label="t('general.language')" :description="t('general.languageDesc')">
          <USelect
            :model-value="store.draft.general.language"
            :items="[
              { label: 'English', value: 'en' },
              { label: '한국어', value: 'ko' },
            ]"
            value-key="value"
            @update:model-value="changeLanguage($event as string)"
          />
        </UFormField>
      </USection>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
    <UAlert
      v-else-if="store.error"
      color="error"
      variant="soft"
      icon="i-lucide-alert-circle"
      :title="store.error"
    />
  </div>
</template>
