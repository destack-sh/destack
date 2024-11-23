<script lang="ts" setup>
import { NAME_TYPE } from "@/language/field";
import { pathToSvg, PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { isGeneratedNodeName, makeNodeName } from "@/language/node";
import { isRunActive } from "@/language/session";
import { ColorShade, ColorType, NodeType, PipeType, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/system/runtime";
import { canvas } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { ICON_BY_PIPE_TYPE, IconInline } from "@/ui/icon";
import { COLOR_BY_RUN_STATUS, getColorHex, getRunColorHex } from "@/ui/style";
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
const pathColorHex = computed(() => {
  const color = pipe.value?.color?.type ?? ColorType.GRAY;
  if (color == ColorType.GRAY) {
    return getColorHex(color, ColorShade.S400);
  } else {
    return getColorHex(color, ColorShade.S600);
  }
});

const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ ck: pipePtr.value?.ck }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ ck: pipePtr.value?.ck }));

const isInspected = computed(() => canvas.isInspected(pipePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(pipePtr.value));
const isHidden = computed(() => pipe.value?.isHidden && !isInspected.value && !isHighlighted.value);
const isGeneratedName = computed(() => pipe.value != null && isGeneratedNodeName(pipe.value.metatype, pipe.value.name));
const showPipeMeta = computed(() => !pipe.value?.isNameHidden || pipe.value?.type != PipeType.PASS);

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
    <svg
      class="group relative cursor-pointer overflow-visible"
      :class="lastRun != null && isRunActive(lastRun) ? 'animate-pulse' : ''"
      :style="{ color: lastRun != null ? getRunColorHex(lastRun.status) : pathColorHex }"
    >
      <defs>
        <!-- Main Arrowhead Marker -->
        <marker
          :id="'arrowhead-main-' + pipe.id"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M0,0 L10,3.5 L0,7 L2,3.5 Z" fill="currentColor" />
        </marker>

        <!-- Background Arrowhead Marker (Larger) -->
        <marker
          :id="'arrowhead-background-' + pipe.id"
          markerWidth="14"
          markerHeight="10"
          refX="12"
          refY="5"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M0,0 L14,5 L0,10 L3,5 Z" fill="currentColor" />
        </marker>
      </defs>

      <!-- Background Path for Hover and Hit Target -->
      <path
        :stroke-width="PIPE_WIDTH * 2"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        fill="none"
        :marker-end="'url(#arrowhead-background-' + pipe.id + ')'"
        :d="pathToSvg(path)"
        class="transition-colors duration-150"
        :class="isInspected || isHighlighted ? 'stroke-current' : 'stroke-transparent group-hover:stroke-current'"
      />

      <!-- Main Path -->
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :marker-end="'url(#arrowhead-main-' + pipe.id + ')'"
        class="transition-colors duration-150"
        :stroke-dasharray="pipe.type === PipeType.STREAM ? `${PIPE_WIDTH * 3},${PIPE_WIDTH * 3}` : undefined"
        :d="pathToSvg(path)"
      />
    </svg>

    <!-- Midpoint meta -->
    <div
      class="group/meta absolute z-10 flex -translate-x-1/2 -translate-y-1/2 select-none flex-row items-center rounded-2xl border transition-colors duration-150"
      :class="[
        showPipeMeta
          ? [isInspected || isHighlighted ? 'border-gray-300 bg-gray-100' : 'border-gray-200 bg-white']
          : 'border-transparent bg-transparent',
        pipe.isNameHidden ? 'px-0.5' : 'px-2',
      ]"
      :style="{ left: path.midpoint.x + 'px', top: path.midpoint.y + 'px' }"
    >
      <!-- Type -->
      <IconInline
        v-if="pipe.type != PipeType.PASS"
        v-bind="ICON_BY_PIPE_TYPE[pipe.type]"
        class="flex h-4 w-4 flex-col justify-center rounded-2xl text-center"
        :class="pipe.type == PipeType.OPTION ? ' ' : 'text-gray-700'"
        :style="{
          color: pipe.type == PipeType.OPTION ? pathColorHex : undefined,
        }"
      />
      <!-- Name -->
      <NativeInput
        v-if="!pipe.isNameHidden"
        id="name"
        ref="nameRef"
        class="ml-1.5 mr-1.5 flex-shrink-0 transition-colors duration-150"
        :class="[
          pipe.isNameHidden && !isInspected && !isHighlighted ? 'opacity-0' : 'opacity-100',
          isInspected || isHighlighted || !pipe.isNameHidden ? 'text-gray-700' : 'text-gray-400',
        ]"
        is-input
        placeholder="Name..."
        :value-type="NAME_TYPE"
        :variant="Variant.STEALTH"
        :model-value="pipe.name"
        @update:model-value="(newValue) => flowCtx.tx.update(pipe!, { name: newValue as string }, { debounce: 'long' })"
      />
    </div>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
