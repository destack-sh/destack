<script lang="ts" setup>
/* eslint-disable vue/no-multiple-template-root */
import { useFloating, type FloatingOptions, type FloatingPlacement } from "@/utils/floating";
import { ref } from "vue";

const props = defineProps<FloatingOptions & {}>();
const triggerRef = ref<HTMLElement | null>(null);
const contentRef = ref<HTMLElement | null>(null);

function getEnterFrom(placement: FloatingPlacement): string {
  if (placement.startsWith("left")) return "translate-x-[6px]";
  else if (placement.startsWith("top")) return "translate-y-[6px]";
  else if (placement.startsWith("right")) return "translate-x-[-6px]";
  /* bottom */ else return "translate-y-[-6px]";
}

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
  isEnabled: isOpen,
  options: props,
});

defineExpose({
  open,
  close,
  toggle,
});
</script>
<template>
  <!-- Popover -->
  <!-- Trigger -->
  <div ref="triggerRef">
    <slot name="trigger" :is-open="isOpen" :open="open" :close="close" :toggle="toggle" />
  </div>

  <Transition
    enter-active-class="transition-all ease-in duration-75"
    :enter-from-class="'opacity-0 ' + getEnterFrom(props.placement)"
    enter-to-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100 translate-x-0 translate-y-0"
    :leave-to-class="'opacity-0 ' + getEnterFrom(props.placement)"
  >
    <!-- Content -->
    <div v-if="isOpen" ref="contentRef" class="absolute z-50">
      <slot name="content" :close="close" />
    </div>
  </Transition>
</template>
