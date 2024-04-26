<script lang="ts" setup>
import { ColorData, ColorShade, ColorType, NodeType, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { PopoverInfoIn } from "@/utils/menu";
import { REAL_COLORS, getColorHex, getColorTitle, makeColor } from "@/utils/style";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { ref, toRef, type Ref } from "vue";

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

function fire(item: ColorType | ColorData) {
  if (typeof item == "number") item = makeColor(item);
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
        (): PopoverInfoIn => ({
          component: ViewType.COLOR,
          placement: 'bottom-left',
          offset: 'referenceWidth',
          props,
          onApply: (value) => apply(value),
        })
      "
    >
      <template v-if="modelValue != null">
        <i class="fas fa-circle-small" :style="{ color: getColorHex(modelValue, ColorShade.S600) }" />
        <div class="ml-1.5">{{ getColorTitle(modelValue) ?? "???" }}</div>
      </template>
      <template v-else>
        <i class="fas fa-palette text-gray-400 group-hover:text-gray-700" />
        <span class="ml-1.5 text-gray-400 group-hover:text-gray-700">Select Color</span>
      </template>
      <i class="fas fa-caret-down ml-auto pl-1.5 text-gray-400" />
    </button>

    <!-- Inline Multi-Toggle -->
    <div v-else class="grid grid-cols-9 gap-x-0.5 gap-y-0.5 rounded border-gray-200 bg-white px-1 py-1">
      <button
        v-for="(color, i) in REAL_COLORS"
        :key="i"
        class="rounded border border-transparent px-1 py-0.5 hover:border-gray-300 hover:bg-gray-100"
        v-tooltip="{ title: getColorTitle(color), showDelay: 200, hideDelay: 100, small: true }"
        @click.stop.prevent="fire(color)"
        @keydown.enter.stop.prevent="fire(color)"
      >
        <i class="fas fa-circle" :style="{ color: getColorHex(color, ColorShade.S600) }" />
      </button>
    </div>
  </ViewContentWrapper>
</template>
