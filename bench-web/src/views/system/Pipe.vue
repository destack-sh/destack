<script lang="ts" setup>
import { NAME_TYPE } from "@/language/field";
import { pathToSvgSpline, PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { isGeneratedNodeName } from "@/language/node";
import { Alignment, ColorShade, ColorType, NodeType, PipeType, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { ICON_BY_PIPE_TYPE, IconInline } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { viewEmits, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const pipePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.PIPE>);
const flowCtx = useFlowContext();
const state = flowCtx.pipesStates.value[pipePtr.value.id!]; // must exist
const { pipe, source, target, path } = state;
const pathSvg = computed(() => (path.value != null ? pathToSvgSpline(path.value.points) : undefined));
const pathColorHex = computed(() => getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S400));

const isInspected = computed(() => canvas.isInspected(pipePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(pipePtr.value));
const isHidden = computed(() => pipe.value?.isHidden && !isInspected.value && !isHighlighted.value);
const isGeneratedName = computed(() => pipe.value != null && isGeneratedNodeName(pipe.value.metatype, pipe.value.name));

//
// Interaction
//

// actions
const actions: Partial<ActionMapImplementation<"common" | "pipe">> = {};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div v-if="pipe != null && path != null" :class="isHidden ? 'group pointer-events-none z-30' : ''">
    <!-- Path -->
    <svg class="absolute cursor-pointer overflow-visible">
      <defs>
        <marker
          id="arrowhead"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          :stroke="pathColorHex"
          :fill="pathColorHex"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M0,0 L10,3.5 L0,7 L2,3.5 Z" :fill="pathColorHex" />
        </marker>
      </defs>
      <!-- Background path -->
      <path
        :stroke-width="PIPE_WIDTH * 2"
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke="currentColor"
        fill="none"
        :stroke-dasharray="pipe.type === PipeType.STREAM ? `${PIPE_WIDTH * 3},${PIPE_WIDTH * 3}` : undefined"
        :d="pathSvg"
        class="text-primary-700 transition-colors duration-150"
        :class="isInspected || isHighlighted ? 'opacity-100' : 'opacity-0 hover:opacity-50'"
      />
      <!-- Main path -->
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        marker-end="url(#arrowhead)"
        class="transition-colors duration-150"
        :style="{ color: pathColorHex }"
        :stroke-dasharray="pipe.type === PipeType.STREAM ? `${PIPE_WIDTH * 3},${PIPE_WIDTH * 3}` : undefined"
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
        id="name"
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
        :alignment="Alignment.MIDDLE"
        :variant="Variant.STEALTH"
        :model-value="pipe.name"
        @update:model-value="(newValue) => flowCtx.tx.update(pipe!, { name: newValue as string }, { debounce: 'long' })"
      />
      <!-- Filter/Mapping/... -->
      <IconInline
        v-if="pipe.type != PipeType.GO"
        v-bind="ICON_BY_PIPE_TYPE[pipe.type]"
        class="flex h-4 w-4 flex-col justify-center rounded-2xl text-center text-white"
        :style="{ backgroundColor: pathColorHex }"
      />
    </div>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
