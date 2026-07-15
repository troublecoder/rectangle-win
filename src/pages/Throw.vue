<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const modifierOptions = [
  { label: 'Win', value: 'Win' },
  { label: 'Ctrl', value: 'Ctrl' },
  { label: 'Alt', value: 'Alt' },
  { label: 'Shift', value: 'Shift' },
]
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <PageHeader :title="t('throw.title')" :description="t('throw.description')" />

    <template v-if="store.draft">
      <!-- Trigger Modifiers -->
      <USection :title="t('throw.triggerModifiers')">
        <UFormField :description="t('throw.triggerModifiersDesc')">
          <USelectMenu
            v-model="store.draft.throw.trigger_modifiers"
            :items="modifierOptions"
            multiple
            value-key="value"
            class="w-full"
          />
        </UFormField>
      </USection>

      <!-- Long Throw -->
      <USection :title="t('throw.longThrow')">
        <UFormField :label="t('throw.longThrowEnabled')" :description="t('throw.longThrowEnabledDesc')">
          <USwitch v-model="store.draft.throw.long_throw_enabled" />
        </UFormField>
        <UFormField
          v-if="store.draft.throw.long_throw_enabled"
          :label="t('throw.longThrowDistance')"
          :description="t('throw.longThrowDistanceDesc')"
        >
          <USlider
            v-model="store.draft.throw.long_throw_distance"
            :min="100"
            :max="1000"
            :step="50"
            class="w-full"
          />
          <template #hint>
            {{ store.draft.throw.long_throw_distance }}px
          </template>
        </UFormField>
      </USection>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
  </div>
</template>
