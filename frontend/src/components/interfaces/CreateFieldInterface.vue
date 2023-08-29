<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectTypeInterface from "@/components/interfaces/SelectTypeInterface.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import type { Field } from "@/gql/graphql";
import { ref } from "vue";

defineProps<{ title: string }>();

const emit = defineEmits<{
  (e: "select", type: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "metadata">): void;
}>();

const open = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

function show() {
  open.value = true;
}

function hide() {
  open.value = false;
}

defineExpose({
  show,
  hide,
});
</script>
<template>
  <div
    v-if="open"
    class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
    @keydown.escape="hide()"
    @click.stop="hide()"
  />
  <FadeTransition>
    <div
      v-if="open"
      @keydown.escape="hide()"
      ref="popoverRef"
      class="z-50 flex w-80 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="popoverPin.pinned.value ? '' : 'absolute top-8'"
    >
      <h5 class="px-1 text-left text-xs font-semibold text-gray-500">{{ title }}</h5>
      <SelectTypeInterface class="mt-2" hide-flags @update:model-value="hide(), emit('select', $event)" />
    </div>
  </FadeTransition>
</template>
