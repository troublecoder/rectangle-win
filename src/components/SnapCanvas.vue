<script setup lang="ts">
/**
 * vue-konva 단일 rect 캔버스 — SnapEditor 확장 행에서 사용되는 축소형.
 *
 * 16:9 모니터 모형 위에 snap 영역을 표시/편집.
 *  - 모니터 본체: 베젤 효과 (외곽선 + 약간 어두운 화면)
 *  - 비율 가이드라인: 1/2, 1/3 세로/가로 점선 (영역 정렬 보조)
 *  - snap 영역: primary 색 반투명 fill + 외곽선 + 중앙 라벨
 *  - semantic 색상 사용 (useCssVar로 resolve)
 */
import { computed } from 'vue'
import { useCssVar } from '@vueuse/core'
import type { SnapTargetArea } from '@/entities/config'

const props = defineProps<{
  area: SnapTargetArea | null
}>()

const emit = defineEmits<{
  update: [id: string, patch: Partial<SnapTargetArea>]
}>()

const CANVAS_W = 320
const CANVAS_H = Math.round(CANVAS_W * 9 / 16)

// konva는 CSS variable을 파싱하지 못하므로 useCssVar로 resolve.
// --ui-color-*-N 은 Nuxt UI가 rgb 채널 트리플("r g b")로 노출.
const primaryRaw = useCssVar('--ui-color-primary-500', undefined, { initialValue: '99 102 241' })
const neutralRaw = useCssVar('--ui-color-neutral-500', undefined, { initialValue: '115 115 115' })
const mutedRaw = useCssVar('--ui-color-neutral-700', undefined, { initialValue: '63 63 70' })
const elevatedRaw = useCssVar('--ui-color-neutral-900', undefined, { initialValue: '24 24 27' })
const invertedRaw = useCssVar('--ui-color-neutral-50', undefined, { initialValue: '250 250 250' })

const primaryChannel = computed(() => primaryRaw.value || '99 102 241')
const neutralChannel = computed(() => neutralRaw.value || '115 115 115')
const mutedChannel = computed(() => mutedRaw.value || '63 63 70')
const elevatedChannel = computed(() => elevatedRaw.value || '24 24 27')
const invertedChannel = computed(() => invertedRaw.value || '250 250 250')

// 모니터 — 베젤(외곽) + 화면(내부 약간 어두운 fill)
const monitorBg = computed(() => `rgb(${elevatedChannel.value})`)
const monitorStroke = computed(() => `rgb(${mutedChannel.value})`)
// snap 영역 색상
const areaStroke = computed(() => `rgb(${primaryChannel.value})`)
const areaFill = computed(() => `rgb(${primaryChannel.value} / 0.25)`)
// 가이드라인 (얇고 희미)
const guideStroke = computed(() => `rgb(${neutralChannel.value} / 0.35)`)
// 라벨 텍스트 (모니터 위에서 대비되는 색)
const labelFill = computed(() => `rgb(${invertedChannel.value})`)

// 비율 가이드라인 — 1/2, 1/3 세로/가로
const guideLines = computed(() => {
  const lines: { points: number[] }[] = []
  // 세로 1/2
  lines.push({ points: [CANVAS_W / 2, 0, CANVAS_W / 2, CANVAS_H] })
  // 세로 1/3
  lines.push({ points: [CANVAS_W / 3, 0, CANVAS_W / 3, CANVAS_H] })
  lines.push({ points: [(CANVAS_W * 2) / 3, 0, (CANVAS_W * 2) / 3, CANVAS_H] })
  // 가로 1/2
  lines.push({ points: [0, CANVAS_H / 2, CANVAS_W, CANVAS_H / 2] })
  return lines
})

const rectConfig = computed(() => {
  if (!props.area) return null
  return {
    x: props.area.x_ratio * CANVAS_W,
    y: props.area.y_ratio * CANVAS_H,
    width: props.area.w_ratio * CANVAS_W,
    height: props.area.h_ratio * CANVAS_H,
    fill: areaFill.value,
    stroke: areaStroke.value,
    strokeWidth: 2,
    cornerRadius: 3,
    draggable: true,
  }
})

const labelConfig = computed(() => {
  if (!props.area) return null
  return {
    x: (props.area.x_ratio + props.area.w_ratio / 2) * CANVAS_W,
    y: (props.area.y_ratio + props.area.h_ratio / 2) * CANVAS_H,
  }
})

const areaPct = computed(() => {
  if (!props.area) return ''
  const w = Math.round(props.area.w_ratio * 100)
  const h = Math.round(props.area.h_ratio * 100)
  return `${w} × ${h}%`
})

function onDragEnd(e: any) {
  if (!props.area) return
  const x = e.target.x() / CANVAS_W
  const y = e.target.y() / CANVAS_H
  emit('update', props.area.id, {
    x_ratio: Math.max(0, Math.min(1 - props.area.w_ratio, x)),
    y_ratio: Math.max(0, Math.min(1 - props.area.h_ratio, y)),
  })
}

function onTransformEnd(e: any) {
  if (!props.area) return
  const t = e.target
  const scaleX = t.scaleX()
  const scaleY = t.scaleY()
  t.scaleX(1)
  t.scaleY(1)
  emit('update', props.area.id, {
    x_ratio: Math.max(0, t.x() / CANVAS_W),
    y_ratio: Math.max(0, t.y() / CANVAS_H),
    w_ratio: Math.max(0.05, Math.min(1, (t.width() * scaleX) / CANVAS_W)),
    h_ratio: Math.max(0.05, Math.min(1, (t.height() * scaleY) / CANVAS_H)),
  })
}
</script>

<template>
  <div class="flex flex-col items-center gap-2">
    <div class="flex items-center justify-center rounded-md border border-default bg-elevated/40 p-3 shadow-inner">
      <v-stage :config="{ width: CANVAS_W, height: CANVAS_H }" class="rounded">
        <v-layer>
          <!-- 모니터 본체 -->
          <v-rect
            :config="{
              x: 0, y: 0,
              width: CANVAS_W, height: CANVAS_H,
              fill: monitorBg,
              stroke: monitorStroke,
              strokeWidth: 2,
              cornerRadius: 6,
            }"
          />
          <!-- 비율 가이드라인 -->
          <v-line
            v-for="(g, i) in guideLines"
            :key="i"
            :config="{
              points: g.points,
              stroke: guideStroke,
              strokeWidth: 1,
              dash: [3, 3],
              listening: false,
            }"
          />
          <!-- snap 영역 -->
          <v-rect
            v-if="rectConfig"
            :config="rectConfig"
            @dragend="onDragEnd"
            @transformend="onTransformEnd"
          />
          <!-- 영역 라벨 (중앙 비율 표시) -->
          <v-text
            v-if="rectConfig && labelConfig"
            :config="{
              ...labelConfig,
              text: areaPct,
              fontSize: 11,
              fontStyle: 'bold',
              fill: labelFill,
              align: 'center',
              verticalAlign: 'middle',
              offsetX: 24,
              offsetY: 7,
              listening: false,
            }"
          />
        </v-layer>
      </v-stage>
    </div>
    <p class="text-xs text-muted">드래그로 이동 · 모서리로 크기 조절</p>
  </div>
</template>
