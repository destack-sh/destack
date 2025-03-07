<script lang="ts" setup>
import { isRunActive } from "@/language/runtime/run";
import { ColorShade, ColorType, LinkType, NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/runtime/runtime";
import { canvas } from "@/system/space";
import { ActionMapKit } from "@/ui/action";
import { LINK_WIDTH, pathToSvg, useFlowContext } from "@/ui/flow";
import { getColorHex, getRunColorHex } from "@/ui/style";
import NodeReference from "@/views/builtins/NodeReference.vue";
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
const state = canvas.registerView(self, id);

const linkPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.LINK>);
const flowCtx = useFlowContext();
const linkState = flowCtx.linksStates.value[linkPtr.value.id!]; // must exist
const { link, source, target, path } = linkState;
const pathColorHex = computed(() => {
  const color = link.value?.color?.type ?? ColorType.GRAY;
  if (color == ColorType.GRAY) {
    return getColorHex(color, ColorShade.S400);
  } else {
    return getColorHex(color, ColorShade.S500);
  }
});

const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ id: linkPtr.value?.id }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ id: linkPtr.value?.id }));

const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

const isInspected = computed(() => canvas.isInspected(linkPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(linkPtr.value));
const isSelected = computed(() => state.isSelected(linkPtr.value));
const strokeDashArray = computed(() => {
  if (link.value?.type == LinkType.DECIDE) {
    // dashed
    return `${LINK_WIDTH * 3},${LINK_WIDTH * 2}`;
  } else {
    return undefined;
  }
});

//
// Interaction
//

// actions
const actions: Partial<ActionMapKit<"space" | "link">> = {
  "space.edit.rename": () => {
    nameRef.value?.focusIdentifier();
  },
};

defineExpose<ViewExpose>({ self, id, actions });
</script>
<template>
  <div v-if="link != null && path != null" class="group pointer-events-none z-30" data-ignore-element="self">
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
          :id="'arrowhead-main-' + link.id"
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
        :stroke-width="LINK_WIDTH * 3"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        fill="none"
        stroke="transparent"
        class="pointer-events-auto cursor-pointer"
        :data-node-type="link.metatype"
        :data-node-id="link.id"
        data-suppress-drag="select"
        :d="pathToSvg(path)"
      />

      <!-- Main Path -->
      <path
        :stroke-width="isInspected || isHighlighted || isSelected ? LINK_WIDTH * 1.5 : LINK_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :marker-end="'url(#arrowhead-main-' + link.id + ')'"
        class="pointer-events-none transition-colors duration-150"
        :stroke-dasharray="strokeDashArray"
        :d="pathToSvg(path)"
      />
    </svg>
  </div>
  <div v-else>
    <!-- link without valid path, can't show anything meaningful here -->
  </div>
</template>
