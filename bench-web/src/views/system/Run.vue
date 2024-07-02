<script lang="ts" setup>
import { ViewData, NodeType, RunData, FieldZone } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef, type Ref } from "vue";
import { useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { PACKAGE_SCOPE } from "@/system/client";
import { unpackProtoJson } from "@/system/transaction";
import { getFieldViews } from "@/system/view";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline" | "variant">
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
const basePtr = computed(() => run.value?.stepPtr ?? run.value?.blockPtr);
const runnableNode = graph.getRef(basePtr, { ignoreAncestors: true });

const inputsPacked = computed(
  () => (run.value?.inputsPacked != null ? unpackProtoJson(run.value.inputsPacked) : {}) as Record<string, any>,
);
const inputFields = graph.getChildrenRef(runnableNode, NodeType.FIELD); // resolve these later :TypeResolution
const inputViews = computed(() =>
  getFieldViews(inputFields.value, inputsPacked.value, graph, { zones: [FieldZone.INPUT], isInput: true }),
);
const outputsPacked = computed(
  () => (run.value?.outputsPacked != null ? unpackProtoJson(run.value.outputsPacked) : {}) as Record<string, any>,
);
const outputFields = graph.getChildrenRef(runnableNode, NodeType.FIELD); // resolve these later :TypeResolution
const outputViews = computed(() =>
  getFieldViews(outputFields.value, outputsPacked.value, graph, { zones: [FieldZone.OUTPUT], isInput: false }),
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    Run!
    {{ run && describeNode(run) }}
    nocheckin
    <!-- Inputs -->
    <div class="flex flex-col">
      {{ inputsPacked }}
    </div>
    <!-- Outputs -->
    <div class="flex flex-col">
      {{ outputsPacked }}
    </div>
    <!-- Error -->
     <div class="">
      {{ run?.error?.type }}
     </div>
  </div>
</template>
