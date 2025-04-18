<script lang="ts" setup>
import { activeSelection, SelectionZone } from "@/ui/drag";

const props = defineProps<{ zone: SelectionZone }>();
</script>
<template>
  <div
    v-if="activeSelection?.activeZone?.id == zone.id && activeSelection.activeOverlay != null"
    class="pointer-events-none absolute z-30 rounded-xs bg-amber-400/20"
    :style="{
      left: `${activeSelection.activeOverlay.x}px`,
      top: `${activeSelection.activeOverlay.y}px`,
      width: `${activeSelection.activeOverlay.width}px`,
      height: `${activeSelection.activeOverlay.height}px`,
    }"
  >
    <!-- Selection area -->
  </div>
  <div v-else />
</template>
