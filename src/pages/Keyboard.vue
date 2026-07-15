<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'
import type { ModifierMode } from '@/entities/config'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())

const modifierOptions = [
  { label: 'Win', value: 'Win' },
  { label: 'Ctrl', value: 'Ctrl' },
  { label: 'Alt', value: 'Alt' },
  { label: 'Shift', value: 'Shift' },
]

const modeItems = computed(() => [
  {
    label: t('keyboard.modifierShared'),
    description: t('keyboard.modifierSharedDesc'),
    value: 'Shared' as ModifierMode,
  },
  {
    label: t('keyboard.modifierSeparate'),
    description: t('keyboard.modifierSeparateDesc'),
    value: 'Separate' as ModifierMode,
  },
  {
    label: t('keyboard.modifierOverrideOs'),
    description: t('keyboard.modifierOverrideOsDesc'),
    value: 'OverrideOs' as ModifierMode,
  },
])
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <PageHeader :title="t('keyboard.title')" :description="t('keyboard.description')" />

    <template v-if="store.draft">
      <!-- Enable -->
      <USection>
        <UFormField :label="t('keyboard.enabled')">
          <USwitch v-model="store.draft.keyboard.enabled" />
        </UFormField>
      </USection>

      <template v-if="store.draft.keyboard.enabled">
        <!-- Modifier Mode -->
        <USection :title="t('keyboard.modifierMode')">
          <URadioGroup
            v-model="store.draft.keyboard.modifier_mode"
            :items="modeItems"
            value-key="value"
          >
            <template #label="{ item }">
              <span class="font-medium">{{ item.label }}</span>
              <p class="text-xs text-muted">{{ item.description }}</p>
            </template>
          </URadioGroup>

          <!-- OverrideOs 경고 -->
          <UAlert
            v-if="store.draft.keyboard.modifier_mode === 'OverrideOs'"
            color="warning"
            variant="soft"
            icon="i-lucide-alert-triangle"
            :description="t('keyboard.overrideWarning')"
          />
        </USection>

        <!-- Trigger Modifiers (Shared 모드가 아닐 때만) -->
        <USection v-if="store.draft.keyboard.modifier_mode !== 'Shared'" :title="t('keyboard.triggerModifiers')">
          <USelectMenu
            v-model="store.draft.keyboard.trigger_modifiers"
            :items="modifierOptions"
            multiple
            value-key="value"
            class="w-full"
          />
        </USection>

        <!-- Cycle Timeout -->
        <USection :title="t('keyboard.cycleTimeout')">
          <UFormField :label="t('keyboard.cycleTimeoutDesc')">
            <UInputNumber
              v-model="store.draft.keyboard.cycle_timeout_ms"
              :min="500"
              :max="5000"
              :step="100"
            />
            <template #hint>ms</template>
          </UFormField>
        </USection>
      </template>

      <SaveBar :dirty="store.isDirty" :saving="store.saving" @save="store.save()" @reset="store.reset()" />
    </template>

    <div v-else-if="store.loading" class="py-8 text-center text-muted">
      <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
    </div>
  </div>
</template>
