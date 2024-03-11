<script lang="tsx" setup>
import { NodeReferenceData, ViewData, Variant } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: NodeReferenceData | null } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "variant" | "isDisabled" | "isLoading"
  >
>();
const emit = defineEmits(viewEmits());

const styleByVariant: Partial<Record<Variant, string>> = {
  // prominent filled button
  [Variant.V1]:
    "bg-primary-300 border rounded-md border-gray-900 px-2 py-1 text-gray-900 shadow-sm shadow-gray-900 hover:bg-primary-400",
  // outline button
  [Variant.V2]:
    "border border-gray-300 bg-white shadow-sm shadow-gray-300 hover:border-gray-400  rounded-md px-2 py-1 text-gray-900 hover:bg-gray-100",
  // 'link' button
  [Variant.V3]:
    "text-gray-900 underline decoration-2 decoration-gray-400 hover:decoration-primary-400 hover:text-primary-800",
  // 'stealth' button
  [Variant.V4]: "text-gray-00 decoration-gray-400 hover:decoration-primary-400 hover:text-primary-800",
};

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <button :class="styleByVariant[variant ?? Variant.V1] ?? styleByVariant[Variant.V1]">
    <slot>
      <IconInline v-if="icon" v-bind="icon" class="mr-2" />
      <span class="font-semibold">{{ title }}</span>
    </slot>
  </button>
</template>
