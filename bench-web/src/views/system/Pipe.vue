<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import { pathToSvg, PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { isGeneratedNodeName } from "@/language/node";
import {
  ColorShade,
  ColorType,
  NodeType,
  PipeFilter,
  PipeModulation,
  PipeType,
  PortType,
  Variant,
  ViewData,
} from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { ICON_BY_PIPE_FILTER, ICON_BY_PIPE_MODULATION, IconInline } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const pipePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.PIPE>);
const flowCtx = useFlowContext();
const state = flowCtx.pipesStates.value[pipePtr.value.id!]; // must exist
const { pipe, source, target, path, sourcePort, targetPort } = state;
const pathSvg = computed(() => (path.value != null ? pathToSvg(path.value.points) : undefined));
const pathColorHex = computed(() => getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S600));
const pathBackgroundColorHex = computed(() => {
  return getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S500);
});

const isInspected = computed(() => canvas.isInspected(pipePtr.value));
const isHighlighted = computed(
  () =>
    canvas.isHighlighted(pipePtr.value) ||
    (pipe.value?.sourcePort?.type == PortType.FIELD &&
      (canvas.isHighlighted(pipe.value.sourcePort.fieldPtr) || canvas.isInspected(pipe.value.sourcePort.fieldPtr))) ||
    (pipe.value?.targetPort?.type == PortType.FIELD &&
      (canvas.isHighlighted(pipe.value.targetPort.fieldPtr) || canvas.isInspected(pipe.value.targetPort.fieldPtr))),
);
const isHidden = computed(() => pipe.value?.isHidden && !isInspected.value && !isHighlighted.value);
const isGeneratedName = computed(() => pipe.value != null && isGeneratedNodeName(pipe.value.metatype, pipe.value.name));

//
// Interaction
//

// actions
const actions: Partial<ActionMapImplementation<"common" | "pipe">> = {
  "pipe.edit.isControl": {
    action: () => {
      if (pipe.value == null) return false;
      const newType = pipe.value.type == PipeType.CONTROL_AND_DATA ? PipeType.DATA : PipeType.CONTROL_AND_DATA;
      flowCtx.tx.update(pipe.value, { type: newType }, { debounce: "tick" });
    },
  },
  "pipe.edit.isHidden": {
    action: () => {
      if (pipe.value == null) return false;
      flowCtx.tx.update(pipe.value, { isHidden: !pipe.value.isHidden }, { debounce: "tick" });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div v-if="pipe != null && path != null" :class="isHidden ? 'group pointer-events-none z-30' : ''">
    <!-- Background/outline path for highlighting (and larger hit area) -->
    <svg
      class="absolute cursor-pointer overflow-visible transition-colors duration-150"
      :class="
        isInspected || isHighlighted
          ? pipe.isHidden
            ? 'opacity-80'
            : 'opacity-100'
          : pipe.isHidden
            ? 'opacity-0'
            : 'opacity-0 hover:opacity-50'
      "
      :style="{ color: pathBackgroundColorHex }"
    >
      <path
        :stroke-width="PIPE_WIDTH * 2"
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke="currentColor"
        fill="none"
        :d="pathSvg"
      />
    </svg>

    <!-- Primary path -->
    <svg
      class="pointer-events-none absolute overflow-visible transition-colors duration-150"
      :class="pipe.isHidden ? (isInspected || isHighlighted ? 'opacity-80' : 'opacity-0') : 'opacity-100'"
      :style="{ color: pathColorHex }"
    >
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :stroke-dasharray="pipe.type === PipeType.DATA ? `${PIPE_WIDTH * 3},${PIPE_WIDTH * 2}` : undefined"
        :d="pathSvg"
      />
    </svg>

    <!-- Midpoint meta -->
    <div
      v-if="!isHidden"
      class="absolute flex -translate-x-1/2 -translate-y-[80%] select-none flex-col items-center gap-x-1 transition-colors duration-150"
      :style="{ left: path.midpoint.x + 'px', top: path.midpoint.y + 'px' }"
    >
      <!-- Name -->
      <NativeInput
        ref="nameRef"
        class="flex-shrink-0 font-medium transition-colors duration-150"
        :class="[
          isGeneratedName && !isInspected && !isHighlighted
            ? 'opacity-0'
            : pipe.isHidden
              ? 'opacity-80'
              : 'opacity-100',
          isInspected || isHighlighted || !isGeneratedName ? 'text-gray-700' : 'text-gray-400',
        ]"
        is-input
        :value-type="NAME_TYPE"
        :variant="Variant.STEALTH"
        :model-value="pipe.name"
        @update:model-value="(newValue) => flowCtx.tx.update(pipe!, { name: newValue }, { debounce: 'long' })"
      />
      <!-- Filter/Mapping/... -->
      <IconInline
        v-if="pipe.filter != null"
        v-tooltip="{ title: toCamelName(PipeFilter, pipe.filter), small: true }"
        v-bind="ICON_BY_PIPE_FILTER[pipe.filter]"
        class="w-5 rounded bg-white text-center text-gray-700"
      />
      <IconInline
        v-if="pipe.modulation != null"
        v-tooltip="{ title: toCamelName(PipeModulation, pipe.modulation), small: true }"
        v-bind="ICON_BY_PIPE_MODULATION[pipe.modulation]"
        class="w-5 rounded bg-white text-center text-gray-700"
      />
    </div>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
