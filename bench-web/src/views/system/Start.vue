<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { makeExpression } from "@/language/expression";
import { makeTypeInfo } from "@/language/field";
import { isRunnable, type RunnableNode } from "@/language/session";
import { packBuiltinObject, unpackBuiltinObject } from "@/language/value";
import {
  BoxData,
  ExpressionOp,
  FeedViewStateData,
  FieldZone,
  NodeType,
  ObjectType,
  Orientation,
  RunProperty,
  RunStatus,
  StepType,
  TypeKind,
  Variant,
  ViewData,
} from "@/proto/wire";
import {
  isNode,
  packProtoJson,
  propertyReference,
  toNodeRef,
  toPlainNodeRef,
  unpackProtoJson,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { useExistingConnection, useNode } from "@/system/connection";
import { runtime } from "@/system/runtime";
import { canvas, inspectionPtr } from "@/system/space";
import { ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { ACCENT_COLOR_BY_RUN_STATUS } from "@/ui/style";
import { toggleHelperViewPin, useViewState, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue, mapRef } from "@/utils/ref";
import { tsToDt } from "@/utils/time";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunError from "@/views/builtins/RunError.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Text from "@/views/content/Text.vue";
import Feed from "@/views/system/Feed.vue";
import ValueObject from "@/views/system/ValueObject.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 800;

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
const { node: someRun } = useNode({
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
  const runNodeProperty = runnablePtr.value?.type == NodeType.BLOCK ? RunProperty.blockPtr : RunProperty.stepPtr;
  const runOtherNodeProperty = runnablePtr.value?.type == NodeType.BLOCK ? RunProperty.stepPtr : RunProperty.blockPtr;
  const feedState: FeedViewStateData = {
    // pre-filter to only runs of this node
    metatype: ObjectType.FEED_VIEW_STATE,
    nodeType: NodeType.RUN,
    filter: makeExpression({
      op: ExpressionOp.AND,
      clauses: [
        makeExpression({
          op: ExpressionOp.EQUALS,
          propertyPtr: propertyReference(ObjectType.RUN, runNodeProperty),
          value: runnablePtr.value,
        }),
        // other node property must not be set (i.e. if we want Run.block == X, Run.step should be null and vice versa)
        makeExpression({
          op: ExpressionOp.NOT_EXISTS,
          propertyPtr: propertyReference(ObjectType.RUN, runOtherNodeProperty),
        }),
      ],
    }),
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
  (packed) => (packed != null ? unpackProtoJson(packed) : {}) as Record<string, any>,
  (unpacked) => packProtoJson(unpacked),
);
const inputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: baseTypePtr.value, baseFieldZone: FieldZone.INPUT })
    : undefined,
);
const outputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: baseTypePtr.value, baseFieldZone: FieldZone.OUTPUT })
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
    <div class="group mx-auto flex w-full flex-row items-center" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Runnable -->
        <NodeReference class="font-medium" :node="lastRunnableNode" :connection="pkgConnection" />
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
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <button
            :disabled="lastRunnableNode == null"
            class="h-fit enabled:text-gray-900 enabled:hover:text-primary-900 disabled:text-gray-400"
            @click="() => createRun()"
          >
            <i class="fas fa-play w-5 text-center" />
            <span class="ml-1">Start</span>
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
      <div class="mx-auto px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <!-- Inputs -->
        <div class="mx-auto">
          <h4 class="font-semibold">Inputs</h4>
          <ValueObject
            class="w-full py-2"
            :value-type="inputType"
            is-inline
            is-input
            :variant="Variant.STEALTH"
            :model-value="inputsPacked"
            @update:model-value="(value) => (inputsPacked = value)"
          />
        </div>
        <!-- Divider -->
        <div class="mx-auto my-2 w-full"><div class="h-[1px] w-full min-w-fit bg-gray-200" /></div>
        <!-- Outputs (last run) -->
        <div v-if="run?.outputsPacked != null" class="mx-auto mt-1 py-3">
          <h4 class="font-semibold">Outputs</h4>
          <ValueObject
            class="w-full py-2"
            :value-type="outputType"
            is-inline
            :variant="Variant.STEALTH"
            :model-value="unpackProtoJson(run.outputsPacked)"
          />
        </div>
        <!-- Error (last run) -->
        <div v-else-if="run?.error != null" class="mx-auto mt-1 py-3">
          <h4 class="font-semibold">Error</h4>
          <RunError class="mt-2" :run="run" :error="run.error" />
        </div>
        <!-- No terminated last run yet -->
        <div v-else-if="variant != Variant.COMPACT" class="mx-auto mt-1 py-3">
          <h4 class="font-semibold">Outputs</h4>
          <!-- Placeholder -->
          <div class="mt-2 w-full">
            <span v-if="run != null">
              <IconInline :class="ACCENT_COLOR_BY_RUN_STATUS[run.status]" v-bind="ICON_BY_RUN_STATUS[run.status]" />
              <span class="ml-1.5" :class="ACCENT_COLOR_BY_RUN_STATUS[run.status]">
                {{ toCamelName(RunStatus, run.status) }}
              </span>
            </span>
            <span v-else>
              <i class="fas fa-circle-dot w-5 text-gray-400" />
              <span class="ml-1">Not yet run</span>
            </span>
          </div>
        </div>
        <!-- Divider -->
        <div class="mx-auto my-2 w-full"><div class="h-[1px] w-full min-w-fit bg-gray-200" /></div>
        <!-- Logs (last run) -->
        <div v-if="run?.logs != null && run.logs.length > 0" class="mt-1 flex flex-col gap-y-1 py-3">
          <h4 class="mb-2 font-semibold">Logs</h4>
          <div v-for="(log, i) in run.logs" :key="i" class="flex flex-row text-gray-900">
            <span class="mr-2 flex-shrink-0 text-gray-400">{{ tsToDt(log.createdAt!).toFormat("HH:mm:ss:SSS") }}</span>
            <pre v-if="log.textPlain" class="w-fit">{{ log.textPlain }}</pre>
            <Text v-else-if="log.text" :model-value="log.text" :variant="Variant.STEALTH" />
          </div>
        </div>
        <!-- Past runs -->
        <div class="mx-auto mt-1 py-3">
          <h4 class="font-semibold">Runs</h4>
          <Feed
            is-inline
            :value-packed="packProtoJson(packBuiltinObject(feedState))"
            :expansion="expansion"
            :focus="focus"
            @update:self="
              (update) => {
                if ('expansion' in update) {
                  const selfNode = spaceGraph.getOrError(self);
                  canvas.tx().update(selfNode, { expansion: update.expansion });
                }
                if ('valuePacked' in update) {
                  updateState(canvas.tx(), {
                    feed: {
                      ...unpackBuiltinObject(unpackProtoJson(update.valuePacked), ObjectType.FEED_VIEW_STATE),
                      filter: undefined,
                    },
                  });
                }
              }
            "
          />
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
