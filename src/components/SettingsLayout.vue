<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import type { NavigationMenuItem } from '@nuxt/ui'
import { en as enLocale, ko as koLocale } from '@nuxt/ui/locale'

const { t, locale } = useI18n()

function changeLocale(value: string) {
  locale.value = value
}
const route = useRoute()

const navItems = computed<NavigationMenuItem[][]>(() => [
  [
    { label: t('nav.general'), icon: 'i-lucide-settings', to: '/general' },
    { label: t('nav.throw'), icon: 'i-lucide-mouse-pointer-click', to: '/throw' },
    { label: t('nav.snapEditor'), icon: 'i-lucide-layout-grid', to: '/snap-editor' },
    { label: t('nav.keyboard'), icon: 'i-lucide-keyboard', to: '/keyboard' },
    { label: t('nav.display'), icon: 'i-lucide-monitor', to: '/display' },
    { label: t('nav.about'), icon: 'i-lucide-info', to: '/about' },
  ],
])

const locales = [enLocale, koLocale]

const pageTitle = computed(() => {
  const name = (route.name as string | undefined)?.replace('-', '') ?? ''
  if (!name) return ''
  // snap-editor → snapEditor 매핑
  const keyMap: Record<string, string> = { 'snapeditor': 'snapEditor' }
  const key = keyMap[name] ?? name
  return t(`nav.${key}`)
})
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-default text-default">
    <!-- Sidebar -->
    <aside
      class="flex w-60 shrink-0 flex-col border-r border-default bg-elevated/40"
    >
      <!-- Header: app name + color mode -->
      <div class="flex items-center justify-between gap-2 px-4 py-4">
        <div class="flex items-center gap-2">
          <UIcon name="i-lucide-square" class="size-5 text-primary" />
          <span class="text-sm font-semibold">{{ t('app.name') }}</span>
        </div>
        <UColorModeButton />
      </div>

      <USeparator />

      <!-- Navigation -->
      <nav class="flex-1 overflow-y-auto p-3">
        <UNavigationMenu
          :items="navItems"
          orientation="vertical"
          class="w-full"
        />
      </nav>

      <USeparator />

      <!-- Footer: locale select + pause/quit -->
      <div class="space-y-3 p-3">
        <div class="flex items-center gap-2">
          <UIcon name="i-lucide-languages" class="size-4 text-muted" />
          <ULocaleSelect
            :model-value="locale"
            :locales="locales"
            size="sm"
            class="flex-1"
            @update:model-value="changeLocale($event as string)"
          />
        </div>
        <div class="flex gap-2">
          <UButton
            :label="t('nav.pause')"
            icon="i-lucide-pause"
            color="neutral"
            variant="soft"
            size="sm"
            block
          />
          <UButton
            :label="t('nav.quit')"
            icon="i-lucide-power"
            color="error"
            variant="ghost"
            size="sm"
            block
          />
        </div>
      </div>
    </aside>

    <!-- Content -->
    <main class="flex flex-1 flex-col overflow-hidden">
      <header
        class="flex items-center gap-3 border-b border-default px-6 py-4"
      >
        <h1 class="text-lg font-semibold">{{ pageTitle }}</h1>
      </header>
      <div class="flex-1 overflow-y-auto p-6">
        <RouterView />
      </div>
    </main>
  </div>
</template>
