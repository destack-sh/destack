<script lang="ts" setup>
import { provideViewSectionGroup } from "@/components/views/sections";
import { useFocusWithin } from "@vueuse/core";
import { ref, watch, type Ref } from "vue";

const props = defineProps<{ focused: boolean; focusSection?: number }>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();
const containerRef: Ref<HTMLDivElement | null> = ref(null);
const api = provideViewSectionGroup(containerRef);

// handle focus
const { focused: inContainerFocused } = useFocusWithin(containerRef);

// focus view when getting focus
watch(inContainerFocused, () => {
  if (inContainerFocused.value) {
    emit("show");
  } else {
    emit("blur");
  }
});
// handle explorer view focus and editor focus
watch(
  () => props.focused,
  (focused, wasFocused) => {
    if (!wasFocused && focused) {
      if (!inContainerFocused.value) {
        // start to focus focus section if nothing was directly focused
        api.focus(props.focusSection ?? 0);
      }
    } else if (wasFocused) {
      api.blur();
    }
  },
  { immediate: true }
);

defineExpose({ api });
</script>
<template>
  <div ref="containerRef" class="relative flex h-full flex-col" :style="{ gap: api.gapY + 'px' }">
    <slot />
  </div>
</template>
