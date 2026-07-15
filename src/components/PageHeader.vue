<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <div>
    <h2 class="text-base font-medium">{{ title }}</h2>
    <p v-if="description" class="text-sm text-muted">{{ description }}</p>
  </div>
</template>
