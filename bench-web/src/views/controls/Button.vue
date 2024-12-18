<script lang="ts" setup>
import { ButtonVariant, ColorShade, NodeType, ViewData, ViewType } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline } from "@/ui/icon";
import { canvas } from "@/system/space";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";
import { useSubnodeProperty } from "@/language/node";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "isDisabled" | "isLoading" | "subnodePacked">
  >
>();
const subnodePacked = toRef(props, "subnodePacked");
const variant = useSubnodeProperty(NodeType.VIEW, ViewType.BUTTON, subnodePacked, "variant");
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);

const classByVariant: Ref<Partial<Record<ButtonVariant, string[]>>> = computed(() => ({
  // prominent filled button
  [ButtonVariant.PRIMARY]: [
    "rounded border border-gray-200",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled
      ? "text-gray-400 bg-gray-200 hover:cursor-not-allowed"
      : "text-gray-900 bg-white hover:border-gray-400 hover:bg-gray-100",
  ],
  // outline button
  [ButtonVariant.SECONDARY]: [
    "rounded",
    props.title ? "px-2 py-1" : "px-1 py-0.5",
    props.isDisabled ? "text-gray-400 bg-gray-50 hover:cursor-not-allowed" : "text-gray-700 bg-white hover:bg-gray-100",
  ],
  // 'link' button
  [ButtonVariant.LINK]: [
    "rounded",
    props.isDisabled ? "text-gray-400 hover:cursor-not-allowed" : "text-gray-700 hover:text-gray-900",
  ],
}));

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus: () => buttonRef.value });
</script>
<template>
  <button
    ref="buttonRef"
    :class="[classByVariant[variant!] ?? classByVariant[ButtonVariant.PRIMARY]]"
    :disabled="isDisabled"
  >
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
