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
const UBadge = resolveComponent('UBadge')
const UDropdownMenu = resolveComponent('UDropdownMenu')

function updateTarget(id: string, patch: Partial<SnapTarget>) {
  if (!store.draft) return
  const idx = store.draft.snap.areas.findIndex((a) => a.id === id)
  if (idx >= 0) {
    store.draft.snap.areas[idx] = { ...store.draft.snap.areas[idx], ...patch } as SnapTarget
  }
}

function deleteTarget(id: string) {
  if (!store.draft) return
  store.draft.snap.areas = store.draft.snap.areas.filter((a) => a.id !== id)
  // Throw 매핑에서도 해당 id 제거 (고립 참조 방지).
  // throw.mapping 과 throw.long_throw.mapping 모두 정리.
  for (const map of [store.draft.throw.mapping, store.draft.throw.long_throw.mapping]) {
    for (const [sector, targetId] of Object.entries(map)) {
      if (targetId === id) delete map[sector]
    }
  }
  // expanded 상태에서도 제거 (TanStack ExpandedState는 Record<string, boolean>)
  const next = { ...expanded.value }
  delete next[id]
  expanded.value = next
}

function addTarget(kind: 'area' | 'action') {
  if (!store.draft) return
  const id = kind === 'area' ? `area-${Date.now()}` : `action-${Date.now()}`
  const name = kind === 'area' ? t('snapEditor.newArea') : t('snapEditor.newAction')
  const target: SnapTarget = kind === 'area'
    ? { kind: 'area', id, name, x_ratio: 0.1, y_ratio: 0.1, w_ratio: 0.3, h_ratio: 0.3 }
    : { kind: 'action', id, name, action: 'Maximize' }
  // 새 항목을 맨 위에 삽입 (사용자가 바로 볼 수 있도록)
  store.draft.snap.areas.unshift(target)
  // 다른 확장은 닫고 새 항목만 확장
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
    id: 'kind',
    header: () => t('snapEditor.type'),
    cell: ({ row }) =>
      h(UBadge, {
        label: row.original.kind === 'area' ? t('snapEditor.area') : t('snapEditor.action'),
        color: row.original.kind === 'area' ? 'primary' : 'info',
        variant: 'soft',
        size: 'sm',
      }),
  },
  {
    id: 'actions',
    header: '',
    cell: ({ row }) =>
      h(UDropdownMenu, {
        items: [
          {
            label: t('common.delete'),
            icon: 'i-lucide-trash-2',
            onSelect: () => deleteTarget(row.original.id),
          },
        ],
      }, () => h(UButton, { icon: 'i-lucide-more-horizontal', color: 'neutral', variant: 'ghost', size: 'xs' })),
  },
])
</script>

<template>
  <UDashboardPanel>
    <template #header>
      <UDashboardNavbar :title="t('snapEditor.title')">
        <template #right>
          <div class="flex items-center gap-2">
            <UDropdownMenu
              :items="[
                { label: t('snapEditor.area'), icon: 'i-lucide-square', onSelect: () => addTarget('area') },
                { label: t('snapEditor.action'), icon: 'i-lucide-zap', onSelect: () => addTarget('action') },
              ]"
            >
              <UButton icon="i-lucide-plus" color="primary" variant="soft" size="sm" :label="t('snapEditor.addTarget')" />
            </UDropdownMenu>
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
              :data="store.draft.snap.areas.filter(a => a.kind === 'area')"
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
</template>
