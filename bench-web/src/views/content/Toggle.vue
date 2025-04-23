<script lang="ts" setup>
import { NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; modelValue?: boolean } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "orientation" | "isInput" | "isDisabled" | "isMinimal">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const inputRef = ref<HTMLInputElement | null>(null);

function toggle() {
  apply(!props.modelValue);
}
function apply(value: boolean) {
  emit("update:modelValue", value);
  emit("apply", value);
}

canvas.registerView(self, id);
defineExpose<ViewExpose>({ self, id, focus: () => inputRef.value, interact: () => toggle() });
</script>
<template>
  <div class="flex flex-row items-center justify-end" @mousedown.stop="!isDisabled && toggle()">
    <!-- Toggle -->
    <button
      v-if="!isMinimal"
      role="switch"
      :disabled="isDisabled || !isInput"
      class="relative inline-flex h-5 w-12 shrink-0 cursor-pointer rounded-sm border transition-colors duration-75 ease-in-out focus:outline-hidden"
      :class="modelValue ? 'border-gray-700 bg-gray-700' : 'border-gray-200 bg-gray-100'"
    >
      <span
        class="inline-block h-[18px] w-5 transform rounded-xs border border-gray-200 bg-white transition duration-75 ease-in-out"
        :class="modelValue ? 'translate-x-[26px]' : 'translate-x-0'"
      />
    </button>
    <!-- Checkbox -->
    <button
      v-else
      class="flex h-5 w-5 flex-row items-center justify-center rounded-sm border bg-white px-[1px] py-[1px] transition-colors duration-75"
      :class="modelValue ? 'border-gray-700' : 'border-gray-200'"
      :disabled="isDisabled || !isInput"
    >
      <span
        class="inline-block h-full w-full rounded-sm transition-colors duration-75"
        :class="modelValue ? 'bg-gray-700' : ''"
      />
    </button>
  </div>
</template>
