<template>
  <div>
    <VueFlow>
      <Background :variant="BackgroundVariant.Dots" pattern-color="#f8f8f8" />
      <template #node-custom="props">
        <FlowNodeDisplay
          :label="props.label"
          v-if="flowsStore.getFlowNode(flow, props.id)"
          :node="flowsStore.flowNode(flow, props.id)"
        >
        </FlowNodeDisplay>
      </template>
    </VueFlow>
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
import { Background, BackgroundVariant, useVueFlow, VueFlow } from "@braks/vue-flow";
import ELK from "elkjs";

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

const { setNodes, setEdges } = useVueFlow({});

// sync vue-flow state from flow
watch(
  () => props.flow.nodes,
  async () => {
    // update nodes
    // "real" function nodes from the flow
    const flowNodes = (props.flow.nodes || []).map((node) => ({
      id: node.id,
      label: node.name,
      type: "custom",
      real: true,
      width: 100,
      height: 50,
    }));
    // "virtual" inputs (artifacts or record inputs)
    const virtualInputNodes = [] as any[];

    const nodes = [...flowNodes, ...virtualInputNodes];

    // update edges
    const flowNodeEdges = (props.flow.node_edges || []).map((edge) => ({
      id: edge.id,
      source: edge.dependency,
      target: edge.dependent,
    }));
    const edges = [...flowNodeEdges];

    const positionedNodes = await layoutNodes(nodes, edges);
    console.log(positionedNodes);
    setNodes(positionedNodes);
    setEdges([...edges]);
  },
  { immediate: true }
);

async function layoutNodes(
  nodes: { id: string; width: number; height: number }[],
  edges: { id: string; source: string; target: string }[]
): Promise<{ id: string; position: { x: number; y: number } }[]> {
  const elk = new ELK();
  const elkNodes = nodes.map((n) => ({ id: n.id, width: n.width, height: n.height }));
  const elkEdges = edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] }));
  const elkGraph = {
    id: "root",
    children: elkNodes,
    edges: elkEdges,
  };

  // see elkjs docs at https://github.com/kieler/elkjs
  // see ELK layered options at https://www.eclipse.org/elk/reference/algorithms/org-eclipse-elk-layered.html
  const positionedElkGraph = await elk.layout(elkGraph, { logging: true, layoutOptions: { algorithm: "layered" } });
  console.log(elkGraph);
  function getNodePosition(id: string) {
    const node = positionedElkGraph.children?.find((n) => n.id == id);
    if (node == null) {
      throw new Error("could not find node with id: " + id);
    }
    // swap x/y for vertical layout
    return { x: node.y as number, y: node.x as number };
  }

  return nodes.map((n) => ({ ...n, position: getNodePosition(n.id) }));
}

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
  flowInputRecord,
  () => {
    console.log("update runtime data");
    // TODO @Broken: update runtime data with all virtual input nodes (record & artifact)
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
