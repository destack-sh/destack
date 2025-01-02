<script lang="ts" setup>
import { NAME_TYPE } from "@/language/field";
import { isRunActive } from "@/language/session";
import { ColorShade, ColorType, NodeType, PipeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { pathToSvg, PIPE_WIDTH, useFlowContext } from "@/system/flow";
import { runtime } from "@/system/runtime";
import { canvas, pkgConnection } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { ICON_BY_PIPE_TYPE, IconInline } from "@/ui/icon";
import { getColorHex, getRunColorHex } from "@/ui/style";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, Ref, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform" | "isMinimal">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const pipePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.PIPE>);
const flowCtx = useFlowContext();
const pipeState = flowCtx.pipesStates.value[pipePtr.value.id!]; // must exist
const { pipe, source, target, path } = pipeState;
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

const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

const isInspected = computed(() => canvas.isInspected(pipePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(pipePtr.value));
const isSelected = computed(() => state.isSelected(pipePtr.value));
const isHidden = computed(() => pipe.value?.isHidden && !isInspected.value && !isHighlighted.value);
const hasStartMarker = computed(
  () => pipe.value?.type == PipeType.FORWARD_AND_BACK || pipe.value?.type == PipeType.SELECT_AND_BACK,
);
const strokeDashArray = computed(() => {
  if (pipe.value?.type === PipeType.SELECT || pipe.value?.type === PipeType.SELECT_AND_BACK) {
    // dashed
    return `${PIPE_WIDTH * 3},${PIPE_WIDTH * 2}`;
  } else {
    return undefined;
  }
});

//
// Interaction
//

// actions
const actions: Partial<ActionMapImplementation<"space" | "pipe">> = {
  "space.edit.rename": () => {
    nameRef.value?.focusIdentifier();
  },
};

defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="pipe != null && path != null"
    :class="isHidden ? 'group pointer-events-none z-30' : ''"
    data-ignore-element="self"
  >
    <!-- Path -->
    <svg
      class="group pointer-events-none relative overflow-visible"
      :class="lastRun != null && isRunActive(lastRun) ? 'animate-pulse' : ''"
      :style="{ color: lastRun != null ? getRunColorHex(lastRun.status) : pathColorHex }"
      data-ignore-element="self"
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

        <!-- Inverted Start Marker -->
        <marker
          :id="'arrowhead-start-' + pipe.id"
          markerWidth="10"
          markerHeight="7"
          refX="1"
          refY="3.5"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M10,0 L0,3.5 L10,7 L8,3.5 Z" fill="currentColor" />
        </marker>

        <!-- Inverted Start Background Marker -->
        <marker
          :id="'arrowhead-start-background-' + pipe.id"
          markerWidth="14"
          markerHeight="10"
          refX="2"
          refY="5"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M14,0 L0,5 L14,10 L11,5 Z" fill="currentColor" />
        </marker>
      </defs>

      <!-- Background Path for Hover and Hit Target -->
      <path
        :stroke-width="PIPE_WIDTH * 2"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        fill="none"
        :marker-start="hasStartMarker ? 'url(#arrowhead-start-background-' + pipe.id + ')' : undefined"
        :marker-end="'url(#arrowhead-background-' + pipe.id + ')'"
        class="pointer-events-auto cursor-pointer transition-colors duration-150"
        :class="
          isInspected || isHighlighted || isSelected
            ? 'stroke-current'
            : 'stroke-transparent group-hover:stroke-current'
        "
        :data-node-type="pipe.metatype"
        :data-node-id="pipe.id"
        :data-node-ck="pipe.ck"
        data-suppress-drag="select"
        :d="pathToSvg(path)"
        :stroke-dasharray="strokeDashArray"
      />

      <!-- Main Path -->
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :marker-start="hasStartMarker ? 'url(#arrowhead-start-' + pipe.id + ')' : undefined"
        :marker-end="'url(#arrowhead-main-' + pipe.id + ')'"
        class="transition-colors duration-150"
        :stroke-dasharray="strokeDashArray"
        :d="pathToSvg(path)"
      />
    </svg>

    <!-- Midpoint meta -->
    <div
      class="group/meta pointer-events-auto absolute z-10 flex -translate-x-1/2 -translate-y-1/2 cursor-pointer select-none flex-row items-center gap-x-1 rounded-2xl border px-1 py-0.5 transition-colors duration-150"
      :class="[
        isInspected ? 'bg-gray-100/80' : 'bg-gray-100/60',
        isInspected || isHighlighted
          ? 'border-gray-300 opacity-100 backdrop-blur-xs'
          : 'border-gray-200 opacity-0 group-hover/meta:opacity-100',
      ]"
      :style="{ left: path.midpoint.x + 'px', top: path.midpoint.y + 'px' }"
      :data-node-type="pipe.metatype"
      :data-node-id="pipe.id"
      :data-node-ck="pipe.ck"
    >
      <!-- Type -->
      <IconInline
        v-bind="ICON_BY_PIPE_TYPE[pipe.type]"
        class="flex h-4 w-4 flex-col justify-center rounded-2xl text-center text-gray-700"
      />
      <!-- Name -->
      <NativeInput
        id="name"
        ref="nameRef"
        class="w-full text-xs"
        is-input
        is-minimal
        placeholder="Name..."
        :placeholder-color="pathColorHex"
        :value-type="NAME_TYPE"
        :model-value="pipe.name"
        @update:model-value="(name) => pkgConnection.tx.update(pipe!, { name: name as string }, { debounce: 'long' })"
      />
    </div>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
