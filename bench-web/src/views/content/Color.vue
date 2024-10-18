<script lang="ts" setup>
import { ColorData, ColorShade, ColorType, NodeReferenceData, NodeType, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { PopoverInfoIn } from "@/ui/popover";
import { REAL_COLORS, getColorHex, getColorTitle, makeColor } from "@/ui/style";
import {
  ViewContentWrapper,
  makeViewId,
  viewEmits,
  type FocusAnchor,
  type ViewExposed,
  type ViewProps,
} from "@/views/common";
import { ref, toRef, type Ref } from "vue";

const COLORS_PER_ROW = 9;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: ColorData } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "variant" | "valueType" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const itemRefs: Ref<Partial<Record<ColorType, HTMLButtonElement>>> = ref({});
const activeResultId: Ref<ColorType | null> = ref(props.modelValue?.type ?? null);

function fire(item: ColorType | ColorData) {
  if (typeof item == "number") item = makeColor(item);
  apply(item);
}
function apply(color: ColorData) {
  emit("update:modelValue", color);
  emit("apply", color);
}

function focus(anchor?: "up" | "down" | "left" | "right" | FocusAnchor | NodeReferenceData) {
  if (props.isInline) {
    let nextIdx;
    const currentIdx = REAL_COLORS.indexOf(activeResultId.value ?? ColorType.GRAY);
    if (anchor == "up") nextIdx = currentIdx - COLORS_PER_ROW;
    else if (anchor == "down") nextIdx = currentIdx + COLORS_PER_ROW;
    else if (anchor == "left") nextIdx = currentIdx - 1;
    else if (anchor == "right") nextIdx = currentIdx + 1;
    else nextIdx = 0;

    const nextColor = REAL_COLORS[(nextIdx + REAL_COLORS.length) % REAL_COLORS.length];
    activeResultId.value = nextColor;
    itemRefs.value[nextColor]?.focus();
  } else {
    buttonRef.value?.focus();
  }
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <ViewContentWrapper :type="ViewType.COLOR" v-bind="props">
    <!-- TODO :Incomplete: Color.isInput/isDisabled/variants/... -->
    <!-- Dropdown -->
    <button
      v-if="!isInline"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.COLOR,
          placement: 'bottom-left',
          offset: 'referenceWidth',
          props: { ...(props as ViewProps), title: undefined, isInline: true },
          onApply: (value) => apply(value),
        })
      "
      :disabled="isDisabled || !isInput"
      class="group flex w-full flex-row items-center rounded border border-gray-200 px-2 py-1 hover:border-gray-300 data-[popover=true]:border-gray-300"
    >
      <!-- Current Value -->
      <template v-if="modelValue != null">
        <i class="fas fa-circle-small w-5 text-center" :style="{ color: getColorHex(modelValue, ColorShade.S600) }" />
        <div class="ml-1.5">{{ getColorTitle(modelValue) ?? "???" }}</div>
      </template>
      <template v-else>
        <i class="fas fa-palette w-5 text-center text-gray-400 group-hover:text-gray-700" />
        <span class="ml-1.5 text-gray-400 group-hover:text-gray-700">Select Color</span>
      </template>
      <!-- Controls -->
      <div v-if="!isDisabled && isInput" class="ml-auto flex-shrink-0 pl-1.5">
        <!-- Clear -->
        <button
          v-if="modelValue != null && !valueType?.isRequired"
          class="mr-2 text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        >
          <i class="fas fa-xmark" />
        </button>
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-primary-900" />
      </div>
    </button>

    <!-- Inline Multi-Toggle -->
    <div v-else class="grid grid-cols-9 gap-x-0.5 gap-y-0.5 rounded border-gray-200 bg-white px-1 py-1">
      <button
        v-for="(color, i) in REAL_COLORS"
        :ref="(ref?: any) => (ref != null ? (itemRefs[color] = ref) : delete itemRefs[color])"
        :key="i"
        v-tooltip="{ title: getColorTitle(color), showDelay: 200, hideDelay: 100, small: true }"
        :disabled="isDisabled || !isInput"
        class="rounded border border-transparent px-1 py-0.5 outline-none hover:border-gray-300 hover:bg-gray-100 focus:ring-0 data-[active=true]:border-gray-300 data-[active=true]:bg-gray-100"
        :data-selected="color == modelValue?.type"
        :data-active="color == activeResultId"
        @click.stop.prevent="fire(color)"
        @keydown.enter.stop.prevent="fire(color)"
        @keydown.up.stop.prevent="focus('up')"
        @keydown.down.stop.prevent="focus('down')"
        @keydown.left.stop.prevent="focus('left')"
        @keydown.right.stop.prevent="focus('right')"
      >
        <i class="fas fa-circle" :style="{ color: getColorHex(color, ColorShade.S600) }" />
      </button>
    </div>
  </ViewContentWrapper>
</template>
