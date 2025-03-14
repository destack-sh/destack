<script lang="ts" setup>
import { getBaseFromNode } from "@/language/core/const";
import { useSubnodeProperty } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { isRunnable, RunnableNode, VERB_BY_RUN_STATUS } from "@/language/runtime/run";
import { getTransactionOptionsForType } from "@/language/runtime/transaction";
import {
  ColorShade,
  FieldType,
  NodeType,
  RunData,
  RunStatus,
  RunStatusOptionInfo,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { CLEAR_RUN_ACTION, getInputType, getOutputType, runtime } from "@/runtime/runtime";
import { canvas, benchGraph } from "@/system/space";
import { IconInline, makeIcon } from "@/ui/icon";
import { getRunColorHex } from "@/ui/style";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate } from "@/utils/time";
import Error from "@/views/builtins/Error.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { ModelValueOptions, type ViewEmits, type ViewExpose } from "@/views/common";
import SomeObject from "@/views/objects/Object.vue";
import { computed, toRef, type Ref } from "vue";

const SECTION_HEADER_HEIGHT = 32;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "nodePtr" | "focus" | "size" | "subnodePacked">
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// node
const nodePtr = computedValue(() => props.nodePtr);
const node = benchGraph.getRef(nodePtr);
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
const runBase = benchGraph.getRef(runBasePtr);
const inputsPacked = useSubnodeProperty(NodeType.VIEW, ViewType.RUN, toRef(props, "subnodePacked"), "inputsPacked");
const resourcesPacked = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.RUN,
  toRef(props, "subnodePacked"),
  "resourcesPacked",
);
// schema
const fields = benchGraph.getChildrenRef(runBasePtr, NodeType.FIELD);
const hasOutputs = computed(() => fields.value.some((f) => f.type == FieldType.OUTPUT));
const variableType = computed(() =>
  runBasePtr.value != null
    ? makeType({
        kind: TypeKind.CUSTOM_OBJECT,
        baseTypePtr: runBasePtr.value,
        baseFieldTypes: [FieldType.RESOURCE],
      })
    : undefined,
);
const inputType = computed(() => (runBase.value != null ? getInputType(runBase.value as RunnableNode) : undefined));
const outputType = computed(() => (runBase.value != null ? getOutputType(runBase.value as RunnableNode) : undefined));

function start() {
  if (node.value == null || !isRunnable(node.value)) return;
  const run = runtime.start(node.value, {
    resourcesPacked: resourcesPacked.value as any,
    inputsPacked: inputsPacked.value as any,
    focus: true,
  });
}

defineExpose<ViewExpose & { start: () => void; run: Ref<RunData | null> }>({ self, id, start, run });
</script>
<template>
  <div v-if="node && nodeIsRunnable" class="h-full w-full">
    <!-- NOTE :UX: turn Run into collapsible sections like in Inspect & Hub (factor out Tabs & Sections?) -->
    <!-- New Run -->
    <div v-if="run == null" class="flex flex-col gap-y-1">
      <!-- Variables/Inputs -->
      <div v-for="fieldType in [FieldType.RESOURCE, FieldType.INPUT]" :key="fieldType" class="px-5">
        <SomeObject
          :id="`fields-${fieldType}`"
          class="w-full"
          :value-type="fieldType == FieldType.INPUT ? inputType : variableType"
          is-inline
          is-input
          is-minimal
          :model-value="fieldType == FieldType.INPUT ? inputsPacked : resourcesPacked"
          @update:model-value="
            (value: any, options?: ModelValueOptions) => {
              const subnode = { [fieldType == FieldType.INPUT ? 'inputsPacked' : 'resourcesPacked']: value };
              state.update(
                { metatype: NodeType.VIEW, type: ViewType.RUN, subnode },
                options?.field != null ? getTransactionOptionsForType(options.field) : { debounce: 'short' },
              );
            }
          "
        />
      </div>
    </div>
    <!-- Existing Run -->
    <div v-else class="flex flex-col gap-y-1">
      <!-- Banner -->
      <div
        class="group/banner mx-5 mb-2 flex flex-row items-center rounded border px-2.5 py-1.5 transition-colors duration-150"
        :style="{
          backgroundColor: getRunColorHex(run.status, ColorShade.S100),
          borderColor: getRunColorHex(run.status, ColorShade.S500),
        }"
      >
        <IconInline
          v-bind="makeIcon(RunStatusOptionInfo[run.status]!.icon!)"
          :style="{ color: getRunColorHex(run.status, ColorShade.S500) }"
          :class="[run.status == RunStatus.RUNNING ? 'animate-spin' : '']"
        />
        <div class="ml-1.5">
          <span class="font-medium">This Run {{ VERB_BY_RUN_STATUS[run.status] }}.</span>
          <span v-if="run.startedAt" class="text-gray-700"> It started {{ formatAbsoluteDate(run.startedAt) }}.</span>
        </div>
        <div class="ml-auto">
          <button
            v-tooltip="{ title: 'Clear this Run', small: true, group: 'run.header' }"
            class="rounded px-0.5 text-gray-400 transition-colors duration-150 hover:text-gray-700 group-hover/banner:opacity-100"
            @click="() => CLEAR_RUN_ACTION.action()"
          >
            <span class="fas fa-xmark" />
          </button>
        </div>
      </div>
      <!-- Variables/Inputs/Outputs -->
      <SomeObject
        v-if="run?.resourcesPacked != null"
        id="fields-variables"
        class="w-full px-5"
        :value-type="variableType"
        is-inline
        is-minimal
        :model-value="run?.resourcesPacked"
      />
      <SomeObject
        v-if="run?.inputsPacked != null"
        id="fields-input"
        class="w-full px-5"
        :value-type="inputType"
        is-inline
        is-minimal
        :model-value="run?.inputsPacked"
      />
      <!-- Line -->
      <div v-if="run?.inputsPacked || run?.outputsPacked" class="w-full py-2">
        <div class="mx-5 h-[1px] bg-gray-200" />
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
        <Error class="" :run="run" :error="run.error" />
      </div>
      <!-- Line -->
      <div v-if="run?.error" class="w-full py-1">
        <div class="mx-5 h-[1px] bg-gray-200" />
      </div>
      <!-- Timeline -->
      <div v-if="run != null" class="mb-5 px-5">
        <RunTimeline :graph="benchGraph" :node-ptr="toNodeRef(run)" class="" layout="linear" />
      </div>
    </div>
  </div>
  <div v-else class="h-full w-full px-5">
    <!-- Empty state -->
    <span class="text-gray-400">You can't run this.</span>
  </div>
</template>
