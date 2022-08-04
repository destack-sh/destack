<template>
  <div>
    <FlowNodeDisplay v-if="inputNode" label="Input" :node="inputNode">
      <RecordForm
        v-model="flowInputRecord"
        :spec="[flowInputSpec]"
        @submit.prevent="$emit('submitInput', { node: inputNode, data: flowInputRecord })"
      />
    </FlowNodeDisplay>

    <FlowNodeDisplay
      v-for="node in augmentNodes"
      label="Augment"
      :node="node"
      :key="node.id"
      @edit="$emit('editNode', node)"
      @delete="$emit('deleteNode', node)"
    >
      {{ node.function_id }}
    </FlowNodeDisplay>

    <div v-if="inputNode && editable" class="self-center px-4">
      <button
        type="button"
        class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
        @click="$emit('addNode', { inputNodes: [inputNode as FlowNode], outputNodes: modelNodes })"
      >
        Add node
      </button>
    </div>

    <!-- Select models -->
    <div class="flex flex-auto items-center gap-4">
      <FlowNodeDisplay
        class="flex-1"
        v-for="node in modelNodes"
        :key="node.id"
        label="Model"
        :node="node"
        @edit="$emit('editNode', node)"
        @delete="$emit('deleteNode', node)"
      >
        <router-link
          :to="'/models/' + modelForNode(node)?.artifact"
          class="font-medium text-gray-900 hover:text-gray-600"
        >
          {{ modelForNode(node)?.artifact }}
        </router-link>
        <p class="text-gray-500">
          {{ modelVersionForNode(node) }}
        </p>
      </FlowNodeDisplay>

      <div class="self-center px-4" v-if="editable">
        <button
          type="button"
          class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click="$emit('addNode', {})"
        >
          Add model
        </button>
      </div>
    </div>

    <div class="self-center px-4" v-if="editable">
      <button
        type="button"
        class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
        @click="$emit('addNode', { inputNodes: modelNodes })"
      >
        Add node
      </button>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { useFlowsStore } from "@/stores";
import { useArtifactsStore } from "@/stores/artifacts";
import { computedAsync } from "@/stores/utils";
import type { ArtifactVersion } from "@/types/artifacts";
import type { FlowInteractionData, FlowNode, FlowRuntimeData, FlowVersion } from "@/types/flows";
import type { RecordSpec, ValueType } from "@/types/spec";
import { mapNameVersion, toNameVersion } from "@/utils/versioning";
import { computed, ref, watch, type Ref } from "vue";
import FlowNodeDisplay from "./FlowNodeDisplay.vue";
import RecordForm from "./RecordForm.vue";

const props = defineProps<{
  flow: FlowVersion;
  runtimeData?: FlowRuntimeData;
  interactionData?: FlowInteractionData;
  editable?: boolean;
}>();
const emit = defineEmits<{
  (e: "addNode", value: { inputNodes?: FlowNode[]; outputNodes?: FlowNode[] }): void;
  (e: "editNode", value: FlowNode): void;
  (e: "deleteNode", value: FlowNode): void;
  (e: "submitInput", value: { node: FlowNode; data: Record<string, any> }): void;
  (e: "update:runtimeData", value: FlowRuntimeData): void;
  (e: "update:interactionData", value: FlowInteractionData): void;
}>();
const flowsStore = useFlowsStore();

const inputNode: Ref<FlowNode | null> = computed(
  () => props.flow.nodes?.find((node) => node.name == "input-0") || null
);
const augmentNodes: Ref<FlowNode[]> = computed(
  () => props.flow.nodes?.filter((node) => !node.name.startsWith("input-") && node.function_id != "bench.model") || []
);
const modelNodes: Ref<FlowNode[]> = computed(
  () => props.flow.nodes?.filter((node) => node.function_id == "bench.model") || []
);

// TODO @Feature: derive input spec from selected models (or any other specs)
//  Also allow multiple inputs.. this will soon be removed anyway.
const flowInputSpec: RecordSpec = {
  _type: "FieldSpec",
  name: "Common input spec",
  type: [
    {
      _type: "FieldSpec",
      name: "text",
      description: "any text",
      type: {
        _type: "ValueType",
        dtype: "string",
      } as ValueType,
    },
  ],
};
const flowInputRecord = ref({});

// update runtime data
watch(
  [flowInputRecord, inputNode],
  () => {
    var inputs = {};
    if (inputNode.value != null) {
      inputs = { [inputNode.value?.name]: flowInputRecord.value };
    }
    emit("update:runtimeData", { ...props.runtimeData, inputs });
  },
  { immediate: true }
);

// models referenced by modelNodes
const artifactsStore = useArtifactsStore();
const { result: usedModelsByNV } = computedAsync(async () => {
  const referencedModels: string[] = modelNodes.value
    .map((node) => flowsStore.artifactEdges(props.flow, node, "argument").pop()?.dependency)
    .filter((model) => model != undefined) as string[];
  const models = await Promise.all(
    referencedModels.map((artifact) => artifactsStore.getVersion(...mapNameVersion(artifact)))
  );
  const modelsByNV: Record<string, ArtifactVersion> = {};
  models.forEach((model) => (modelsByNV[toNameVersion(model)] = model));
  return modelsByNV;
});

function modelVersionForNode(modelNode: FlowNode): string | null {
  const model = modelForNode(modelNode);
  if (model == null) return null;
  if (artifactsStore.isHead(model)) {
    return "latest";
  } else {
    return model.name || model.version;
  }
}

function modelForNode(modelNode: FlowNode): ArtifactVersion | null {
  const referencedModel = flowsStore.artifactEdges(props.flow, modelNode, "argument").pop()?.dependency;
  return (usedModelsByNV.value || {})[referencedModel || ""];
}
</script>
