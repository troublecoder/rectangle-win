<script setup lang="ts">
/**
 * vue-konva 단일 rect 캔버스 — SnapEditor 확장 행에서 사용되는 축소형.
 * 16:9 모니터 모형 위에 snap 영역을 표시/편집.
 * 고정 hex 색상 사용 — Nuxt UI CSS 변수는 tree-shaking으로 누락될 수 있음.
 */
import { computed } from 'vue'
import type { SnapTargetArea } from '@/entities/config'

const props = defineProps<{
  area: SnapTargetArea | null
}>()

const emit = defineEmits<{
  update: [id: string, patch: Partial<SnapTargetArea>]
}>()

const CANVAS_W = 320
const CANVAS_H = Math.round(CANVAS_W * 9 / 16)

// 고정 색상 — 테마 무관하게 항상 보이도록.
const COLOR_MONITOR = '#1e293b'
const COLOR_MONITOR_STROKE = '#475569'
const COLOR_AREA_FILL = 'rgba(59, 130, 246, 0.25)'
const COLOR_AREA_STROKE = '#3b82f6'
const COLOR_GUIDE = 'rgba(148, 163, 184, 0.3)'
const COLOR_LABEL = '#f1f5f9'

const guideLines = computed(() => {
  const lines: { points: number[] }[] = []
  lines.push({ points: [CANVAS_W / 2, 0, CANVAS_W / 2, CANVAS_H] })
  lines.push({ points: [CANVAS_W / 3, 0, CANVAS_W / 3, CANVAS_H] })
  lines.push({ points: [(CANVAS_W * 2) / 3, 0, (CANVAS_W * 2) / 3, CANVAS_H] })
  lines.push({ points: [0, CANVAS_H / 2, CANVAS_W, CANVAS_H / 2] })
  return lines
})

const monitorConfig = computed(() => ({
  x: 0, y: 0,
  width: CANVAS_W, height: CANVAS_H,
  fill: COLOR_MONITOR,
  stroke: COLOR_MONITOR_STROKE,
  strokeWidth: 2,
  cornerRadius: 6,
}))

const guideLineConfig = (points: number[]) => ({
  points,
  stroke: COLOR_GUIDE,
  strokeWidth: 1,
  dash: [3, 3],
  listening: false,
})

const rectConfig = computed(() => {
  if (!props.area) return null
  return {
    x: props.area.x_ratio * CANVAS_W,
    y: props.area.y_ratio * CANVAS_H,
    width: props.area.w_ratio * CANVAS_W,
    height: props.area.h_ratio * CANVAS_H,
    fill: COLOR_AREA_FILL,
    stroke: COLOR_AREA_STROKE,
    strokeWidth: 2,
    cornerRadius: 3,
    draggable: true,
  }
})

const labelConfigFull = computed(() => {
  if (!props.area) return null
  const w = Math.round(props.area.w_ratio * 100)
  const h = Math.round(props.area.h_ratio * 100)
  return {
    x: (props.area.x_ratio + props.area.w_ratio / 2) * CANVAS_W,
    y: (props.area.y_ratio + props.area.h_ratio / 2) * CANVAS_H,
    text: `${w} × ${h}%`,
    fontSize: 11,
    fontStyle: 'bold',
    fill: COLOR_LABEL,
    align: 'center',
    verticalAlign: 'middle',
    offsetX: 24,
    offsetY: 7,
    listening: false,
  }
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
          <v-rect :config="monitorConfig" />
          <v-line
            v-for="(g, i) in guideLines"
            :key="i"
            :config="guideLineConfig(g.points)"
          />
          <v-rect
            v-if="rectConfig"
            :config="rectConfig"
            @dragend="onDragEnd"
            @transformend="onTransformEnd"
          />
          <v-text
            v-if="labelConfigFull"
            :config="labelConfigFull"
          />
        </v-layer>
      </v-stage>
    </div>
    <p class="text-xs text-muted">드래그로 이동 · 모서리로 크기 조절</p>
  </div>
</template>
