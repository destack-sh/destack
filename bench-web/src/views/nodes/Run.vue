<script lang="ts" setup>
import { getBaseFromNode } from "@/language/const";
import { makeType } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import { isRunnable, RunnableNode } from "@/language/session";
import { getTransactionOptionsForType } from "@/language/transaction";
import {
  BenchType,
  FieldType,
  InterruptionData,
  InterruptionType,
  NodeType,
  RunData,
  TypeData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
import { computedValue } from "@/utils/ref";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { ModelValueOptions, viewEmits, type ViewExposed } from "@/views/common";
import SomeObject from "@/views/objects/Object.vue";
import { computed, toRef, type Ref } from "vue";

const HEADER_HEIGHT = 32;
const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT = 28;
const INTERRUPT_TYPES = [InterruptionType.YIELD]; // NOTE :UX: make shown interrupt types configurable

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "nodePtr" | "focus" | "size" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// node
const nodePtr = computedValue(() => props.nodePtr);
const node = pkgGraph.getRef(nodePtr);
const nodeIsRunnable = computed(() => isRunnable(node.value));

// run is the focused run if it contains this runnable
const run = computed(() => {
  if (nodePtr.value != null && runtime.focusedRun != null && runtime.focusedRunTree.hasBase(nodePtr.value)) {
    return runtime.focusedRun;
  } else {
    return null;
  }
});
const runBasePtr = computed(() => (run.value != null ? getBaseFromNode(run.value) : nodePtr.value));
const runTree = computed(() => runtime.focusedRunTree);
const inputsPacked = useSubnodeProperty(NodeType.VIEW, ViewType.RUN, toRef(props, "subnodePacked"), "inputsPacked");
const variablesPacked = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.RUN,
  toRef(props, "subnodePacked"),
  "variablesPacked",
);
// schema
const fields = pkgGraph.getChildrenRef(runBasePtr, NodeType.FIELD);
const hasVariables = computed(() => fields.value.some((f) => f.type == FieldType.VARIABLE));
const hasInputs = computed(() => fields.value.some((f) => f.type == FieldType.INPUT));
const hasOutputs = computed(() => fields.value.some((f) => f.type == FieldType.OUTPUT));
const variableType = computed(() =>
  runBasePtr.value != null
    ? makeType({
        kind: TypeKind.CUSTOM_OBJECT,
        baseTypePtr: runBasePtr.value,
        baseFieldTypes: [FieldType.VARIABLE],
      })
    : undefined,
);
const inputType = computed(() =>
  runBasePtr.value != null
    ? makeType({
        kind: TypeKind.PARTIAL_OBJECT,
        benchType: BenchType.ACTION,
        baseTypePtr: runBasePtr.value,
        baseFieldTypes: [FieldType.INPUT],
        propertyFieldTypes: [FieldType.INPUT],
      })
    : undefined,
);
const outputType = computed(() =>
  runBasePtr.value != null
    ? makeType({ kind: TypeKind.CUSTOM_OBJECT, baseTypePtr: runBasePtr.value, baseFieldTypes: [FieldType.OUTPUT] })
    : undefined,
);
function getRunObjectType(fieldType: FieldType) {
  if (fieldType == FieldType.INPUT) return inputType.value;
  else if (fieldType == FieldType.OUTPUT) return outputType.value;
  else if (fieldType == FieldType.VARIABLE) return variableType.value;
  else return undefined;
}
function hasRunObjectFields(fieldType: FieldType) {
  if (fieldType == FieldType.INPUT) return hasInputs.value;
  else if (fieldType == FieldType.OUTPUT) return hasOutputs.value;
  else if (fieldType == FieldType.VARIABLE) return hasVariables.value;
  else return false;
}
function getRunObjectValue(fieldType: FieldType) {
  if (fieldType == FieldType.INPUT) return run.value?.inputsPacked;
  else if (fieldType == FieldType.OUTPUT) return run.value?.outputsPacked;
  else if (fieldType == FieldType.VARIABLE) return run.value?.variablesPacked;
  else return undefined;
}

