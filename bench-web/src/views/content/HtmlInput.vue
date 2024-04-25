<script lang="ts" setup>
import { ColorShade, ColorType, NodeType, Variant, ViewType, type ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline } from "@/system/icon";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<
      ViewData,
      "type" | "name" | "title" | "text" | "icon" | "variant" | "valueType" | "orientation" | "isInput" | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const modelValue = defineModel<string | number>();
const inputRef = ref<HTMLInputElement | null>(null);
const inputType = computed(() => {
  if (props.valueType?.isSecret) return "password";
  else if (props.type == ViewType.NUMBER) return "number";
  else return "text";
});

const id = makeViewId(props);
canvas.registerView(self, id);
defineExpose<ViewExposed>({
  self,
  id,
  variants: [Variant.PRIMARY, Variant.SECONDARY, Variant.STEALTH],
  focus: () => inputRef.value,
});
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <div
      v-if="isInput"
      class="group flex flex-1 flex-row items-center rounded outline-1 outline-primary-900 focus-within:outline-dotted hover:border-gray-300"
      :class="[
        isDisabled ? 'bg-gray-100 text-gray-700' : 'bg-white text-gray-900',
        variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1' : '',
      ]"
    >
      <IconInline v-if="icon" v-bind="icon" :shade="ColorShade.S400" class="mr-1.5 w-5" />
      <input
        ref="inputRef"
        :type="inputType"
        v-model="modelValue"
        class="w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :disabled="isDisabled"
      />
    </div>
    <span v-else>{{ modelValue }}</span>
  </ViewContentWrapper>
</template>
