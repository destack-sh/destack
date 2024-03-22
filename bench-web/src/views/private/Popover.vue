<script lang="tsx" setup>
/* eslint-disable vue/no-multiple-template-root */
import { useFloating, type FloatingOptions } from "@/utils/floating";
import { ref } from "vue";

const props = defineProps<FloatingOptions & {}>();
const triggerRef = ref<HTMLElement | null>(null);
const contentRef = ref<HTMLElement | null>(null);

const isOpen = ref(false);
function open() {
  isOpen.value = true;
}
function close() {
  isOpen.value = false;
}
function toggle() {
  isOpen.value = !isOpen.value;
}

// float the content element as specified
useFloating({
  floating: contentRef,
  reference: triggerRef,
  enabled: isOpen,
  options: props,
});
</script>
<template>
  <!-- Popover -->
  <!-- Trigger -->
  <div ref="triggerRef">
    <slot name="trigger" :isOpen="isOpen" :open="open" :close="close" :toggle="toggle" />
  </div>

  <Transition
    enter-active-class="transition-all ease-in duration-75"
    enter-from-class="opacity-0 scale-95"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-95"
  >
    <!-- Content -->
    <div class="absolute z-50" ref="contentRef" v-if="isOpen">
      <slot name="content" :close="close" />
    </div>
  </Transition>
</template>
