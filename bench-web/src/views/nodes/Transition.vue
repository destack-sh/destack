<script lang="ts" setup>
import { isProcessActive } from "@/language/runtime/process";
import { ColorShade, ColorType, NodeType, TransitionType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/runtime/runtime";
import { canvas } from "@/system/space";
import { CommandMapKit } from "@/ui/command";
import { TRANSITION_WIDTH, pathToSvg, useFlowContext } from "@/ui/flow";
import { getColorHex, getProcessColorHex } from "@/ui/style";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, Ref, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform" | "isMinimal">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const transitionPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.TRANSITION>);
const flowCtx = useFlowContext();
const transitionState = flowCtx.transitionsStates.value[transitionPtr.value.id!]; // must exist
const { transition, source, target, path } = transitionState;
const pathColorHex = computed(() => {
  const color = transition.value?.color?.type ?? ColorType.GRAY;
  if (color == ColorType.GRAY) {
    return getColorHex(color, ColorShade.S400);
  } else {
    return getColorHex(color, ColorShade.S400);
  }
});

const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ id: transitionPtr.value?.id }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ id: transitionPtr.value?.id }));

const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

const isInspected = computed(() => canvas.isInspected(transitionPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(transitionPtr.value));
const isSelected = computed(() => canvas.isSelected(transitionPtr.value));
const strokeDashArray = computed(() => {
  if (transition.value?.type == TransitionType.DECIDE) {
    // dashed
    return `${TRANSITION_WIDTH * 3},${TRANSITION_WIDTH * 2}`;
  } else {
    return undefined;
  }
});

//
// Interaction
//

// actions
const commands: Partial<CommandMapKit<"space" | "transition">> = {
  "space.edit.rename": () => {
    nameRef.value?.focusIdentifier();
  },
};

defineExpose<ViewExpose>({ self, id, commands });
</script>
<template>
  <div v-if="transition != null && path != null" class="group pointer-events-none z-30" data-suppress-node="self">
    <!-- Path -->
    <svg
      class="group pointer-events-none relative overflow-visible"
      :class="lastRun != null && isProcessActive(lastRun) ? 'animate-pulse' : ''"
      :style="{ color: lastRun != null ? getProcessColorHex(lastRun.status) : pathColorHex }"
      data-suppress-node="self"
    >
      <defs>
        <!-- Main Arrowhead Marker -->
        <marker
          :id="'arrowhead-main-' + transition.id"
          markerWidth="12"
          markerHeight="8"
          refX="11"
          refY="4"
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path d="M0,0 L12,4 L0,8 L2,4 Z" fill="currentColor" />
        </marker>
      </defs>

      <!-- Background Hit Target -->
      <path
        :stroke-width="TRANSITION_WIDTH * 3"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        fill="none"
        stroke="transparent"
        class="pointer-events-auto cursor-pointer"
        :data-node-type="transition.metatype"
        :data-node-id="transition.id"
        :data-node-ck="(transition as any).ck"
        :data-node-bench-id="(transition as any).benchPtr?.id"
        data-suppress-drag="select"
        :d="pathToSvg(path)"
      />

      <!-- Main Path -->
      <path
        :stroke-width="isInspected || isHighlighted || isSelected ? TRANSITION_WIDTH * 1.5 : TRANSITION_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :marker-end="'url(#arrowhead-main-' + transition.id + ')'"
        class="pointer-events-none transition-colors duration-150"
        :stroke-dasharray="strokeDashArray"
        :d="pathToSvg(path)"
      />
    </svg>
  </div>
  <div v-else>
    <!-- transition without valid path, can't show anything meaningful here -->
  </div>
</template>
