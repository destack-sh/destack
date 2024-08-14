<script lang="ts" setup>
import { ColorShade, NodeType, Variant, ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline } from "@/ui/icon";
import { canvas } from "@/system/space";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "variant" | "isDisabled" | "isLoading">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);

const classByVariant: Ref<Partial<Record<Variant, string[]>>> = computed(() => ({
  // prominent filled button
  [Variant.PRIMARY]: [
    "rounded border border-gray-900",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled
      ? "text-gray-600 bg-gray-200 hover:cursor-not-allowed"
      : "text-gray-900 bg-primary-300 hover:border-primary-900 hover:text-primary-900 hover:outline outline-1 outline-primary-900",
  ],
  // outline button
  [Variant.SECONDARY]: [
    "rounded border border-gray-200 bg-white",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled
      ? "text-gray-700 bg-gray-50 hover:cursor-not-allowed"
      : "text-gray-900 hover:border-gray-300 hover:bg-gray-100 focus:border-primary-900",
  ],
  // 'link' button
  [Variant.COMPACT]: [
    "rounded underline decoration-2 underline-offset-3",
    props.isDisabled
      ? "text-gray-500 decoration-gray-200 hover:cursor-not-allowed"
      : "text-gray-900 decoration-gray-300 hover:text-primary-900 hover:decoration-primary-900 focus:decoration-primary-900",
  ],
  // 'stealth' button
  [Variant.STEALTH]: [
    "rounded",
    props.isDisabled
      ? "text-gray-500 hover:cursor-not-allowed"
      : "text-gray-900 hover:text-primary-900 focus:underline decoration-2 underline-offset-3 focus:decoration-primary-900",
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
    <IconInline
      v-else-if="icon"
      v-bind="icon"
      class="w-5 text-center no-underline"
      :shade="ColorShade.S900"
      :class="title ? 'mr-1.5' : ''"
    />
    <span v-if="title" class="select-none font-semibold">{{ title }}</span>
  </button>
</template>
