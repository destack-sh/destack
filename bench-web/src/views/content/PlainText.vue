<script lang="tsx" setup>
import { NodeReferenceData, type ViewData } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { makeViewId } from "@/views";
import { type ViewExposed, viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: NodeReferenceData } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "orientation" | "isInput" | "isDisabled" | "isSecret"
  >
>();
const emit = defineEmits(viewEmits());
const modelValue = defineModel<string>();


const self = toRef(props, "self");
defineExpose<ViewExposed>({ self, id: makeViewId(props) });
</script>
<template>
  <div v-if="!isInput">
    <label v-if="title" class="block text-gray-900">{{ title }}</label>
    <span>{{ modelValue }}</span>
  </div>
  <div v-else>
    <label v-if="title" class="block font-medium text-gray-900">{{ title }}</label>
    <div
      class="group mt-0.5 flex flex-row items-center rounded-md border border-gray-300 px-2 py-0.5 shadow-inset-sm shadow-gray-200 focus-within:border-primary-400 focus-within:ring-1 focus-within:ring-primary-400"
      :class="[isDisabled ? 'bg-gray-100 text-gray-700' : 'text-gray-900 bg-white']"
    >
      <IconInline v-if="icon" v-bind="icon" class="mr-2 text-gray-400" />
      <input
        :type="isSecret ? 'password' : 'text'"
        v-model="modelValue"
        class="w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :disabled="isDisabled"
      />
    </div>
  </div>
</template>
…