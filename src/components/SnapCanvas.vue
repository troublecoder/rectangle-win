<script setup lang="ts">
/**
 * vue-konva 모니터 도화감 — snap 영역을 시각적으로 표시하고 드래그/리사이즈.
 *
 * 좌표계: 모니터 bounds (예: 1920×1080) → 캔버스 픽셀로 스케일.
 * SnapTarget.Area의 ratio(0~1) 기반이므로 캔버스 크기에 비례 렌더링.
 */
import { computed, ref } from 'vue'
import type { SnapTargetArea } from '@/entities/config'

const props = defineProps<{
  areas: SnapTargetArea[]
  selectedId: string | null
  /** 캔버스 가로 픽셀 (16:9 비율 가정) */
  width?: number
}>()

const emit = defineEmits<{
  select: [id: string]
  update: [id: string, patch: Partial<SnapTargetArea>]
}>()

const CANVAS_W = computed(() => props.width ?? 480)
const CANVAS_H = computed(() => Math.round(CANVAS_W.value * 9 / 16))

function areaRect(area: SnapTargetArea) {
  return {
    x: area.x_ratio * CANVAS_W.value,
    y: area.y_ratio * CANVAS_H.value,
    width: area.w_ratio * CANVAS_W.value,
    height: area.h_ratio * CANVAS_H.value,
  }
}

const dragAreaId = ref<string | null>(null)

function onDragStart(id: string) {
  dragAreaId.value = id
  emit('select', id)
}

function onDragEnd(id: string, area: SnapTargetArea, e: { target: { x: () => number; y: () => number } }) {
  const x = e.target.x()
  const y = e.target.y()
  emit('update', id, {
    x_ratio: Math.max(0, Math.min(1 - area.w_ratio, x / CANVAS_W.value)),
    y_ratio: Math.max(0, Math.min(1 - area.h_ratio, y / CANVAS_H.value)),
  })
  dragAreaId.value = null
}

function onResize(id: string, area: SnapTargetArea, e: { target: { width: () => number; height: () => number } }) {
  emit('update', id, {
    w_ratio: Math.max(0.05, Math.min(1 - area.x_ratio, e.target.width() / CANVAS_W.value)),
    h_ratio: Math.max(0.05, Math.min(1 - area.y_ratio, e.target.height() / CANVAS_H.value)),
  })
}

const colorFor = (area: SnapTargetArea) =>
  area.id === props.selectedId ? '#cba6f7' : '#89b4fa80'
</script>

<template>
  <div class="flex items-center justify-center rounded-lg border border-default bg-inverted/5 p-4">
    <v-stage :config="{ width: CANVAS_W, height: CANVAS_H }" class="rounded bg-inverted">
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

        <!-- 각 Snap 영역 -->
        <template v-for="area in areas" :key="area.id">
          <v-rect
            :config="{
              ...areaRect(area),
              fill: colorFor(area),
              stroke: area.id === selectedId ? '#cba6f7' : '#89b4fa',
              strokeWidth: area.id === selectedId ? 2 : 1,
              cornerRadius: 2,
              draggable: true,
            }"
            @dragstart="onDragStart(area.id)"
            @dragend="onDragEnd(area.id, area, $event)"
            @transformend="onResize(area.id, area, $event)"
            @click="emit('select', area.id)"
            @tap="emit('select', area.id)"
          />
          <!-- 라벨 -->
          <v-text
            :config="{
              x: areaRect(area).x + 4,
              y: areaRect(area).y + 4,
              text: area.name,
              fontSize: 11,
              fill: area.id === selectedId ? '#cba6f7' : '#cdd6f4',
            }"
            @click="emit('select', area.id)"
          />
        </template>
      </v-layer>
    </v-stage>
  </div>
</template>
