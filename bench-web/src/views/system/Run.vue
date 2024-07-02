<script lang="ts" setup>
import { ViewData, NodeType, RunData } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef, type Ref } from "vue";
import { useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { PACKAGE_SCOPE } from "@/system/client";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { connection, graph } =
  props.preparedConnection ??
  useGetConnection(
    { name: `log.${props.nodePtr?.id}` },
    computed(() => ({
      scope: PACKAGE_SCOPE.value,
      roots: [props.nodePtr!],
      isEnabled: props.nodePtr != null,
      ancestorTypes: [NodeType.RUN],
      descendantTypes: [NodeType.RUN],
    })),
  );
const run = graph.getRef(props.nodePtr, { ignoreAncestors: true }) as Ref<RunData | undefined>;

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    Run!
    {{ run && describeNode(run) }}
    nocheckin
  </div>
</template>
