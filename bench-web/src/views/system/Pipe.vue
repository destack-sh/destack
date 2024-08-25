<script lang="ts" setup>
import { PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { NodeType, PipeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const pipePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.PIPE>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(pipePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
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
    <svg class="cursor-pointer overflow-visible text-gray-600">
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
