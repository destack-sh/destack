<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { SortOp, type Sort } from "@/gql/graphql";
import type { Field } from "@/state/module";
import { ArrowUpIcon } from "@heroicons/vue/24/outline";
import { TrashIcon } from "@heroicons/vue/24/outline";
import { ref } from "vue";

const props = defineProps<{
  field: Field;
  modelValue?: Sort;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value?: Sort): void;
}>();

function toggleOrder() {
  const value = props.modelValue;
  if (value == null) return;
  emit("update:modelValue", {
    ...value,
    order: value.order == SortOp.Ascending ? SortOp.Descending : SortOp.Ascending,
  });
}

// display

const editing = ref(false);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

function close() {
  editing.value = false;
}

function open() {
  editing.value = true;
}
</script>
<template>
  <!-- Sort pill -->
  <button
    class="flex flex-row items-center rounded-xl border border-amber-900/[12%] px-1.5 py-0.5 hover:bg-amber-100"
    @click="open"
  >
    <TypePreview :type="field" />
    <span class="ml-1 text-gray-900 underline decoration-gray-300 underline-offset-4">{{ field.name }}</span>
    <ArrowUpIcon
      class="ml-0.5 h-4 w-4 transform transition-transform"
      :class="modelValue?.order == SortOp.Ascending ? 'rotate-0' : 'rotate-180'"
    />
  </button>
  <!-- Prevent scroll and capture click outside -->
  <div v-if="editing" class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="close()" />
  <!-- Edit popover -->
  <FadeTransition>
    <div
      v-if="editing"
      ref="popoverRef"
      class="z-50 flex w-60 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      :class="[popoverPin.pinned.value ? '' : 'absolute left-0 top-7']"
      @keydown.escape.exact.prevent.stop="close()"
    >
      <div class="flex flex-row items-center">
        <!-- Field (can't change here) -->
        <span
          class="border border-orange-900/[12%] px-1.5 py-0.5 font-bold underline decoration-gray-300 underline-offset-4"
          >{{ field.name }}</span
        >
        <!-- Sort order -->
        <button
          class="ml-2 flex flex-row items-center rounded-sm border border-orange-900/[12%] px-1 py-0.5 text-gray-900 hover:bg-amber-100"
          @click="toggleOrder()"
        >
          {{ modelValue?.order == SortOp.Ascending ? "Ascending" : "Descending" }}
          <ArrowUpIcon
            class="ml-0.5 h-4 w-4 transform transition-transform"
            :class="modelValue?.order == SortOp.Ascending ? 'rotate-0' : 'rotate-180'"
          />
        </button>
        <!-- Delete -->
        <button class="ml-auto text-gray-700 hover:bg-orange-100" @click="emit('update:modelValue', undefined)">
          <TrashIcon class="h-4 w-4" />
        </button>
      </div>
    </div>
  </FadeTransition>
</template>
