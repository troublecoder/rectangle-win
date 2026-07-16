<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/features/config-store'
import PageHeader from '@/components/PageHeader.vue'
import SaveBar from '@/components/SaveBar.vue'

const { t } = useI18n()
const store = useConfigStore()

onMounted(() => store.load())
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
        <!-- Trigger modifier 안내 (throw 와 공유, Win+Alt 고정) -->
        <USection :title="t('keyboard.triggerModifiers')">
          <UAlert
            color="info"
            variant="soft"
            icon="i-lucide-info"
            :description="t('keyboard.sharedModifiersNote')"
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
