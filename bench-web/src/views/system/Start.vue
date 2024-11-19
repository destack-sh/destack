<script lang="ts" setup>
import { makeExpression } from "@/language/expression";
import { makeTypeInfo } from "@/language/field";
import { unpackSubnodeProperty, useSubnodeProperty } from "@/language/node";
import { isRunnable, isRunTerminal, type RunnableNode } from "@/language/session";
import {
  RectangleData,
  ChangeCategory,
  ExpressionType,
  FeedViewData,
  FieldType,
  NodeType,
  ObjectType,
  Orientation,
  RunProperty,
  StepType,
  Timestamp,
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
import { toggleHelperViewPin, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import CustomObject from "@/views/system/CustomObject.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 1200;

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
  } & Pick<ViewData, "name" | "title" | "nodePtr" | "focus" | "variant" | "subnodePacked">
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
// NOTE: we 'sticky' the last runnable node (so even if we currently don't have one, we keep the last one)
const currentRunnableNode: Ref<RunnableNode | null | undefined> = computed(() =>
  ancestors.value.find((node) => isRunnable(node, pkgGraph)),
);
const lastRunnableNode: Ref<RunnableNode | null> = ref(null);
watch(currentRunnableNode, (newNode) => {
  if (newNode != null) lastRunnableNode.value = newNode;
});
const runnablePtr = computed(() => (lastRunnableNode.value != null ? toPlainNodeRef(lastRunnableNode.value) : null));
const baseTypePtr = computed(() => {
  if (isNode(lastRunnableNode.value, NodeType.STEP) && lastRunnableNode.value.type == StepType.ACTION) {
    return unpackSubnodeProperty(NodeType.STEP, StepType.ACTION, lastRunnableNode.value.subnodePacked, "delegatePtr");
  } else {
    return runnablePtr.value ?? undefined;
  }
});
const inputsPacked: Ref<Record<string, any>> = ref({}); // nocheckin
const inputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: baseTypePtr.value, baseFieldType: FieldType.INPUT })
    : undefined,
);
const outputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: baseTypePtr.value, baseFieldType: FieldType.OUTPUT })
    : undefined,
);

function createRun() {
  if (lastRunnableNode.value == null) return;
  const run = runtime.createRun(lastRunnableNode.value, { inputsPacked: inputsPacked.value });
  runPtr.value = toNodeRef(run);
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="lastRunnableNode" class="h-full w-full">
    <!-- Body -->
    <Scroll
      id="scroll"
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div
        class="mx-auto flex flex-col gap-y-2 px-5 pb-5"
        :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Inputs -->
        <div class="flex-1">
          <h4 class="font-semibold">Inputs</h4>
          <CustomObject
            id="inputs"
            class="w-full py-2"
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
            class="w-full py-2"
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
    </Scroll>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- NOTE :UX: display possible nodes to start in context & all runs if nothing runnable selected -->
    <!-- Empty state -->
    <span>
      <i class="fas fa-empty-set text-gray-500" />
      <span class="ml-1.5 text-gray-600">Select Node to Run</span>
    </span>
  </div>
</template>
