<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

const items = computed<NavigationMenuItem[][]>([
  [
    { label: 'Settings', type: 'label' },
    { label: 'General', icon: 'i-lucide-settings', to: '/general' },
    { label: 'Throw', icon: 'i-lucide-mouse-pointer-click', to: '/throw' },
    { label: 'Snap Editor', icon: 'i-lucide-layout-grid', to: '/snap-editor' },
    { label: 'Keyboard', icon: 'i-lucide-keyboard', to: '/keyboard' },
    { label: 'Display', icon: 'i-lucide-monitor', to: '/display' },
    { label: 'About', icon: 'i-lucide-info', to: '/about' },
  ],
  [
    { label: 'Pause', icon: 'i-lucide-pause' },
    { label: 'Quit', icon: 'i-lucide-log-out' },
  ],
])
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-default text-default">
    <aside class="w-56 shrink-0 border-r border-default flex flex-col">
      <div class="p-4 border-b border-default">
        <h1 class="text-lg font-bold flex items-center gap-2">
          <span class="inline-block size-5 rounded bg-primary-500"></span>
          Rectangle Win
        </h1>
      </div>
      <UNavigationMenu
        orientation="vertical"
        :items="items"
        highlight
        class="flex-1 data-[orientation=vertical]:w-full border-0"
      />
    </aside>
    <main class="flex-1 overflow-auto p-6">
      <RouterView />
    </main>
  </div>
</template>
