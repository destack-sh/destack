<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import { unpackProtoJson } from "@/proto/wiring";
import { FieldZone, NodeType, RunData, TypeKind, Variant, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useExistingConnection, useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas, pkgConnection } from "@/system/space";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import RunError from "@/views/builtins/RunError.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import ValueObject from "@/views/system/ValueObject.vue";
import { computed, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline" | "variant">
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
    computed(() => ({
      scope: PACKAGE_SCOPE.value,
      roots: [nodePtr.value!],
      isEnabled: nodePtr.value != null,
      ancestorTypes: [NodeType.RUN],
      descendantTypes: [NodeType.RUN],
    })),
  );
const run = runGraph.getRef(nodePtr.value, { ignoreAncestors: true }) as Ref<RunData | undefined>;
const basePtr = computed(() => run.value?.stepPtr ?? run.value?.blockPtr);

const { graph: pkgGraph } = useExistingConnection(basePtr);
const runnableNode = pkgGraph.getRef(basePtr, { ignoreAncestors: true });

const inputsPacked = computed(
  () => (run.value?.inputsPacked != null ? unpackProtoJson(run.value.inputsPacked) : {}) as Record<string, any>,
);
const inputType = computed(() =>
  makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: basePtr.value, baseFieldZone: FieldZone.INPUT }),
);
const outputsPacked = computed(
  () => (run.value?.outputsPacked != null ? unpackProtoJson(run.value.outputsPacked) : {}) as Record<string, any>,
);
const outputType = computed(() =>
  makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: basePtr.value, baseFieldZone: FieldZone.OUTPUT }),
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="run">
    <!-- NOTE :Incomplete :UX: Run View is currently only intended for inline display in Feed -->
    <!-- Header -->
    <!-- ... -->
    <!--  (also :Architecture views like Run should respond to their size) -->
    <!-- IO -->
    <div v-if="runnableNode">
      <!-- Inputs -->
      <ValueObject
        class="w-full py-2"
        :model-value="inputsPacked"
        :value-type="inputType"
        is-inline
        :variant="Variant.STEALTH"
      />
      <!-- Output -->
      <ValueObject
        class="mt-2 w-full py-2"
        :model-value="outputsPacked"
        :value-type="outputType"
        is-inline
        :variant="Variant.STEALTH"
      />
    </div>
    <Inaccessible v-else :node="basePtr" :connection="pkgConnection" />

    <!-- Error -->
    <RunError v-if="run?.error" class="mt-2" :run="run" :error="run.error" />

    <!-- Logs/Spans/Events/Attempts/Timeline/... -->
    <!-- ... -->
  </div>
  <Inaccessible v-else :node="nodePtr" :connection="runConnection" />
</template>
