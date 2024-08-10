<script lang="ts" setup>
import { makeExpression } from "@/language/expression";
import { makeTypeInfo, propertyType } from "@/language/field";
import { isRunnable } from "@/language/node";
import { makeRun } from "@/language/session";
import { packProtoJson, unpackProtoJson } from "@/language/transaction";
import { packBuiltinObject, packValueJson, unpackBuiltinObject } from "@/language/value";
import {
  BlockData,
  BoxData,
  ExpressionOp,
  FeedViewStateData,
  FieldZone,
  NodeType,
  ObjectType,
  Orientation,
  RunProperty,
  StepData,
  TypeKind,
  Variant,
  ViewData
} from "@/proto/wire";
import {
  propertyReference,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData
} from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { DEFAULT_HEADER_HEIGHT, useViewState } from "@/ui/canvas";
import { ScrollbarWidth } from "@/ui/layout";
import { toggleHelperViewPin } from "@/ui/view";
import { computedValue, mapRef } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Feed from "@/views/system/Feed.vue";
import ValueObject from "@/views/system/ValueObject.vue";
import { computed, toRef, type Ref } from "vue";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 800;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "nodePtr" | "valuePacked" | "focus" | "expansion"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const focusPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(focusPtr);
const { state, updateState, useStateProp } = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.START_VIEW_STATE,
  props,
  emit,
});
const feedState = computed(
  (): FeedViewStateData => ({
    // pre-filter to only runs of this node
    metatype: ObjectType.FEED_VIEW_STATE,
    nodeType: NodeType.RUN,
    filter: makeExpression({
      op: ExpressionOp.EQUALS,
      propertyPtr: propertyReference(ObjectType.RUN, RunProperty.blockPtr),
      valuePacked: packValueJson(runnablePtr.value, propertyType(ObjectType.RUN, RunProperty.blockPtr)),
    }),
    filterPills: state.value.feed?.filterPills ?? [],
  }),
);
const ancestors = pkgGraph.getAncestorsRef(focusPtr, { includeSelf: true });

// current runnable / inputs
const runnableNode: Ref<BlockData | StepData | null> = computed(() => {
  // for some reason this type checks but ancestors.find doesn't
  for (const ancestor of ancestors.value) {
    if (isRunnable(ancestor, pkgGraph)) {
      return ancestor as BlockData | StepData;
    }
  }
  return null;
});
const runnablePtr = computed(() => (runnableNode.value != null ? toPlainNodeRef(runnableNode.value) : null));
const inputsPacked: Ref<Record<string, any>> = mapRef(
  useStateProp("inputsPacked", undefined, { debounce: "short" }), // have to :DebounceNestedValue
  (packed) => (packed != null ? unpackProtoJson(packed) : {}) as Record<string, any>,
  (unpacked) => packProtoJson(unpacked),
);
const inputType = computed(() =>
  runnablePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: runnablePtr.value, baseFieldZone: FieldZone.INPUT })
    : undefined,
);

// last run

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="runnableNode" class="h-full w-full">
    <!-- Header -->
    <div class="group mx-auto flex w-full flex-row items-center" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Runnable -->
        <NodeReference class="font-medium" :node="runnableNode" :connection="pkgConnection" />
        <!-- Pin/unpin node -->
        <button
          v-tooltip="{ title: 'Pin node in view', small: true, placement: 'bottom' }"
          :disabled="nodePtr == null && runnableNode == null"
          class="ml-1.5 hover:text-primary-900"
          :class="nodePtr != null ? 'text-gray-700' : 'text-gray-400'"
          @click="toggleHelperViewPin(spaceConnection.tx, spaceGraph, { self, nodePtr: runnableNode })"
        >
          <i class="fas mr-1.5" :class="nodePtr == null ? 'fa-unlock' : 'fa-lock'" />
        </button>
        <!-- Meta & Controls -->
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <button
            :disabled="runnableNode == null"
            class="h-fit enabled:text-gray-900 enabled:hover:text-primary-900 disabled:text-gray-400"
            @click="
              () => {
                if (runnableNode == null) return;
                const run = makeRun(runnableNode, pkgGraph, { inputsPacked });
                pkgConnection.tx.create(run);
              }
            "
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
      <!-- Inputs -->
      <div class="mx-auto px-5 pt-1" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <h4 class="font-semibold">Inputs</h4>
        <ValueObject
          class="w-full py-2"
          :model-value="inputsPacked"
          :value-type="inputType"
          is-inline
          is-input
          :variant="Variant.STEALTH"
          @update:model-value="(value) => (inputsPacked = value)"
        />
      </div>
      <!-- Divider -->
      <div class="mx-auto my-2 w-full px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <div class="h-[1px] w-full min-w-fit bg-gray-200" />
      </div>
      <!-- Past runs -->
      <div class="mx-auto mt-1 px-5 py-3" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
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
                updateState({
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
