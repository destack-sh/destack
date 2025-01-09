<script lang="ts" setup>
import { getBaseFromNode, toCamelName } from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import { getInterruptDurationString, isRunnable, RunnableNode } from "@/language/session";
import { getTransactionOptionsForType } from "@/language/transaction";
import {
  FieldType,
  InterruptionData,
  InterruptionStatus,
  InterruptionType,
  NodeType,
  RunData,
  TypeData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { getInterruptActions, runtime } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
import { ICON_BY_INTERRUPTION_TYPE, IconInline } from "@/ui/icon";
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
    ? makeTypeInfo({
        kind: TypeKind.CUSTOM_OBJECT,
        baseTypePtr: runBasePtr.value,
        baseFieldTypes: [FieldType.VARIABLE],
      })
    : undefined,
);
const inputType = computed(() =>
  runBasePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.CUSTOM_OBJECT, baseTypePtr: runBasePtr.value, baseFieldTypes: [FieldType.INPUT] })
    : undefined,
);
const outputType = computed(() =>
  runBasePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.CUSTOM_OBJECT, baseTypePtr: runBasePtr.value, baseFieldTypes: [FieldType.OUTPUT] })
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
    const inputType = makeTypeInfo({
      kind: TypeKind.CUSTOM_OBJECT,
      baseTypePtr: getBaseFromNode(interrupt)!,
      baseFieldTypes: [FieldType.INPUT],
    });
    const outputType = makeTypeInfo({
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
      <!-- Variables/Inputs -->
      <div
        v-for="fieldType in [FieldType.VARIABLE, FieldType.INPUT].filter((ft) => hasRunObjectFields(ft))"
        :key="fieldType"
        class="px-5"
      >
        <h4
          class="flex flex-row items-center font-semibold"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span>{{ toCamelName(FieldType, fieldType) }}s</span>
        </h4>
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
    <div v-else class="flex flex-col gap-y-2">
      <!-- Variables/Inputs/Outputs -->
      <div
        v-for="fieldType in [FieldType.VARIABLE, FieldType.INPUT, FieldType.OUTPUT].filter((ft) =>
          hasRunObjectFields(ft),
        )"
        :key="fieldType"
        class="px-5"
      >
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">{{ toCamelName(FieldType, fieldType) }}s</span>
        </div>
        <SomeObject
          :id="`fields-${fieldType}`"
          class="w-full"
          :value-type="getRunObjectType(fieldType)"
          is-inline
          is-minimal
          :model-value="getRunObjectValue(fieldType)"
        />
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
        <RunTimeline :graph="pkgGraph" :node-ptr="toNodeRef(run)" class="" />
      </div>
      <!-- Interruptions -->
      <div v-if="interruptions.length > 0" class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Interruptions</span>
          <span
            v-if="interruptions.some((i) => i.interruption.status == InterruptionStatus.OPEN)"
            class="ml-1.5 text-gray-400"
          >
            ({{ interruptions.filter((i) => i.interruption.status == InterruptionStatus.OPEN).length }} open)
          </span>
        </div>
        <div class="flex flex-col gap-y-1.5">
          <!-- Interrupt -->
          <div v-for="interrupt of interruptions" :key="interrupt.interruption.id" class="">
            <!-- Interrupt Header -->
            <div class="flex flex-row items-center gap-x-1.5">
              <!-- Highlight -->
              <IconInline
                v-tooltip="{ title: 'Interrupted', small: true, group: 'run.status' }"
                class="w-5 text-center transition-colors duration-150"
                :class="interrupt.interruption.status == InterruptionStatus.OPEN ? 'text-pink-500' : 'text-gray-700'"
                v-bind="ICON_BY_INTERRUPTION_TYPE[interrupt.interruption.type]"
              />
              <!-- Node -->
              <span>{{ interrupt.base?.name ?? "???" }}</span>
              <!-- Duration -->
              <span class="ml-0.5 text-gray-400">
                {{ getInterruptDurationString(interrupt.interruption, { minUnit: "s" }) }}
              </span>
              <!-- Meta/Controls -->
              <div class="ml-auto flex flex-row items-center gap-x-1">
                <!-- Actions -->
              </div>
            </div>
            <!-- Interrupt Body -->
            <div v-if="interrupt.interruption.status == InterruptionStatus.OPEN">
              <!-- Interrupt Outputs -->
              <SomeObject
                id="interrupt-outputs"
                class="w-full"
                :value-type="interrupt.outputType"
                is-inline
                is-input
                is-minimal
                :model-value="interrupt.interruption.outputsPacked"
                @update:model-value="
                  (value) => {
                    runTree.tx.update(interrupt.interruption, { outputsPacked: value }, { debounce: 'short' });
                  }
                "
              />
              <!-- Actions -->
              <div class="ml-auto mt-1 flex flex-row justify-end gap-x-1 py-1">
                <button
                  v-for="action in getInterruptActions(interrupt.interruption)"
                  :key="action.title"
                  v-tooltip="{ title: action.title, small: true, group: 'run' }"
                  class="rounded px-1 py-0.5 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
                  @click="action.action()"
                >
                  <IconInline class="w-5 text-center" v-bind="action.icon" />
                  <span v-if="action.isPrimary" class="ml-1">{{ action.title }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      <!-- Events -->
      <!-- ... -->
    </div>
  </div>
  <div v-else class="h-full w-full px-5">
    <!-- Empty state -->
    <span class="text-gray-400">You can't run this.</span>
  </div>
</template>
