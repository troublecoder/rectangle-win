<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{
  dirty: boolean
  saving?: boolean
}>()
const emit = defineEmits<{ save: []; reset: [] }>()
const { t } = useI18n()
</script>

<template>
  <div class="flex items-center justify-end gap-2 border-t border-default pt-4">
    <UButton
      :label="t('common.reset')"
      icon="i-lucide-rotate-ccw"
      color="neutral"
      variant="ghost"
      :disabled="!dirty || saving"
      @click="emit('reset')"
    />
    <UButton
      :label="t('common.save')"
      icon="i-lucide-save"
      color="primary"
      :loading="saving"
      :disabled="!dirty"
      @click="emit('save')"
    />
  </div>
</template>
