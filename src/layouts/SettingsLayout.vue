<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { NavigationMenuItem } from '@nuxt/ui'
import { useConfigStore } from '@/features/config-store'

const { t } = useI18n()
const store = useConfigStore()

const navItems = computed<NavigationMenuItem[][]>(() => [
  [
    { label: t('nav.general'), icon: 'i-lucide-settings', to: '/general' },
    { label: t('nav.snapEditor'), icon: 'i-lucide-layout-grid', to: '/snap-editor' },
    { label: t('nav.throw'), icon: 'i-lucide-mouse-pointer-click', to: '/throw' },
    { label: t('nav.display'), icon: 'i-lucide-monitor', to: '/display' },
    { label: t('nav.about'), icon: 'i-lucide-info', to: '/about' },
  ],
])

async function toggleEnabled(value: boolean) {
  if (!store.draft) return
  store.draft.keyboard.enabled = value
  await store.save()
}
</script>

<template>
  <UDashboardGroup>
    <UDashboardSidebar :default-size="18" :min-size="16" :max-size="22">
      <template #header>
        <div class="flex w-full items-center justify-between">
          <div class="flex items-center gap-2">
            <UIcon name="i-lucide-square" class="size-5 text-primary" />
            <span class="text-sm font-semibold">{{ t('app.name') }}</span>
          </div>
          <UColorModeButton />
        </div>
      </template>

      <template #default>
        <UNavigationMenu :items="navItems" orientation="vertical" class="w-full" />
      </template>

      <template #footer>
        <div class="flex w-full flex-col gap-2">
          <USeparator />
          <div class="flex items-center justify-between gap-2">
            <div class="flex min-w-0 items-center gap-2">
              <UIcon name="i-lucide-zap" class="size-4 shrink-0 text-muted" />
              <span class="truncate text-sm whitespace-nowrap">{{ t('nav.enableSnap') }}</span>
            </div>
            <USwitch
              :model-value="store.draft?.keyboard.enabled ?? false"
              @update:model-value="toggleEnabled($event as boolean)"
            />
          </div>
          <UButton
            :label="t('nav.quit')"
            icon="i-lucide-power"
            color="error"
            variant="ghost"
            block
          />
        </div>
      </template>
    </UDashboardSidebar>

    <RouterView />
  </UDashboardGroup>
</template>
