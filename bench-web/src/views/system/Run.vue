<script lang="ts" setup>
import { makeTypeInfo, TypeIdentity } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import {
  getInterruptBasePtr,
  getInterruptDurationString,
  getRunBasePtr,
  isRunnable,
  RunnableNode,
} from "@/language/session";
import {
  FieldType,
  InterruptData,
  InterruptStatus,
  NodeType,
  RunData,
  TypeInfoData,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { getInterruptActions, runtime } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
import { getNodeIcon, ICON_BY_INTERRUPT_TYPE, IconInline } from "@/ui/icon";
import { computedValue } from "@/utils/ref";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import CustomObject from "@/views/system/CustomObject.vue";
import { computed, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = 32;
const SECTION_HEADER_HEIGHT = 32;
const ROW_HEIGHT = 28;

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

// node
const nodePtr = computedValue(() => props.nodePtr);
const node = pkgGraph.getRef(nodePtr);

// run is the focused run if it contains this runnable
const run = computed(() => {
  if (nodePtr.value != null && runtime.focusedRun != null && runtime.focusedRunTree.hasBase(nodePtr.value)) {
    return runtime.focusedRun;
  } else {
    return null;
  }
});
const runBasePtr = computed(() => (run.value != null ? getRunBasePtr(run.value) : nodePtr.value));
const runTree = computed(() => runtime.focusedRunTree);

// run
const inputsPacked = useSubnodeProperty(NodeType.VIEW, ViewType.RUN, toRef(props, "subnodePacked"), "inputsPacked");
const inputType = computed(() =>
  runBasePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: runBasePtr.value, baseFieldType: FieldType.INPUT })
    : undefined,
);
const outputType = computed(() =>
  runBasePtr.value != null
    ? makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: runBasePtr.value, baseFieldType: FieldType.OUTPUT })
    : undefined,
);

const inputsRef: Ref<InstanceType<typeof CustomObject> | null> = ref(null);
const outputsRef: Ref<InstanceType<typeof CustomObject> | null> = ref(null);

// interrupts
type InterruptInfo = {
  base: RunnableNode;
  interrupt: InterruptData;
  inputType: TypeInfoData;
  outputType: TypeInfoData;
};
const interrupts = computed(() => {
  const interrupts: InterruptInfo[] = [];
  for (const interrupt of runTree.value.interrupts) {
    const base = runTree.value.getBase(getInterruptBasePtr(interrupt)!)!;
    const inputType = makeTypeInfo({
      kind: TypeKind.OBJECT,
      baseTypePtr: getInterruptBasePtr(interrupt)!,
      baseFieldType: FieldType.INPUT,
    });
    const outputType = makeTypeInfo({
      kind: TypeKind.OBJECT,
      baseTypePtr: getInterruptBasePtr(interrupt)!,
      baseFieldType: FieldType.OUTPUT,
    });
    interrupts.push({ base, interrupt, inputType, outputType });
  }
  return interrupts;
});

function start() {
  if (node.value == null || !isRunnable(node.value)) return;
  const run = runtime.start(node.value, { inputsPacked: inputsPacked.value as any, focus: true });
}

defineExpose<ViewExposed & { start: () => void; run: Ref<RunData | null> }>({ self, id, start, run });
</script>
<template>
  <div v-if="node" class="h-full w-full">
    <!-- TODO :UX: also turn this into collapsible sections like in Inspect & Hub (factor out Tabs & Sections?) -->
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
                { metatype: NodeType.VIEW, type: ViewType.RUN, subnode: { inputsPacked: value } },
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
          :value-type="inputType"
          is-inline
          :variant="Variant.STEALTH"
          :model-value="run.inputsPacked"
        />
        <span v-if="inputsRef?.fields.length == 0" class="text-gray-400">Nothing</span>
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
        <span v-if="outputsRef?.fields.length == 0" class="text-gray-400">Nothing</span>
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
        <RunTimeline :node-ptr="toNodeRef(run)" class="" />
      </div>
      <!-- Interrupts -->
      <div v-if="interrupts.length > 0" class="px-5">
        <div
          class="flex flex-row items-center"
          :style="{
            height: `${SECTION_HEADER_HEIGHT}px`,
          }"
        >
          <span class="font-semibold">Interruptions</span>
          <span v-if="interrupts.some((i) => i.interrupt.status == InterruptStatus.OPEN)" class="ml-1.5 text-gray-400">
            ({{ interrupts.filter((i) => i.interrupt.status == InterruptStatus.OPEN).length }} open)
          </span>
        </div>
        <div class="flex flex-col">
          <!-- Interrupt -->
          <div v-for="interrupt of interrupts" :key="interrupt.interrupt.id" class="">
            <!-- Interrupt Header -->
            <div
              class="flex flex-row items-center gap-x-2"
              :style="{
                height: `${ROW_HEIGHT}px`,
              }"
            >
              <!-- Highlight -->
              <IconInline
                v-tooltip="{ title: 'Interrupted', small: true, group: 'run.status' }"
                class="transition-colors duration-75"
                :class="interrupt.interrupt.status == InterruptStatus.OPEN ? 'text-pink-500' : 'text-gray-700'"
                v-bind="ICON_BY_INTERRUPT_TYPE[interrupt.interrupt.type]"
              />
              <!-- Base -->
              <span>{{ interrupt.base.name }}</span>
              <!-- Duration -->
              <span class="text-gray-400">{{ getInterruptDurationString(interrupt.interrupt, { minUnit: "s" }) }}</span>
              <!-- Meta/Controls -->
              <div class="ml-auto flex flex-row items-center gap-x-1">
                <!-- Actions -->
                <button
                  v-for="action in getInterruptActions(interrupt.interrupt)"
                  :key="action.title"
                  v-tooltip="{ title: action.title, small: true, group: 'run' }"
                  class="rounded px-1 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
                  @click="action.action()"
                >
                  <IconInline v-bind="action.icon" />
                </button>
              </div>
            </div>
            <!-- Interrupt Body -->
            <div v-if="interrupt.interrupt.status == InterruptStatus.OPEN">
              <CustomObject
                id="interrupt-outputs"
                class="w-full"
                :value-type="interrupt.outputType"
                is-inline
                is-input
                :variant="Variant.STEALTH"
                :model-value="interrupt.interrupt.outputsPacked"
                @update:model-value="
                  (value) => {
                    runTree.tx.update(interrupt.interrupt, { outputsPacked: value }, { debounce: 'short' });
                  }
                "
              />
            </div>
          </div>
        </div>
      </div>
      <!-- Events -->
      <!-- ... -->
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty state -->
  </div>
</template>
