<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import { FieldType, NodeType, RunData, TypeKind, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useExistingConnection, useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunError from "@/views/builtins/RunError.vue";
import RunTimeline from "@/views/builtins/RunTimeline.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import CustomObject from "@/views/system/CustomObject.vue";
import { computed, toRef, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 1200;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "size" | "nodePtr" | "isInline" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr));

const { graph: runGraph, connection: runConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `log.${nodePtr.value?.id}` },
    computed(() => ({ scope: PACKAGE_SCOPE.value, roots: [nodePtr.value!], isEnabled: nodePtr.value != null })),
  );
const run = runGraph.getRef(nodePtr.value, { ignoreAncestors: true }) as Ref<RunData | undefined>;
const basePtr = computed(() => run.value?.stepPtr ?? run.value?.blockPtr);

const { graph: pkgGraph } = useExistingConnection(basePtr);
const runnableNode = pkgGraph.getRef(basePtr, { ignoreAncestors: true });

const inputsPacked = computed(
  () => (run.value?.inputsPacked != null ? run.value.inputsPacked : {}) as Record<string, any>,
);
const inputType = computed(() =>
  makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: basePtr.value, baseFieldType: FieldType.INPUT }),
);
const outputsPacked = computed(
  () => (run.value?.outputsPacked != null ? run.value.outputsPacked : {}) as Record<string, any>,
);
const outputType = computed(() =>
  makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: basePtr.value, baseFieldType: FieldType.OUTPUT }),
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="run != null">
    <!-- Header -->
    <div
      v-if="variant != Variant.COMPACT"
      class="group mx-auto flex w-full flex-row items-center"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center pl-2 pr-2.5"
        :style="{ minWidth: MIN_WIDTH + 'px' }"
      >
        <!-- Runnable -->
        <NodeReference class="font-medium" :node="run" :connection="runConnection" />
      </div>
    </div>

    <!-- Body -->
    <div
      class="flex flex-col gap-y-2"
      :class="variant != Variant.COMPACT ? 'mx-auto px-5 pb-5' : ''"
      :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
    >
      <!-- Inputs -->
      <div class="flex-1">
        <h4 class="font-semibold">Inputs</h4>
        <CustomObject
          class="w-full py-2"
          :value-type="inputType"
          is-inline
          :variant="Variant.STEALTH"
          :model-value="inputsPacked"
        />
      </div>
      <!-- Outputs (last run) -->
      <div v-if="outputsPacked != null" class="flex-1">
        <h4 class="font-semibold">Outputs</h4>
        <CustomObject
          class="w-full py-2"
          :value-type="outputType"
          is-inline
          :variant="Variant.STEALTH"
          :model-value="outputsPacked"
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
      <div v-if="nodePtr && run != null">
        <h4 class="font-semibold">Timeline</h4>
        <RunTimeline :node-ptr="nodePtr" class="mt-2" />
      </div>
    </div>
  </div>
  <Inaccessible v-else :node="nodePtr" :connection="runConnection" />
</template>
