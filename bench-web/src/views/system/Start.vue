<script lang="ts" setup>
import { makeExpression } from "@/language/expression";
import { makeTypeInfo } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import { isRunnable } from "@/language/session";
import {
  ExpressionType,
  FeedViewData,
  FieldType,
  NodeType,
  ObjectType,
  RunProperty,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import {
  propertyReference,
  toNodeRef,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { runtime } from "@/system/runtime";
import { canvas, inspectionPtr, pkgGraph, space, spaceConnection } from "@/system/space";
import { computedValue } from "@/utils/ref";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import CustomObject from "@/views/system/CustomObject.vue";
import { computed, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = 32;
const SECTION_HEADER_HEIGHT = 32;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "nodePtr" | "focus" | "size" | "variant" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));

// run is the focused
const run = computed(() => {
  if (runtime.focusedRun != null && runtime.focusedRunBase?.ck == nodePtr.value?.ck) {
    return runtime.focusedRun;
  } else {
    return null;
  }
});
const feed = computed((): FeedViewData => {
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
  return {
    // pre-filter to only runs of this node
    queryNodeType: NodeType.RUN,
    filter: makeExpression({ type: ExpressionType.AND, clauses }),
    filterPills: [],
  };
});

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
  spaceConnection.tx.update(space.value!, { runPtr: toNodeRef(run) });
}

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="node" class="h-full w-full">
    <!-- Controls -->
    <div
      class="mx-5 flex flex-row items-center"
      :style="{
        height: `${HEADER_HEIGHT}px`,
      }"
    >
      <button @click="createRun">start</button>
      <button @click="() => spaceConnection.tx.update(space!, { runPtr: undefined })">clear</button>
    </div>
    <!-- New Run -->
    <div v-if="run == null" class="flex flex-col gap-y-2">
      <!-- Inputs -->
      <div class="px-5">
        <h4
          class="flex flex-row items-center font-semibold"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span>Inputs</span>
        </h4>
        <CustomObject
          id="inputs"
          ref="inputsRef"
          class="w-full"
          :value-type="inputType"
          is-inline
          is-input
          :variant="Variant.STEALTH"
          :model-value="inputsPacked"
          @update:model-value="
            (value) => {
              state.update(
                { metatype: NodeType.VIEW, type: ViewType.START, subnode: { inputsPacked: value } },
                { debounce: 'short' },
              );
            }
          "
        />
        <span v-if="inputsRef?.fields.length == 0" class="text-gray-400">No inputs</span>
      </div>
    </div>
    <!-- Existing Run -->
    <div v-else class="flex flex-col gap-y-2">
      <!-- Ancestor runs? -->
      <div class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Inputs</span>
        </div>
        <CustomObject
          id="inputs"
          ref="inputsRef"
          class="w-full"
          :value-type="outputType"
          is-inline
          :variant="Variant.STEALTH"
          :model-value="run.inputsPacked"
        />
        <span v-if="inputsRef?.fields.length == 0" class="text-gray-400">No inputs</span>
      </div>
      <!-- Outputs (last run) -->
      <div v-if="run?.outputsPacked != null" class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Outputs</span>
        </div>
        <CustomObject
          id="outputs"
          ref="outputsRef"
          class="w-full"
          :value-type="outputType"
          is-inline
          :variant="Variant.STEALTH"
          :model-value="run.outputsPacked"
        />
        <span v-if="outputsRef?.fields.length == 0" class="text-gray-400">No outputs</span>
      </div>
      <!-- Error -->
      <div v-if="run?.error != null" class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Error</span>
        </div>
        <RunError class="" :run="run" :error="run.error" />
      </div>
      <!-- Timeline -->
      <div v-if="run != null" class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Timeline</span>
        </div>
        <RunTimeline :node-ptr="toPlainNodeRef(run)" class="" />
      </div>
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty state -->
  </div>
</template>
