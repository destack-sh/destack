<script lang="ts" setup>
import { PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { ColorShade, NodeType, PipeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
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

//
// Interaction
//

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="pipe && path">
    <svg
      class="cursor-pointer overflow-visible text-gray-600"
      :style="{
        color: pipe.color != null ? getColorHex(pipe.color, pipe.color?.shade ?? ColorShade.S600) : undefined,
      }"
    >
      <path
        :stroke-width="PIPE_WIDTH"
        stroke-linecap="round"
        stroke-linejoin="bevel"
        stroke="currentColor"
        :stroke-dasharray="pipe.type == PipeType.THEN ? undefined : '8,8'"
        :d="flowCtx.pathToSvg(path)"
      />
    </svg>
  </div>
  <div v-else>
    <!-- pipe without valid path, can't show anything meaningful here -->
  </div>
</template>
