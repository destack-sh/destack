<script lang="tsx" setup>
import { NodeReferenceData, Variant, ViewData } from "@/proto/wire";
import { IconInline } from "@/system/icon";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: NodeReferenceData } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "variant" | "isDisabled" | "isLoading"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);

const classByVariant: Ref<Partial<Record<Variant, string[]>>> = computed(() => ({
  // prominent filled button
  [Variant.PRIMARY]: [
    "rounded-md border border-gray-900 shadow-sm shadow-gray-900",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled
      ? "text-gray-600 bg-primary-200 hover:cursor-not-allowed"
      : "text-gray-900 bg-primary-300 hover:bg-primary-400 focus:shadow-primary-600",
  ],
  // outline button
  [Variant.SECONDARY]: [
    "rounded-md border border-gray-300 bg-white shadow-sm shadow-gray-300",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled
      ? "text-gray-700 bg-gray-50 hover:cursor-not-allowed"
      : "text-gray-900 hover:border-gray-400 hover:bg-gray-100 focus:shadow-primary-600",
  ],
  // 'link' button
  [Variant.COMPACT]: [
    "rounded-md underline decoration-2 underline-offset-4",
    props.isDisabled
      ? "text-gray-500 decoration-gray-200 hover:cursor-not-allowed"
      : "text-gray-900 decoration-gray-300 hover:text-primary-900 hover:decoration-primary-400",
  ],
  // 'stealth' button
  [Variant.STEALTH]: [
    "rounded-md",
    props.isDisabled ? "text-gray-500 hover:cursor-not-allowed" : "text-gray-900 hover:text-primary-900",
  ],
}));

canvas.registerView(self, id);
defineExpose<ViewExposed>({
  self,
  id,
  variants: [Variant.PRIMARY, Variant.SECONDARY, Variant.COMPACT, Variant.STEALTH],
  focus: () => buttonRef.value,
});
</script>
<template>
  <button ref="buttonRef" :class="[classByVariant[variant!] ?? classByVariant[Variant.PRIMARY]]" :disabled="isDisabled">
    <i v-if="isLoading" class="fas fa-spinner-third mr-2 animate-spin no-underline" />
    <IconInline v-else-if="icon" v-bind="icon" class="no-underline" :class="title ? 'mr-2' : ''" />
    <span v-if="title" class="select-none font-semibold">{{ title }}</span>
  </button>
</template>
