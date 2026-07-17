<script setup lang="ts">
/**
 * vue-konva 단일 rect 캔버스 — 선택된 snap 영역 하나만 표시/편집.
 * 영역 목록에서 선택하면 이 캔버스의 rect가 해당 영역의 위치/크기로 갱신됨.
 * 드래그/리사이즈로 영역 속성을 직접 편집.
 */
import { computed } from 'vue'
import type { SnapTargetArea } from '@/entities/config'

const props = defineProps<{
  area: SnapTargetArea | null
  selectedId: string | null
  width?: number
}>()

const emit = defineEmits<{
  update: [id: string, patch: Partial<SnapTargetArea>]
}>()

const CANVAS_W = computed(() => props.width ?? 480)
const CANVAS_H = computed(() => Math.round(CANVAS_W.value * 9 / 16))

const rectConfig = computed(() => {
  if (!props.area) return null
  return {
    x: props.area.x_ratio * CANVAS_W.value,
    y: props.area.y_ratio * CANVAS_H.value,
    width: props.area.w_ratio * CANVAS_W.value,
    height: props.area.h_ratio * CANVAS_H.value,
    fill: '#89b4fa',
    stroke: '#cba6f7',
    strokeWidth: 2,
    cornerRadius: 2,
    draggable: true,
  }
})

function onDragEnd(e: any) {
  if (!props.area) return
  const x = e.target.x() / CANVAS_W.value
  const y = e.target.y() / CANVAS_H.value
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
    x_ratio: Math.max(0, t.x() / CANVAS_W.value),
    y_ratio: Math.max(0, t.y() / CANVAS_H.value),
    w_ratio: Math.max(0.05, Math.min(1, (t.width() * scaleX) / CANVAS_W.value)),
    h_ratio: Math.max(0.05, Math.min(1, (t.height() * scaleY) / CANVAS_H.value)),
  })
}
</script>

<template>
  <div class="flex items-center justify-center rounded-lg border border-default bg-inverted/5 p-4">
    <v-stage :config="{ width: CANVAS_W, height: CANVAS_H }" class="rounded" style="background: #181825;">
      <v-layer>
        <!-- 모니터 프레임 -->
        <v-rect
          :config="{
            x: 0, y: 0,
            width: CANVAS_W, height: CANVAS_H,
            stroke: '#6c7086', strokeWidth: 2,
            cornerRadius: 4,
          }"
        />
        <!-- 선택된 영역 (단일 rect 재사용) -->
        <v-rect
          v-if="rectConfig"
          :config="rectConfig"
          @dragend="onDragEnd"
          @transformend="onTransformEnd"
        />
      </v-layer>
    </v-stage>
  </div>
</template>
