<script lang="tsx" setup>
import { ViewVariant } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { type ViewEmits, type ViewPropsAll } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<Pick<ViewPropsAll, "self" | "name" | "title" | "text" | "icon" | "variant"> & {}>();
const emits = defineEmits<ViewEmits & {}>();

const styleByVariant: Partial<Record<ViewVariant, string>> = {
  [ViewVariant.PRIMARY]:
    "bg-primary-300 border rounded-md border-gray-900 px-2 py-1 text-gray-900 shadow-sm shadow-gray-900 hover:bg-primary-400",
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