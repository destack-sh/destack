<script lang="ts" setup>
import { ViewData, NodeType, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; modelValue?: boolean } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "orientation" | "isInput" | "isDisabled" | "isMinimal">
  >
>();
const emit = defineEmits(viewEmits());
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
defineExpose<ViewExposed>({ self, id, focus: () => inputRef.value, interact: () => toggle() });
</script>
<template>
  <ViewContentWrapper :type="ViewType.TOGGLE" v-bind="props">
    <button
      v-if="!isMinimal"
      role="switch"
      :disabled="isDisabled || !isInput"
      class="relative inline-flex h-5 w-12 flex-shrink-0 cursor-pointer rounded border border-gray-200 transition-colors duration-75 ease-in-out focus:outline-none"
      :class="modelValue ? 'bg-gray-700' : 'bg-gray-100'"
      @click.stop="toggle"
    >
      <span
        class="inline-block h-[18px] w-5 transform rounded-sm border border-gray-200 bg-white transition duration-75 ease-in-out"
        :class="modelValue ? 'translate-x-[26px]' : 'translate-x-0'"
      />
    </button>
    <!-- Checkbox -->
    <button
      v-else
      class="h-5 w-5 rounded border border-gray-200 bg-white p-[1px] transition-colors duration-75"
      :disabled="isDisabled || !isInput"
      @click.stop="toggle"
    >
      <span
        class="inline-block h-full w-full rounded transition-colors duration-75"
        :class="modelValue ? 'bg-gray-700' : ''"
      />
    </button>
  </ViewContentWrapper>
</template>
