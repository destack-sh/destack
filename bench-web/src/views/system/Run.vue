<script lang="ts" setup>
import { ViewData, NodeType, RunData, FieldZone, RunErrorKind, RunErrorType, Variant } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, pkgConnection } from "@/system/space";
import { computed, toRef, type Ref } from "vue";
import { useExistingConnection, useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { PACKAGE_SCOPE } from "@/system/client";
import { unpackProtoJson } from "@/system/transaction";
import { getFieldViews } from "@/system/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { toCamelName } from "@/system/lang";
import Text from "@/views/content/Text.vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { graph: runGraph } =
  props.preparedConnection ??
  useGetConnection(
    { name: `log.${props.nodePtr?.id}` },
    computed(() => ({
      scope: PACKAGE_SCOPE.value,
      roots: [props.nodePtr!],
      isEnabled: props.nodePtr != null,
      ancestorTypes: [NodeType.RUN],
      descendantTypes: [NodeType.RUN],
    })),
  );
const run = runGraph.getRef(props.nodePtr, { ignoreAncestors: true }) as Ref<RunData | undefined>;
const basePtr = computed(() => run.value?.stepPtr ?? run.value?.blockPtr);

const { graph: pkgGraph } = useExistingConnection(basePtr);
const runnableNode = pkgGraph.getRef(basePtr, { ignoreAncestors: true });

const inputsPacked = computed(
  () => (run.value?.inputsPacked != null ? unpackProtoJson(run.value.inputsPacked) : {}) as Record<string, any>,
);
const inputFields = pkgGraph.getChildrenRef(runnableNode, NodeType.FIELD); // resolve these later :TypeResolution
const inputViews = computed(() =>
  getFieldViews(inputFields.value, inputsPacked.value, pkgGraph, { zones: [FieldZone.INPUT], isInput: false }),
);
const outputsPacked = computed(
  () => (run.value?.outputsPacked != null ? unpackProtoJson(run.value.outputsPacked) : {}) as Record<string, any>,
);
const outputFields = pkgGraph.getChildrenRef(runnableNode, NodeType.FIELD); // resolve these later :TypeResolution
const outputViews = computed(() =>
  getFieldViews(outputFields.value, outputsPacked.value, pkgGraph, { zones: [FieldZone.OUTPUT], isInput: false }),
);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- NOTE :Incomplete :UX: Run View is currently only intended for inline display in Feed -->
    <!-- IO -->
    <div v-if="runnableNode">
      <!-- Inputs -->
      <div class="flex flex-col">
        <div
          v-for="{ field, viewType, viewProps, isFullWidth, storageKey } in inputViews"
          :key="field.id"
          class="py-1"
          :class="[isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row  items-center gap-x-2.5']"
        >
          <!-- Title -->
          <span class="font-medium">{{ field.name }}</span>
          <!-- Value -->
          <template v-if="viewType != null && hasViewComponent(viewType)">
            <!-- Old -->
            <component
              :is="getViewComponent(viewType)"
              v-if="inputsPacked[storageKey] != null"
              :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
              :style="{ width: isFullWidth ? '100%' : 'calc(45%)' }"
              v-bind="viewProps"
              :model-value="inputsPacked[storageKey]"
            />
            <div v-else class="w-full text-right"><span class="text-gray-400">Unset</span></div>
          </template>
          <div v-else class="flex flex-row items-center px-1 py-0.5 text-warning-600">
            <i class="fas fa-empty-set" />
            <span class="ml-1.5">No View for Type Type</span>
          </div>
        </div>
      </div>

      <!-- Output -->
      <!-- NOTE :Cleanup: Run inputs/outputs are basically duplicated -->
      <div v-if="!run?.error" class="mt-2 flex flex-col">
        <div
          v-for="{ field, viewType, viewProps, isFullWidth, storageKey } in outputViews"
          :key="field.id"
          class="py-1"
          :class="[isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row  items-center gap-x-2.5']"
        >
          <!-- Title -->
          <span class="font-medium">{{ field.name }}</span>
          <!-- Value -->
          <template v-if="viewType != null && hasViewComponent(viewType)">
            <!-- Old -->
            <component
              :is="getViewComponent(viewType)"
              v-if="outputsPacked[storageKey] != null"
              :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
              :style="{ width: isFullWidth ? '100%' : 'calc(45%)' }"
              v-bind="viewProps"
              :model-value="outputsPacked[storageKey]"
            />
            <div v-else class="w-full text-right"><span class="text-gray-400">Unset</span></div>
          </template>
          <div v-else class="flex flex-row items-center px-1 py-0.5 text-warning-600">
            <i class="fas fa-empty-set" />
            <span class="ml-1.5">No View for Type Type</span>
          </div>
        </div>
      </div>
    </div>
    <Inaccessible v-else :node="basePtr" :is-connected="pkgConnection.isConnected.value" />

    <!-- Error -->
    <div v-if="run?.error" class="mt-2">
      <div class="">
        <!-- Header -->
        <div class="flex flex-row">
          <!-- Title -->
          <span class="max-w-60 truncate font-medium">{{ run.error.title ?? "Error" }}</span>
          <!-- Details -->
          <div class="ml-auto pl-4 flex-shrink-0 text-gray-400">
            <span>{{ toCamelName(RunErrorKind, run.error.kind) }}</span>
            <template v-if="run.error.type"
              >/<span>{{ toCamelName(RunErrorType, run.error.type) }}</span></template
            >
          </div>
        </div>
        <!-- Text -->
        <div v-if="run.error.text" class="mt-1">
          <Text :model-value="run.error.text" :variant="Variant.STEALTH" />
        </div>
        <div v-else class="mt-1">
          <span class="text-gray-400">No Error Message</span>
        </div>
      </div>
    </div>

    <!-- Attempts/Timeline/Inner runs/etc. (see above) -->
    <!-- ... -->
  </div>
</template>
