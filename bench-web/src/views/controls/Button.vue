<script lang="tsx" setup>
import { NodeReferenceData, ViewData, ViewVariant } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: NodeReferenceData | null } & Pick<ViewData, "name" | "title" | "text" | "icon" | "variant">
>();
const emit = defineEmits(viewEmits());

const styleByVariant: Partial<Record<ViewVariant, string>> = {
  // prominent filled button
  [ViewVariant.PRIMARY]:
    "bg-primary-300 border rounded-md border-gray-900 px-2 py-1 text-gray-900 shadow-sm shadow-gray-900 hover:bg-primary-400",
  // outline button
  [ViewVariant.SECONDARY]:
    "border border-gray-900 bg-white shadow-sm shadow-gray-900 rounded-md px-2 py-1 text-gray-900 hover:bg-gray-100",
  // 'link' button
  [ViewVariant.TERTIARY]:
    "text-gray-900 underline decoration-2 underline-offset-4 decoration-gray-400 hover:decoration-primary-400 hover:text-gray-700",
};

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <button :class="styleByVariant[variant ?? ViewVariant.PRIMARY] ?? styleByVariant[ViewVariant.PRIMARY]">
    <slot>
      <IconInline v-if="icon" v-bind="icon" class="mr-2" />
      <span class="font-semibold">{{ title }}</span>
    </slot>
  </button>
</template>
