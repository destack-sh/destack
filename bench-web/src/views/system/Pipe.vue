<script lang="ts" setup>
import { pathToSvg, PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { ColorShade, ColorType, NodeType, PipeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, inspectionPtr } from "@/system/space";
import { getColorHex } from "@/ui/style";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
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
const { pipe, source, target, path } = state;
const pathSvg = computed(() => (path.value != null ? pathToSvg(path.value.points) : undefined));
const pathColorHex = computed(() => getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S600));

//
// Interaction
//

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="pipe && path">
    <!-- Background/outline path for highlighting (and larger hit area) -->
    <!-- NOTE :UX: improve pipe highlighting (maybe use glow?) -->
    <svg
      class="absolute cursor-pointer overflow-visible transition-colors duration-75"
      :class="inspectionPtr?.id == pipePtr.id ? 'text-gray-700' : 'text-transparent hover:text-gray-700'"
    >
      <path
        :stroke-width="PIPE_WIDTH + 2"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :stroke-dasharray="pipe.type == PipeType.THEN ? undefined : '8,8'"
        :d="pathSvg"
      />
    </svg>
    <!-- Primary path -->
    <svg class="pointer-events-none absolute overflow-visible text-gray-600" :style="{ color: pathColorHex }">
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :stroke-dasharray="pipe.type == PipeType.THEN ? undefined : '8,12'"
        :d="pathSvg"
      />
    </svg>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
