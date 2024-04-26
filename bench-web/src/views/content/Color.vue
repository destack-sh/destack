<script lang="ts" setup>
import { ColorData, ColorShade, NodeType, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { OverlayMenuInfoIn } from "@/utils/menu";
import { getColorHex, getColorTitle } from "@/utils/style";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { ref, toRef, type Ref } from "vue";

const DEFAULT_WIDTH = 380;
const MAX_HEIGHT = 280;
const ITEMS_PER_ROW = 10;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: ColorData } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);

function fire(item: ColorData) {
  apply(item);
}
function apply(color: ColorData) {
  emit("update:modelValue", color);
  emit("apply", color);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- TODO :Incomplete: Color.isInput/isDisabled/variants/... -->
    <!-- Dropdown -->
    <button
      ref="buttonRef"
      v-if="!isInline"
      class="group flex w-full flex-row items-center rounded border border-gray-200 px-2 py-1 hover:border-gray-300 data-[menu=true]:border-gray-300"
      v-menu="
        (): OverlayMenuInfoIn => ({
          component: ViewType.COLOR,
          placement: 'bottom-left',
          offset: 'referenceWidth',
          props,
          onApply: (value) => apply(value),
        })
      "
    >
      <template v-if="modelValue != null">
        <i class="fas fa-circle-small" :style="{ color: getColorHex(modelValue, ColorShade.S500) }" />
        <div class="ml-1.5">{{ getColorTitle(modelValue) ?? "???" }}</div>
      </template>
      <template v-else>
        <i class="fas fa-palette text-gray-400 group-hover:text-gray-700" />
        <span class="ml-1.5 text-gray-400 group-hover:text-gray-700">Select Color</span>
      </template>
      <i class="fas fa-caret-down ml-auto pl-1.5 text-gray-400" />
    </button>

    <!-- Inline Multi-Toggle -->
    <div v-else>nocheckin: inline multi-toggle for Color!</div>
  </ViewContentWrapper>
</template>
