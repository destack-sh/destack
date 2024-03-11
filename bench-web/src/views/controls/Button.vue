<script lang="tsx" setup>
import { NodeReferenceData, ViewData, Variant } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { viewEmits } from "@/views/common";
import { computed, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: NodeReferenceData | null } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "variant" | "isDisabled" | "isLoading"
  >
>();
const emit = defineEmits(viewEmits());

const classByVariant: Ref<Partial<Record<Variant, string[]>>> = computed(() => ({
  // prominent filled button
  [Variant.V1]: [
    "rounded-md border border-gray-900 px-2 py-1 shadow-sm shadow-gray-900",
    props.isDisabled
      ? "text-gray-600 bg-primary-200 hover:cursor-not-allowed"
      : "text-gray-900 bg-primary-300 hover:bg-primary-400 focus:shadow-primary-600",
  ],
  // outline button
  [Variant.V2]: [
    "rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300",
    props.isDisabled
      ? "text-gray-700 bg-gray-50 hover:cursor-not-allowed"
      : "text-gray-900 hover:border-gray-400 hover:bg-gray-100 focus:shadow-primary-600",
  ],
  // 'link' button
  [Variant.V3]: [
    "rounded-md underline decoration-2 underline-offset-4",
    props.isDisabled
      ? "text-gray-500 decoration-gray-200 hover:cursor-not-allowed"
      : "text-gray-900 decoration-gray-300 hover:text-primary-900 hover:decoration-primary-400",
  ],
  // 'stealth' button
  [Variant.V4]: [
    "rounded-md",
    props.isDisabled
      ? "text-gray-500 hover:cursor-not-allowed"
      : "text-gray-900 hover:text-primary-900 hover:decoration-primary-400"
  ],
}));

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <button
    :class="[classByVariant[variant ?? Variant.V1] ?? classByVariant[Variant.V1]]"
    :disabled="isDisabled"
  >
    <slot>
      <IconInline v-if="icon" v-bind="icon" class="mr-2 no-underline" />
      <span class="font-semibold">{{ title }}</span>
      <i v-if="isLoading" class="fas fa-spin fa-spinner-third ml-2 no-underline" />
    </slot>
  </button>
</template>
