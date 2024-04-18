<script lang="ts" setup>
import { NodeReferenceData, NodeType, Variant, type ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline } from "@/system/icon";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { type ViewExposed, viewEmits } from "@/views/common";
import { ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Pick<
    ViewData,
    "title" | "text" | "icon" | "variant" | "valueType" | "isInput" | "isDisabled"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const modelValue = defineModel<string>();
const inputRef = ref<HTMLInputElement | null>(null);

const id = makeViewId(props);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH], focus: () => inputRef.value });
</script>
<template>
  <div>
    <label v-if="title" class="mb-0.5 block font-medium text-gray-900">{{ title }}</label>
    <div
      v-if="isInput"
      class="group flex flex-row items-center rounded-md border-gray-300 px-2 py-0.5 focus-within:border-primary-400"
      :class="[
        isDisabled ? 'bg-gray-100 text-gray-700' : 'bg-white text-gray-900',
        variant != Variant.STEALTH ? 'border focus-within:ring-1 focus-within:ring-primary-400' : '',
      ]"
    >
      <IconInline v-if="icon" v-bind="icon" class="mr-2 text-gray-400" />
      <input
        ref="inputRef"
        :type="valueType?.isSecret ? 'password' : 'text'"
        v-model="modelValue"
        class="w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :disabled="isDisabled"
      />
    </div>
    <span v-else>{{ modelValue }}</span>
  </div>
</template>
