<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'

const { t } = useI18n()
const store = useConfigStore()

const appVersion = ref('0.1.0')
const checking = ref(false)
const updateStatus = ref<'idle' | 'available' | 'up-to-date'>('idle')

onMounted(() => store.load())

const channelItems = [
  { label: t('about.channelStable'), value: 'stable' },
  { label: t('about.channelBeta'), value: 'beta' },
]

async function checkForUpdates() {
  checking.value = true
  updateStatus.value = 'idle'
  try {
    // TODO: Tauri updater plugin 연동
    await new Promise(resolve => setTimeout(resolve, 1500))
    updateStatus.value = 'up-to-date'
  } finally {
    checking.value = false
  }
}
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <PageHeader :title="t('about.title')" />

    <template v-if="store.draft">
      <!-- App Info -->
      <USection>
        <div class="flex items-center gap-4">
          <div class="flex size-12 items-center justify-center rounded-lg bg-primary/10">
            <UIcon name="i-lucide-square" class="size-6 text-primary" />
          </div>
          <div>
            <p class="font-semibold">{{ t('app.name') }}</p>
            <p class="text-sm text-muted">{{ t('about.version') }} {{ appVersion }}</p>
          </div>
        </div>
        <UButton
          :label="t('about.github')"
          icon="i-lucide-github"
          color="neutral"
          variant="outline"
          to="https://github.com/troublecoder/rectangle-win"
          target="_blank"
        />
      </USection>

      <!-- Auto Update -->
      <USection :title="t('about.update')">
        <UFormField :label="t('about.autoUpdate')" :description="t('about.autoUpdateDesc')">
          <USwitch v-model="store.draft.update.enabled" />
        </UFormField>

        <template v-if="store.draft.update.enabled">
          <UFormField :label="t('about.updateChannel')">
            <USelect
              v-model="store.draft.update.channel"
              :items="channelItems"
              value-key="value"
            />
          </UFormField>

          <div class="flex items-center gap-3">
            <UButton
              :label="checking ? t('about.checking') : t('about.checkForUpdates')"
              icon="i-lucide-refresh-cw"
              :loading="checking"
              color="primary"
              variant="soft"
              @click="checkForUpdates"
            />
            <UBadge
              v-if="updateStatus === 'up-to-date'"
              color="success"
              variant="soft"
              :label="t('about.upToDate')"
              icon="i-lucide-check"
            />
            <UBadge
              v-else-if="updateStatus === 'available'"
              color="warning"
              variant="soft"
              :label="t('about.updateAvailable')"
              icon="i-lucide-arrow-up-circle"
            />
          </div>
        </template>
      </USection>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
  </div>
</template>
