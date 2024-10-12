<script lang="ts" setup>
import { makeExpression } from "@/language/expression";
import { makeTypeInfo } from "@/language/field";
import { isRunnable, isRunTerminal, type RunnableNode } from "@/language/session";
import {
  BoxData,
  ChangeCategory,
  ExpressionOp,
  FeedViewStateData,
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
import { toggleHelperViewPin, useViewState, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue, mapRef } from "@/utils/ref";
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
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "nodePtr" | "valuePacked" | "focus" | "expansion" | "variant"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const focusPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(focusPtr);
const {
  state,
  updateState,
  useStateProp: useViewStateProp,
} = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.START_VIEW_STATE,
  props,
  emit,
});
const runPtr = useViewStateProp(canvas.tx, "runPtr", undefined) as Ref<
  TypedNodeReferenceData<NodeType.RUN> | undefined
>;
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
const feedState = computed((): FeedViewStateData => {
  const runNodeProperty = runnablePtr.value?.nodeType == NodeType.BLOCK ? RunProperty.blockPtr : RunProperty.stepPtr;
  const clauses = [
    makeExpression({
      op: ExpressionOp.EQUALS,
      propertyPtr: propertyReference(ObjectType.RUN, runNodeProperty),
      value: runnablePtr.value,
    }),
  ];
  if (runnablePtr.value?.nodeType == NodeType.BLOCK) {
    clauses.push(
      makeExpression({
        op: ExpressionOp.NOT_EXISTS,
        propertyPtr: propertyReference(ObjectType.RUN, RunProperty.stepPtr),
        value: runnablePtr.value,
      }),
    );
  }
  const feedState: FeedViewStateData = {
    // pre-filter to only runs of this node
    metatype: ObjectType.FEED_VIEW_STATE,
    nodeType: NodeType.RUN,
    filter: makeExpression({ op: ExpressionOp.AND, clauses }),
    filterPills: state.value.feed?.filterPills ?? [],
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
  if (isNode(lastRunnableNode.value, NodeType.STEP) && lastRunnableNode.value.type == StepType.BLOCK)
    return lastRunnableNode.value.nodePtr;
  else return runnablePtr.value ?? undefined;
});
const inputsPacked: Ref<Record<string, any>> = mapRef(
  useViewStateProp(canvas.tx, "inputsPacked", undefined, { debounce: "short" }), // have to :DebounceNestedValue
  (packed) => (packed != null ? packed : {}) as Record<string, any>,
  (unpacked) => unpacked,
);
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

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="lastRunnableNode" class="h-full w-full">
    <!-- NOTE :UX: start view is ugly -->
    <!-- Header -->
    <div class="group flex w-full flex-row items-center" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Runnable -->
        <NodeReference class="font-medium" :node="lastRunnableNode" is-input :connection="pkgConnection" />
        <!-- Pin/unpin node -->
        <button
          v-tooltip="{ title: 'Pin node in view', small: true, placement: 'bottom' }"
          :disabled="nodePtr == null && lastRunnableNode == null"
          class="ml-1.5 hover:text-primary-900"
          :class="nodePtr != null ? 'text-gray-700' : 'text-gray-400'"
          @click="toggleHelperViewPin(spaceConnection.tx, spaceGraph, { self, nodePtr: lastRunnableNode })"
        >
          <i class="fas mr-1.5" :class="nodePtr == null ? 'fa-unlock' : 'fa-lock'" />
        </button>
        <!-- Meta & Controls -->
        <div class="ml-auto flex flex-row items-center gap-x-2 pl-1.5">
          <!-- Start -->
          <button
            :disabled="lastRunnableNode == null"
            class="h-fit enabled:text-gray-900 enabled:hover:text-primary-900 disabled:text-gray-400"
            @click="() => createRun()"
          >
            <i class="fas fa-play w-5 text-center" />
            <span class="ml-1">Start</span>
          </button>
          <!-- Stop -->
          <button
            :disabled="lastRunnableNode == null || run == null || isRunTerminal(run)"
            class="h-fit enabled:text-gray-900 enabled:hover:text-primary-900 disabled:text-gray-400"
            @click="
              () =>
                run &&
                runConnection.tx.with({ category: ChangeCategory.SESSION }).update(run, { killedAt: Timestamp.now() })
            "
          >
            <i class="fas fa-stop w-5 text-center" />
            <span class="ml-1">Stop</span>
          </button>
        </div>
      </div>
    </div>
    <!-- Body -->
    <Scroll
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
