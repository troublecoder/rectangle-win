<script setup lang="ts">
import { h, onMounted, ref, computed, watch, nextTick, resolveComponent } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ColumnDef } from '@tanstack/vue-table'
import { useConfigStore } from '@/features/config-store'
import SaveActions from '@/components/SaveActions.vue'
import SnapCanvas from '@/components/SnapCanvas.vue'
import SnapProperties from '@/components/SnapProperties.vue'
import type { SnapTarget } from '@/entities/config'

const { t } = useI18n()
const store = useConfigStore()
// 테이블 루트를 참조하여 expand 시 해당 행으로 스크롤
// UTable ref는 컴포넌트 인스턴스이므로 $el로 실제 DOM에 접근
const tableRef = ref<any>(null)

onMounted(() => store.load())

// TanStack ExpandedState는 Record<string, boolean>. ref<string[]>가 아님에 주의.
// Accordion 동작: 한 번에 하나의 행만 확장. 새로운 행을 확장하면 기존 행은 닫힌다.
const expanded = ref<Record<string, boolean>>({})

// 삭제 확인 다이얼로그 상태.
const pendingDeleteId = ref<string | null>(null)

// 사용자가 추가한 영역인지 (프리셋 기본 영역은 삭제 불가).
function isCustomArea(id: string): boolean {
  return id.startsWith('area-')
}

// 가장 최근에 확장된 행 id (스크롤용)
let lastExpandedId: string | null = null

watch(expanded, (val) => {
  const openIds = Object.keys(val).filter((k) => val[k])
  if (openIds.length > 1) {
    // 가장 최근에 열린 하나만 남기고 닫기
    expanded.value = { [openIds[openIds.length - 1]]: true }
    return
  }
  const currentId = openIds[0] ?? null
  if (currentId && currentId !== lastExpandedId) {
    lastExpandedId = currentId
    // expand 렌더링 완료 후 해당 행으로 스크롤
    nextTick(() => scrollToRow(currentId))
  } else if (!currentId) {
    lastExpandedId = null
  }
}, { deep: true })

