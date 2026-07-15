<script setup lang="ts">
defineProps<{
  title?: string
}>()
</script>

<template>
  <UCard variant="subtle">
    <template v-if="title" #header>
      <h3 class="text-sm font-medium">{{ title }}</h3>
    </template>
    <div class="space-y-4">
      <slot />
    </div>
  </UCard>
</template>
