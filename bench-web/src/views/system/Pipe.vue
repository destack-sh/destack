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
const { pipe, source, target, path, sourcePort, targetPort } = state;
const pathSvg = computed(() => (path.value != null ? pathToSvg(path.value.points) : undefined));
const pathColorHex = computed(() => getColorHex(pipe.value?.color ?? ColorType.GRAY, ColorShade.S600));

//
// Interaction
//

const isInspected = computed(() => inspectionPtr.value?.id == pipePtr.value?.id);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="pipe && path">
    <!-- Background/outline path for highlighting (and larger hit area) -->
    <svg
      class="absolute cursor-pointer overflow-visible blur-sm text-primary-500 transition-all duration-75"
      :class="isInspected ? 'opacity-80' : 'opacity-0 hover:opacity-60'"
    >
      <path
        :stroke-width="PIPE_WIDTH + 2"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        fill="none"
        :d="pathSvg"
      />
    </svg>
    <!-- Primary path -->
    <svg class="pointer-events-none absolute overflow-visible" :style="{ color: pathColorHex }">
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        :stroke-dashoffset="5 ?? 0"
        fill="none"
        :stroke-dasharray="
          pipe.type == PipeType.THEN ? `${PIPE_WIDTH * 3},${PIPE_WIDTH * 2}` : `${PIPE_WIDTH * 0.5},${PIPE_WIDTH * 2}`
        "
        :d="pathSvg"
      >
        <animate
          attributeName="stroke-dashoffset"
          from="0"
          to="-60"
          :dur="`${pipe.type == PipeType.THEN ? '4s' : '4s'}`"
          repeatCount="indefinite"
          fill="freeze"
        />
      </path>
    </svg>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
