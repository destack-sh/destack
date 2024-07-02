<script lang="ts" setup>
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
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { makeExpression } from "@/system/expression";
import { RUNNABLE_BLOCK_TYPES } from "@/system/lang";
import { makeRun } from "@/system/session";
import { canvas, inspectionPtr } from "@/system/space";
import { unpackProtoJson } from "@/system/transaction";
import {
  getPropertyType,
  packBuiltinObject,
  packValue,
  packValueSimple,
  packValueSimpleStruct,
  propertyType,
  unpackBuiltinObject,
} from "@/system/value";
import { getFieldViews } from "@/system/view";
import { ScrollbarWidth } from "@/utils/layout";
import { computedValue } from "@/utils/ref";
import NodeCrumb from "@/views/builtins/NodeCrumb.vue";
import { DEFAULT_HEADER_HEIGHT, useViewState } from "@/views/canvas";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import Feed from "@/views/system/Feed.vue";
import { computed, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 800;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "nodePtr" | "valuePacked"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const focusPtr = computedValue(() => props.nodePtr ?? inspectionPtr.value);
const { graph: spaceGraph } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(focusPtr);
const { state, updateState, useStateProp } = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.START_VIEW_STATE,
  props,
  emit,
});
const feedState = computed(
  () =>
    ({
      // pre-filter to only runs of this node
      metatype: ObjectType.FEED_VIEW_STATE,
      nodeType: NodeType.RUN,
      filter: makeExpression({
        op: ExpressionOp.EQUALS,
        propertyPtr: propertyReference(ObjectType.RUN, RunProperty.blockPtr),
        valuePacked: packValueSimpleStruct(focusPtr.value, propertyType(ObjectType.RUN, RunProperty.blockPtr)),
      }),
      filterPills: state.value.feed?.filterPills,
    }) as FeedViewStateData,
);
const ancestors = pkgGraph.getAncestorsRef(focusPtr, { includeSelf: true });
const runnableNode: Ref<BlockData | StepData | null> = computed(() => {
  // for some reason this type checks but ancestors.find doesn't
  for (const ancestor of ancestors.value) {
    if (
      (isNode(ancestor, NodeType.BLOCK) && RUNNABLE_BLOCK_TYPES.includes(ancestor.type)) ||
      isNode(ancestor, NodeType.STEP)
    ) {
      return ancestor;
    }
  }
  return null;
});
// NOTE :UX :Architecture: run inputs should be recorded in view node state somehow
const inputsPacked: Ref<Record<string, any>> = ref({});
const inputFields = pkgGraph.getChildrenRef(runnableNode, NodeType.FIELD); // these need to be resolved later :TypeResolution
const inputViews = computed(() =>
  getFieldViews(inputFields.value, inputsPacked.value, pkgGraph, { zones: [FieldZone.INPUT], isInput: true }),
);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="runnableNode" class="h-full w-full bg-white">
    <!-- Header -->
    <div class="group mx-auto flex w-full flex-row items-center" :style="{ height: HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-1.5 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Runnable -->
        <NodeCrumb class="font-medium" :node="runnableNode" :connection="pkgConnection" />
        <!-- Controls -->
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <button
            :disabled="runnableNode == null"
            class="h-fit enabled:text-gray-700 enabled:hover:text-primary-900 disabled:text-gray-400"
            @click="
              () => {
                if (runnableNode == null) return;
                const run = makeRun(runnableNode, pkgGraph, { inputsPacked });
                pkgConnection.tx.create(run);
              }
            "
          >
            <i class="fas fa-play w-5 text-center" />
            <span class="ml-0.5">Start</span>
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
      <div class="mx-auto mt-1 px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <h4 class="font-semibold">Inputs</h4>
      </div>
      <ul class="flex flex-col gap-y-2.5 py-3">
        <!-- Property -->
        <li
          v-for="{ field, storageKey, viewType, viewProps, isFullWidth } of inputViews"
          :key="field.id"
          class="mx-auto w-full px-5"
          :class="[isFullWidth ? 'flex flex-col' : 'flex flex-row flex-wrap items-center gap-x-[10%]']"
          :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
        >
          <!-- Title & Controls -->
          <span class="w-[100px]">
            <span class="max-w-full truncate py-1 font-medium text-gray-700">{{ field.name }}</span>
          </span>
          <!-- Value -->
          <component
            :is="getViewComponent(viewType)"
            v-if="viewType != null && hasViewComponent(viewType)"
            :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
            :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
            v-bind="viewProps"
            :model-value="inputsPacked[storageKey]"
            @update:model-value="(value: any) => (inputsPacked[storageKey] = value)"
          />
          <div v-else class="ml-auto text-warning-600">
            {{ viewType != null ? ViewType[viewType] : "No View for Type" }}
          </div>
        </li>
        <!-- Empty -->
        <li v-if="inputViews.length === 0" class="mx-auto w-full px-5">
          <span class="text-gray-500">No Inputs</span>
        </li>
      </ul>
      <!-- Divider -->
      <div class="mx-auto my-2 w-full px-5" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <div class="h-[1px] w-full min-w-fit bg-gray-200" />
      </div>
      <!-- Feed -->
      <div class="mx-auto mt-1 px-5 py-3" :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }">
        <h4 class="font-semibold">Runs</h4>
        <Feed
          is-inline
          :value-packed="packBuiltinObject(feedState)"
          @update:self="
            (update) =>
              updateState({
                feed: {
                  ...unpackBuiltinObject(unpackProtoJson(update.valuePacked), ObjectType.FEED_VIEW_STATE),
                  filter: undefined,
                },
              })
          "
        />
      </div>
    </Scroll>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty state -->
    <span>
      <i class="fas fa-empty-set text-gray-500" />
      <span class="ml-1.5 text-gray-600">Select Node to Run</span>
    </span>
  </div>
</template>
