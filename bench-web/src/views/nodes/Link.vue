<script lang="ts" setup>
import { NAME_TYPE } from "@/language/core/type";
import { isRunActive } from "@/language/runtime/run";
import { ColorShade, ColorType, NodeType, LinkType, LinkTypeOptionInfo, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/runtime/runtime";
import { canvas, pkgConnection } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { pathToSvg, LINK_WIDTH, useFlowContext } from "@/ui/flow";
import { IconInline, makeIcon } from "@/ui/icon";
import { getColorHex, getRunColorHex } from "@/ui/style";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
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
    return getColorHex(color, ColorShade.S600);
  }
});

const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ ck: linkPtr.value?.ck }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ ck: linkPtr.value?.ck }));

const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

const isInspected = computed(() => canvas.isInspected(linkPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(linkPtr.value));
const isSelected = computed(() => state.isSelected(linkPtr.value));
const strokeDashArray = computed(() => {
  if (link.value?.type === LinkType.AUTO) {
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
const actions: Partial<ActionMapImplementation<"space" | "link">> = {
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
        :data-node-ck="link.ck"
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
      :data-node-type="link.metatype"
      :data-node-id="link.id"
      :data-node-ck="link.ck"
      aria-hidden
    >
      <!-- Type -->
      <IconInline
        v-bind="makeIcon(LinkTypeOptionInfo[link.type]!.icon!)"
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
        aria-hidden
        :placeholder-color="pathColorHex"
        :value-type="NAME_TYPE"
        :model-value="link.name"
        @update:model-value="(name) => pkgConnection.tx.update(link!, { name: name as string }, { debounce: 'long' })"
      />
    </div>
  </div>
  <div v-else>
    <!-- link without valid path, can't show anything meaningful here -->
  </div>
</template>