// 테이블 tbody 내에서 해당 id의 행(tr)을 찾아 scrollIntoView.
// UTable은 tr에 data 식별자를 노출하지 않으므로, 행 인덱스 기반으로 탐색.
function scrollToRow(id: string) {
  if (!store.draft || !tableRef.value) return
  const rowIndex = store.draft.snap.areas.findIndex((a) => a.id === id)
  if (rowIndex < 0) return
  // UTable ref에서 실제 DOM 추출 (컴포넌트 인스턴스의 $el 또는 자기 자신)
  const root = (tableRef.value?.$el ?? tableRef.value) as HTMLElement | null
  if (!root) return
  const tbody = root.querySelector('tbody')
  if (!tbody) return
  // 데이터 행 tr (aria-hidden placeholder 제외). 확장 콘텐츠 tr은 데이터 행 바로 뒤.
  const rows = Array.from(tbody.querySelectorAll(':scope > tr:not([aria-hidden="true"])'))
  const targetRow = rows[rowIndex] as HTMLElement | undefined
  if (targetRow) {
    targetRow.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

// Nuxt UI 컴포넌트를 h() 안에서 참조하려면 resolveComponent 필요.
const UButton = resolveComponent('UButton')

function updateTarget(id: string, patch: Partial<SnapTarget>) {
  if (!store.draft) return
  const idx = store.draft.snap.areas.findIndex((a) => a.id === id)
  if (idx >= 0) {
    store.draft.snap.areas[idx] = { ...store.draft.snap.areas[idx], ...patch } as SnapTarget
  }
}

function deleteTarget(id: string) {
  pendingDeleteId.value = id
}

function confirmDelete() {
  const id = pendingDeleteId.value
  if (!id || !store.draft) return
  store.draft.snap.areas = store.draft.snap.areas.filter((a) => a.id !== id)
  for (const map of [store.draft.throw.mapping, store.draft.throw.long_throw.mapping]) {
    for (const [sector, targetId] of Object.entries(map)) {
      if (targetId === id) delete map[sector]
    }
  }
  const next = { ...expanded.value }
  delete next[id]
  expanded.value = next
  pendingDeleteId.value = null
}

function cancelDelete() {
  pendingDeleteId.value = null
}

const pendingDeleteName = computed(() => {
  if (!pendingDeleteId.value || !store.draft) return ''
  return store.draft.snap.areas.find((a) => a.id === pendingDeleteId.value)?.name ?? ''
})

function addTarget() {
  if (!store.draft) return
  const id = `area-${Date.now()}`
  const target: SnapTarget = {
    kind: 'area',
    id,
    name: t('snapEditor.newArea'),
    x_ratio: 0.1,
    y_ratio: 0.1,
    w_ratio: 0.3,
    h_ratio: 0.3,
  }
  store.draft.snap.areas.unshift(target)
  expanded.value = { [id]: true }
}

const columns = computed<ColumnDef<SnapTarget>[]>(() => [
  {
    id: 'expand',
    header: '',
    cell: ({ row }) =>
      h(
        UButton,
        {
          icon: row.getIsExpanded() ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right',
          color: 'neutral',
          variant: 'ghost',
          size: 'xs',
          onClick: () => row.toggleExpanded(),
        },
      ),
  },
  {
    accessorKey: 'name',
    header: () => t('snapEditor.name'),
  },
  {
    id: 'actions',
    header: '',
    cell: ({ row }) =>
      isCustomArea(row.original.id)
        ? h(UButton, {
            icon: 'i-lucide-trash-2',
            color: 'error',
            variant: 'ghost',
            size: 'xs',
            onClick: () => deleteTarget(row.original.id),
          })
        : undefined,
  },
])
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('snapEditor.title')">
        <template #right>
          <div class="flex items-center gap-2">
            <UButton
              icon="i-lucide-plus"
              color="primary"
              variant="soft"
              size="sm"
              :label="t('snapEditor.addTarget')"
              @click="addTarget"
            />
            <SaveActions
              v-if="store.draft"
              :dirty="store.isDirty"
              :saving="store.saving"
              @save="store.save()"
              @reset="store.reset()"
            />
          </div>
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <UContainer class="py-8">
        <div v-if="store.loading" class="py-8 text-center text-muted">
          <UIcon name="i-lucide-loader-circle" class="size-5 animate-spin" />
        </div>

        <UAlert
          v-else-if="store.error"
          color="error"
          variant="soft"
          icon="i-lucide-alert-circle"
          :title="store.error"
        />

        <template v-else-if="store.draft">
          <UCard variant="subtle">
            <UTable
              ref="tableRef"
              :data="store.draft.snap.areas.filter(a => a.kind === 'area' && isCustomArea(a.id))"
              :columns="columns"
              v-model:expanded="expanded"
              :get-row-id="(row: SnapTarget) => row.id"
              class="w-full"
            >
              <!-- 인라인 템플릿 방식: h() 반환값을 {{ }} 로 보간하면 VNode가 텍스트로 렌더되므로
                   직접 컴포넌트를 배치한다. -->
              <template #expanded="{ row }">
                <div class="grid gap-6 px-4 py-4 lg:grid-cols-[1fr_320px]">
                  <div class="space-y-4">
                    <SnapProperties
                      :target="row.original"
                      @update="(patch) => updateTarget(row.original.id, patch)"
                    />
                  </div>
                  <SnapCanvas
                    v-if="row.original.kind === 'area'"
                    :area="row.original"
                    @update="(id, patch) => updateTarget(id, patch)"
                  />
                </div>
              </template>
            </UTable>
          </UCard>
        </template>
      </UContainer>
    </template>
  </UDashboardPanel>

  <!-- 삭제 확인 다이얼로그
       UModal을 UDashboardPanel 밖에 두어야 DashboardPanel의 default slot과
       충돌하지 않음 (Panel 내부에 두면 header/body slot이 렌더되지 않는 버그). -->
  <UModal :open="pendingDeleteId !== null" @update:open="(v: boolean) => { if (!v) cancelDelete() }">
    <template #content>
      <div class="space-y-4 p-6">
        <div class="flex items-center gap-3">
          <div class="flex size-10 items-center justify-center rounded-full bg-error/10">
            <UIcon name="i-lucide-trash-2" class="size-5 text-error" />
          </div>
          <div>
            <h3 class="text-base font-medium">{{ t('snapEditor.deleteTitle') }}</h3>
            <p class="text-sm text-muted">{{ t('snapEditor.deleteConfirm', { name: pendingDeleteName }) }}</p>
          </div>
        </div>
        <div class="flex justify-end gap-2">
          <UButton :label="t('common.cancel')" color="neutral" variant="ghost" @click="cancelDelete" />
          <UButton :label="t('common.delete')" icon="i-lucide-trash-2" color="error" variant="solid" @click="confirmDelete" />
        </div>
      </div>
    </template>
  </UModal>
</template>
