<script lang="ts" setup>
import { PIPE_WIDTH, useFlowContext } from "@/language/flow";
import { NodeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
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
const ctx = useFlowContext();

const pipe = pkgGraph.getRef(pipePtr, { ignoreAncestors: props.self == null });
const source = pkgGraph.getRef(computed(() => pipe.value?.sourcePtr));
const target = pkgGraph.getRef(computed(() => pipe.value?.targetPtr));

//
// Interaction
//

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="pipe">
    <!-- nocheckin: pipe -->
    pipedy pipe
    <!-- <path
      :stroke-width="PIPE_WIDTH"
      stroke-linecap="round"
      stroke-linejoin="bevel"
      stroke="currentColor"
      :d="
        ctx.computePathSvg(
          ctx.getPortPosition(ctx.draggable.step, ctx.draggable.port)!,
          ctx.draggable.cursorWorldPos!,
        )
      "
    /> -->
  </div>
  <Inaccessible v-else class="h-full w-full" :node="pipePtr" :connection="pkgConnection" />
</template>
