<script setup lang="ts">
/**
 * vue-konva 단일 rect 캔버스 — SnapEditor 확장 행에서 사용되는 축소형.
 * 320px 고정 폭, semantic 색상 사용 (useCssVar로 resolve).
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

// konva는 CSS variable을 파싱하지 못하므로 useCssVar로 resolve된 hex 값을 사용.
// --ui-color-primary-500 은 Nuxt UI가 rgb 채널 트리플("r g b")로 노출하므로
// rgb()로 감싸서 최종 색상 문자열 생성. alpha는 별도 조립.
// initialValue fallback을 두어 마운트 전/변수 미정의 시에도 유효한 색상이 되도록 함.
const primaryRaw = useCssVar('--ui-color-primary-500', undefined, { initialValue: '99 102 241' })
const neutralRaw = useCssVar('--ui-color-neutral-500', undefined, { initialValue: '115 115 115' })
const primaryChannel = computed(() => primaryRaw.value || '99 102 241')
const neutralChannel = computed(() => neutralRaw.value || '115 115 115')
const primaryColor = computed(() => `rgb(${primaryChannel.value})`)
const primaryFill = computed(() => `rgb(${primaryChannel.value} / 0.3)`)
const neutralStroke = computed(() => `rgb(${neutralChannel.value})`)

const rectConfig = computed(() => {
  if (!props.area) return null
  return {
    x: props.area.x_ratio * CANVAS_W,
    y: props.area.y_ratio * CANVAS_H,
    width: props.area.w_ratio * CANVAS_W,
    height: props.area.h_ratio * CANVAS_H,
    fill: primaryFill.value,
    stroke: primaryColor.value,
    strokeWidth: 2,
    cornerRadius: 2,
    draggable: true,
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
  <div class="flex items-center justify-center rounded-md border border-default bg-elevated/30 p-3">
    <v-stage :config="{ width: CANVAS_W, height: CANVAS_H }" class="rounded">
      <v-layer>
        <v-rect
          :config="{
            x: 0, y: 0,
            width: CANVAS_W, height: CANVAS_H,
            stroke: neutralStroke,
            strokeWidth: 1,
            cornerRadius: 4,
          }"
        />
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
