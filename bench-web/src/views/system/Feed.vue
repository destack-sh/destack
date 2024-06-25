<script lang="ts" setup>
import { ViewData, NodeType, BoxData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { useSearchConnection } from "@/system/connection";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size?: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "variant" | "isInput" | "isInline" | "valueType" | "valuePacked">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

// TODO :Incomplete!: store Feed query (and View-type-specific data) in view ndoe
const { roots, graph, connection } = useSearchConnection(
  { name: `feed.${id}` },
  {
    nodeType: NodeType.LOG, // nocheckin: parameterize Feed search
		first: 5,
  },
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    le feed
    <!-- nocheckin: Feed -->
  </div>
</template>
