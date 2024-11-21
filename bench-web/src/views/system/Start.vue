<script lang="ts" setup>
import { makeExpression } from "@/language/expression";
import { makeTypeInfo } from "@/language/field";
import { unpackSubnodeProperty, useSubnodeProperty } from "@/language/node";
import { isRunnable, type RunnableNode } from "@/language/session";
import {
  ExpressionType,
  FeedViewData,
  FieldType,
  NodeType,
  ObjectType,
  Orientation,
  RunProperty,
  StepType,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import {
  isNode,
  propertyReference,
  toNodeRef,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { useExistingConnection, useNode } from "@/system/connection";
import { runtime } from "@/system/runtime";
import { canvas, inspectionPtr } from "@/system/space";
import { ScrollbarWidth } from "@/ui/layout";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import CustomObject from "@/views/system/CustomObject.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "nodePtr" | "focus" | "size" | "variant" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const focusPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(focusPtr);

const runPtr = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.START,
  toRef(props, "subnodePacked"),
  "runPtr",
) as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;
const { node: someRun, connection: runConnection } = useNode({
  name: "start.run",
  live: true,
  nodePtr: runPtr,
  isOptional: true,
  isEnabled: computed(() => runPtr.value != null),
});
const run = computed(() => {
  if (runPtr.value == null || runPtr.value.baseCk != runnablePtr.value?.ck) return null;
  else return someRun.value;
});
const feedState = computed((): FeedViewData => {
  const runNodeProperty = runnablePtr.value?.nodeType == NodeType.BLOCK ? RunProperty.blockPtr : RunProperty.stepPtr;
  const clauses = [
    makeExpression({
      type: ExpressionType.EQUALS,
      propertyPtr: propertyReference(ObjectType.RUN, runNodeProperty),
      value: runnablePtr.value,
    }),
  ];
  if (runnablePtr.value?.nodeType == NodeType.BLOCK) {
    clauses.push(
      makeExpression({
        type: ExpressionType.NOT_EXISTS,
        propertyPtr: propertyReference(ObjectType.RUN, RunProperty.stepPtr),
        value: runnablePtr.value,
      }),
    );
  }
  const feedState: FeedViewData = {
    // pre-filter to only runs of this node
    queryNodeType: NodeType.RUN,
    filter: makeExpression({ type: ExpressionType.AND, clauses }),
    filterPills: [],
  };
  return feedState;
});
const ancestors = pkgGraph.getAncestorsRef(focusPtr, { includeSelf: true });

// current runnable / inputs
const node = pkgGraph.getRef(nodePtr);
const runnablePtr = computed(() => (node.value != null ? toPlainNodeRef(node.value) : null));
const inputsPacked = useSubnodeProperty(NodeType.VIEW, ViewType.START, toRef(props, "subnodePacked"), "inputsPacked");
const inputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: runnablePtr.value, baseFieldType: FieldType.INPUT })
    : undefined,
);
const outputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: runnablePtr.value, baseFieldType: FieldType.OUTPUT })
    : undefined,
);

const inputsRef: Ref<InstanceType<typeof CustomObject> | null> = ref(null);
const outputsRef: Ref<InstanceType<typeof CustomObject> | null> = ref(null);

function createRun() {
  if (node.value == null || !isRunnable(node.value)) return;
  const run = runtime.createRun(node.value, { inputsPacked: inputsPacked.value as any });
  runPtr.value = toNodeRef(run);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="node" class="flex h-full w-full flex-col gap-y-1.5 px-5">
    <!-- nocheckin: proper Run controls -->
    <button @click="createRun">start</button>
    <!-- Inputs -->
    <div class="flex-1">
      <h4 class="font-semibold">Inputs</h4>
      <CustomObject
        id="inputs"
        ref="inputsRef"
        class="w-full py-1"
        :value-type="inputType"
        is-inline
        is-input
        :variant="Variant.STEALTH"
        :model-value="inputsPacked"
        @update:model-value="(value) => (inputsPacked = value)"
      />
    </div>
    <!-- Outputs (last run) -->
    <div v-if="run?.outputsPacked != null" class="flex-1">
      <h4 class="font-semibold">Outputs</h4>
      <CustomObject
        id="outputs"
        ref="outputsRef"
        class="w-full py-1"
        :value-type="outputType"
        is-inline
        :variant="Variant.STEALTH"
        :model-value="run.outputsPacked"
      />
    </div>
    <!-- Error (last run) -->
    <div v-else-if="run?.error != null" class="flex-1">
      <h4 class="font-semibold">Error</h4>
      <RunError class="mt-2" :run="run" :error="run.error" />
    </div>
    <div v-else class="flex-1 text-center">
      <!-- Placeholder -->
    </div>
    <!-- Timeline -->
    <div v-if="runPtr && run != null">
      <h4 class="font-semibold">Timeline</h4>
      <RunTimeline :node-ptr="runPtr" class="mt-2" />
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty state -->
  </div>
</template>
