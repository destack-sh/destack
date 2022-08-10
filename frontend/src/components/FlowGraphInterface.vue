<template>
  <div class="h-1/2 w-full">
    <VueFlow
      class="relative border-t border-b border-gray-300"
      :nodes-draggable="true"
      :pan-on-drag="true"
      :pan-on-scroll="false"
      :min-zoom="0.6"
      :max-zoom="1.0"
      :default-zoom="1.0"
      :connect-on-click="editable"
      :nodes-connectable="editable"
      :edges-updatable="editable"
      :connection-mode="ConnectionMode.Strict"
      @connect="addEdge"
    >
      <Background :variant="BackgroundVariant.Dots" pattern-color="#bbbbbb" :size="0.6" :gap="12" />
      <template #node-custom-real="props">
        <template v-if="flowsStore.getFlowNode(flow, props.id)">
          <FlowNodeDisplay
            :label="props.label"
            :name="flowsStore.flowNode(flow, props.id).function_id"
            @edit="$emit('editNode', flowsStore.flowNode(flow, props.id))"
            @delete="$emit('deleteNode', flowsStore.flowNode(flow, props.id))"
          >
          </FlowNodeDisplay>
          <Handle
            v-for="targetPort in props.data.sourcePorts"
            :key="targetPort"
            type="target"
            :position="Position.Top"
            :is-valid-connection="isValidConnection"
          />
          <Handle
            v-for="sourcePort in props.data.targetPorts"
            :key="sourcePort"
            type="source"
            class="w-10"
            :position="Position.Bottom"
            :is-valid-connection="isValidConnection"
          />
        </template>
      </template>
      <template #node-custom-virtual="props">
        <FlowNodeDisplay label="Input" :name="props.label">
          <!--<RecordForm
            v-model="flowInputRecord"
            :spec="[flowInputSpec]"
            @submit.prevent="$emit('submitInput', { node: inputNode, data: flowInputRecord })"
          /> -->
        </FlowNodeDisplay>
      </template>
    </VueFlow>
  </div>
</template>
<script lang="ts" setup>
import { useFlowsStore, useMetaStore } from "@/stores";
import { useArtifactsStore } from "@/stores/artifacts";
import type { FlowInteractionData, FlowNode, FlowRuntimeData, FlowVersion } from "@/types/flows";
import type { RecordSpec, ValueType } from "@/types/spec";
import { isValidEdge, nodePorts } from "@/utils/flows";
import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  Handle,
  Position,
  useVueFlow,
  VueFlow,
  type Connection,
} from "@braks/vue-flow";
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
  (e: "addNodeEdge", value: { source: FlowNode; sourcePort: string; target: FlowNode; targetPort: string }): void;
  (e: "submitInput", value: { node: FlowNode; data: Record<string, any> }): void;
  (e: "update:runtimeData", value: FlowRuntimeData): void;
  (e: "update:interactionData", value: FlowInteractionData): void;
}>();

function addEdge(connection: Connection) {
  emit("addNodeEdge", {
    source: flowsStore.flowNode(props.flow, connection.source),
    sourcePort: connection.sourceHandle || "*",
    target: flowsStore.flowNode(props.flow, connection.target),
    targetPort: connection.targetHandle || "*",
  });
}

const flowsStore = useFlowsStore();
const metaStore = useMetaStore();

const { setNodes, setEdges, fitView } = useVueFlow({});

function buildVueFlowGraph() {
  // "real" function nodes from the flow
  const flowNodes = (props.flow.nodes || []).map((node) => {
    let nodeRect = { width: 200, height: 100 };
    const targetPorts = nodePorts(node, metaStore.functionHandlersByName, "input");
    const sourcePorts = nodePorts(node, metaStore.functionHandlersByName, "output");
    return {
      id: node.id,
      label: node.name,
      type: "custom-real",
      width: nodeRect.width,
      height: nodeRect.height,
      sourcePosition: Position.Top,
      targetPosition: Position.Bottom,
      data: {
        real: true,
        sourcePorts,
        targetPorts,
      },
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
  () => [props.flow.nodes, props.flow.node_edges, props.flow.artifact_edges],
  async () => {
    await updateVueFlowGraph();
  },
  { immediate: true, deep: true }
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

function isValidConnection(connection: Connection): boolean {
  return isValidEdge(props.flow, {
    dependent: connection.source,
    connection_name_dependent: connection.sourceHandle || "*",
    dependency: connection.target,
    connection_name_dependency: connection.targetHandle || "*",
    connection_type: "input",
  }).valid;
}
</script>

<style>
.vue-flow__node {
  cursor: default;
}

.vue-flow__handle {
  width: 8px;
  height: 8px;
}

.vue-flow__handle.source {
  background-color: rgb(234, 88, 12);
  border: none;
}

.vue-flow__handle.target {
  background-color: white;
  border-width: 2px;
  border-color: rgb(234, 88, 12);
}

.vue-flow__handle-connecting {
  border-color: red;
  cursor: not-allowed;
}

.vue-flow__handle-valid {
  width: 16px;
  height: 16px;
  border-color: green;
  cursor: pointer;
}

.vue-flow__edge-path {
  stroke: rgb(234, 88, 12);
  stroke-width: 2;
}

.vue-flow__connection-path {
  stroke: rgb(234, 88, 12);
  stroke-width: 2;
  stroke-dasharray: 5;
  -webkit-animation: dashdraw 0.5s linear infinite;
  animation: dashdraw 0.5s linear infinite;
}
</style>
