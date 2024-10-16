<script lang="ts" setup>
import { ViewData, NodeType, Variant, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: boolean } & Partial<
    Pick<
      ViewData,
      "name" | "title" | "text" | "icon" | "nodePtr" | "variant" | "orientation" | "isInput" | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const inputRef = ref<HTMLInputElement | null>(null);

function toggle() {
  apply(!props.modelValue);
}
function apply(value: boolean) {
  emit("update:modelValue", value);
  emit("apply", value);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus: () => inputRef.value });
</script>
<template>
  <ViewContentWrapper :type="ViewType.TOGGLE" v-bind="props">
    <button
      v-if="variant == null || variant == Variant.PRIMARY"
      role="switch"
      :disabled="isDisabled || !isInput"
      class="relative inline-flex h-[20px] w-12 flex-shrink-0 cursor-pointer rounded border border-gray-200 transition-colors duration-75 ease-in-out focus:outline-none"
      :class="modelValue ? 'bg-gray-700' : 'bg-gray-100'"
      @click="toggle"
    >
      <span
        class="inline-block h-[18px] w-5 transform rounded-sm border border-gray-200 bg-white transition duration-75 ease-in-out"
        :class="modelValue ? 'translate-x-[26px]' : 'translate-x-0'"
      />
    </button>
    <!-- Checkbox -->
    <input
      v-else
      type="checkbox"
      class="h-4 w-4 rounded border-gray-200 text-gray-200 focus:ring-0"
      :checked="modelValue"
      @input="(e) => apply((e.target as HTMLInputElement).checked)"
    />
  </ViewContentWrapper>
</template>
