<script lang="tsx" setup>
import { NodeReferenceData, type ViewData } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: NodeReferenceData | null } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "isInput" | "isDisabled" | "isSecret"
  >
>();
const emit = defineEmits(viewEmits());
const modelValue = defineModel<string>();

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div v-if="!isInput">
    <label v-if="title" class="block text-gray-900">{{ title }}</label>
    <span>{{ modelValue }}</span>
  </div>
  <div v-else>
    <label v-if="title" class="block font-medium text-gray-900">{{ title }}</label>
    <div
      class="group mt-0.5 flex flex-row items-center shadow-sm shadow-gray-300 rounded-md border border-gray-300 bg-white px-2 py-0.5 text-gray-900 focus-within:border-primary-400 focus-within:ring-primary-400 focus-within:ring-1"
    >
      <IconInline v-if="icon" v-bind="icon" class="mr-2 text-gray-400" />
      <input
        :type="isSecret ? 'password' : 'text'"
        v-model="modelValue"
        class="w-full border-0 p-0 ring-0 focus:ring-0 outline-none"
      />
    </div>
  </div>
</template>