// interruptions
type InterruptionInfo = {
  base: RunnableNode | null;
  interruption: InterruptionData;
  inputType: TypeData;
  outputType: TypeData;
};
const interruptions = computed(() => {
  const interruptions: InterruptionInfo[] = [];
  for (const interrupt of runTree.value.interruptions) {
    if (!INTERRUPT_TYPES.includes(interrupt.type)) continue;
    const base = runTree.value.getBase(getBaseFromNode(interrupt)!)!;
    const inputType = makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: getBaseFromNode(interrupt)!,
      baseFieldTypes: [FieldType.INPUT],
    });
    const outputType = makeType({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: getBaseFromNode(interrupt)!,
      baseFieldTypes: [FieldType.OUTPUT],
    });

    interruptions.push({ base, interruption: interrupt, inputType, outputType });
  }
  return interruptions;
});

function start() {
  if (node.value == null || !isRunnable(node.value)) return;
  const run = runtime.start(node.value, {
    variablesPacked: variablesPacked.value as any,
    inputsPacked: inputsPacked.value as any,
    focus: true,
  });
}

defineExpose<ViewExposed & { start: () => void; run: Ref<RunData | null> }>({ self, id, start, run });
</script>
<template>
  <div v-if="node && nodeIsRunnable" class="h-full w-full">
    <!-- TODO :UX: also turn this into collapsible sections like in Inspect & Hub (factor out Tabs & Sections?) -->
    <!-- New Run -->
    <div v-if="run == null" class="flex flex-col gap-y-2">
      <h4
        class="flex flex-row items-center font-semibold"
        :style="{
          height: `${SECTION_HEADER_HEIGHT}px`,
        }"
      >
        <span>Variables</span>
      </h4>
      <!-- Variables/Inputs -->
      <div
        v-for="fieldType in [FieldType.VARIABLE, FieldType.INPUT].filter((ft) => hasRunObjectFields(ft))"
        :key="fieldType"
        class="px-5"
      >
        <SomeObject
          :id="`fields-${fieldType}`"
          class="w-full"
          :value-type="getRunObjectType(fieldType)"
          is-inline
          is-input
          is-minimal
          :model-value="fieldType == FieldType.INPUT ? inputsPacked : variablesPacked"
          @update:model-value="
            (value, options?: ModelValueOptions) => {
              const subnode = { [fieldType == FieldType.INPUT ? 'inputsPacked' : 'variablesPacked']: value };
              state.update(
                { metatype: NodeType.VIEW, type: ViewType.RUN, subnode },
                options != null ? getTransactionOptionsForType(options?.field) : { debounce: 'short' },
              );
            }
          "
        />
      </div>
    </div>
    <!-- Existing Run -->
    <div v-else class="flex flex-col">
      <!-- Variables/Inputs/Outputs -->
      <SomeObject
        id="fields-variables"
        class="w-full px-5"
        :value-type="variableType"
        is-inline
        is-minimal
        :model-value="run?.variablesPacked"
      />
      <SomeObject
        id="fields-input"
        class="w-full px-5"
        :value-type="inputType"
        is-inline
        is-minimal
        :model-value="run?.inputsPacked"
      />
      <!-- Arrow -->
      <div v-if="hasOutputs" class="relative my-0.5 w-full text-center">
        <span class="fas fa-arrow-down text-gray-400" />
      </div>
      <SomeObject
        id="fields-output"
        class="w-full px-5"
        :value-type="outputType"
        is-inline
        is-minimal
        :model-value="run?.outputsPacked"
      />
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
        <!-- nocheckin: inline RunTimeline (as a list of Run/RunSpan?) -->
        <RunTimeline :graph="pkgGraph" :node-ptr="toNodeRef(run)" class="" />
      </div>
    </div>
  </div>
  <div v-else class="h-full w-full px-5">
    <!-- Empty state -->
    <span class="text-gray-400">You can't run this.</span>
  </div>
</template>
