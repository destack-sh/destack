<template>
  <div class="h-1/2 w-full">
    <VueFlow
      class="border-t border-b border-gray-300"
      :nodes-draggable="true"
      :pan-on-drag="true"
      :pan-on-scroll="false"
      :min-zoom="0.6"
      :max-zoom="1.0"
      :default-zoom="1.0"
    >
      <Background :variant="BackgroundVariant.Dots" pattern-color="#bbbbbb" :size="0.6" :gap="12" />
      <template #node-custom="props">
        <FlowNodeDisplay
          :label="props.label"
          v-if="flowsStore.getFlowNode(flow, props.id)"
          :node="flowsStore.flowNode(flow, props.id)"
        >
        </FlowNodeDisplay>
        <Handle type="target" class="bg-orange-500" :position="Position.Top" />
        <Handle type="source" class="w-10" :position="Position.Bottom" />
      </template>
    </VueFlow>
    <!-- <FlowNodeDisplay v-if="inputNode" label="Input" :node="inputNode">
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
    </div> -->
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
import { Background, BackgroundVariant, Handle, Position, useVueFlow, VueFlow } from "@braks/vue-flow";
import ELK from "elkjs";
import { computed, ref, watch, type Ref } from "vue";
import FlowNodeDisplay from "./FlowNodeDisplay.vue";

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

const { setNodes, setEdges, fitView } = useVueFlow({});

function buildVueFlowGraph() {
  // "real" function nodes from the flow
  const flowNodes = (props.flow.nodes || []).map((node) => {
    let nodeRect = { width: 200, height: 100 };
    return {
      id: node.id,
      label: node.name,
      type: "custom",
      real: true,
      width: nodeRect.width,
      height: nodeRect.height,
      sourcePosition: Position.Top,
      targetPosition: Position.Bottom,
    };
  });
  // "virtual" inputs (artifacts or record inputs)
  const virtualInputNodes = [] as any[];
  const nodes = [...flowNodes, ...virtualInputNodes];

  // update edges
  const flowNodeEdges = (props.flow.node_edges || []).map((edge) => ({
    id: edge.id,
    source: edge.dependency,
    target: edge.dependent,
    type: "smoothstep",
  }));
  const edges = [...flowNodeEdges];

  return { nodes, edges };
}

async function updateVueFlowGraph() {
  const { nodes, edges } = buildVueFlowGraph();
  const positionedNodes = await layoutNodes(nodes, edges);
  setNodes(positionedNodes);
  setEdges([...edges]);

  fitView.apply({ padding: 0.2 });
}

// sync vue-flow state from flow
watch(
  () => props.flow.nodes,
  async () => {
    await updateVueFlowGraph();
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
  const positionedElkGraph = await elk.layout(elkGraph, {
    logging: true,
    measureExecutionTime: true,
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "DOWN",
      "spacing.nodeNodeBetweenLayers": "80",
    },
  });
  console.log(positionedElkGraph);
  function getNodePosition(id: string) {
    const node = positionedElkGraph.children?.find((n) => n.id == id);
    if (node == null) {
      throw new Error("could not find node with id: " + id);
    }
    // swap x/y for vertical layout
    return { x: node.x as number, y: node.y as number };
  }

  return nodes.map((n) => ({ ...n, position: getNodePosition(n.id) }));
}

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

<style>
.vue-flow__handle {
  width: 8px;
  height: 8px;
  background-color: rgb(234, 88, 12);
}

.vue-flow__handle.source {
}

.vue-flow__handle.target {
}

.vue-flow__edge-path {
  stroke: rgb(234, 88, 12);
  stroke-width: 2;
}
</style>
