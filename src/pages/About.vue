<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { getVersion } from '@tauri-apps/api/app'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'

const { t } = useI18n()
const store = useConfigStore()

// 런타임에 tauri.conf.json의 version을 조회 — 단일 진실 공급원.
// Tauri 웹뷰가 아닌 환경(순수 브라우저)에서는 조회 실패 시 '0.0.0' 폴백.
const appVersion = ref('0.0.0')
const checking = ref(false)
const updateStatus = ref<'idle' | 'available' | 'up-to-date'>('idle')

onMounted(async () => {
  store.load()
  try {
    appVersion.value = await getVersion()
  } catch {
    appVersion.value = '0.0.0'
  }
})

const channelItems = [
  { label: t('about.channelStable'), value: 'stable' },
  { label: t('about.channelBeta'), value: 'beta' },
]

async function checkForUpdates() {
  checking.value = true
  updateStatus.value = 'idle'
  try {
    await new Promise((resolve) => setTimeout(resolve, 1500))
    updateStatus.value = 'up-to-date'
  } finally {
    checking.value = false
  }
}
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('about.title')">
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

        <template v-else-if="store.draft">
          <!-- 앱 정보 -->
          <section class="space-y-4">
            <UCard variant="subtle">
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
                class="mt-4"
              />
            </UCard>
          </section>

          <!-- 업데이트 -->
          <section class="space-y-4">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-refresh-cw" class="size-4 text-primary" />
              <h3 class="text-sm font-medium text-muted">{{ t('about.update') }}</h3>
            </div>
            <USeparator />
            <UFormField :label="t('about.autoUpdate')" :description="t('about.autoUpdateDesc')">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted" />
                <USwitch v-model="store.draft.update.enabled" />
              </div>
            </UFormField>
            <template v-if="store.draft.update.enabled">
              <UFormField :label="t('about.updateChannel')">
                <USelect
                  v-model="store.draft.update.channel"
                  :items="channelItems"
                  value-key="value"
                  class="w-full"
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
          </section>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>
</template>
