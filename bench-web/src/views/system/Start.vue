<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import { useSubnodeProperty } from "@/language/node";
import { getRunBasePtr, isRunnable } from "@/language/session";
import { FieldType, NodeType, RunData, TypeKind, Variant, ViewData, ViewType } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
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

// run
const inputsPacked = useSubnodeProperty(NodeType.VIEW, ViewType.START, toRef(props, "subnodePacked"), "inputsPacked");
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

function start() {
  if (node.value == null || !isRunnable(node.value)) return;
  const run = runtime.start(node.value, { inputsPacked: inputsPacked.value as any, focus: true });
}

defineExpose<ViewExposed & { start: () => void; run: Ref<RunData | null> }>({ self, id, start, run });
</script>
<template>
  <div v-if="node" class="h-full w-full">
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
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty state -->
  </div>
</template>
