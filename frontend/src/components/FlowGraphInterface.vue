<template>
  <div class="h-1/2 w-full">
    <VueFlow
      class="relative border-t border-b border-gray-300"
      :nodes-draggable="editable"
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
      <!-- Real nodes -->
      <template #node-custom-real="props">
        <template v-if="flowsStore.getFlowNode(flow, props.id)">
          <FlowNodeDisplay
            :node="props.data.node"
            :editable="editable"
            @edit="$emit('editNode', props.data.node)"
            @delete="$emit('deleteNode', props.data.node)"
          />
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
      <!-- Virtual input nodes -->
      <template #node-custom-virtual-input="props">
        <div
          v-if="props.data"
          class="h-full w-full divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
        >
          <div class="flex flex-row items-center justify-between px-4 py-2 sm:px-6">
            <div>
              <h3 class="text-md font-medium leading-6 text-gray-900">Input</h3>
              <h5 class="text-xs font-normal text-gray-500">
                {{ props.data.node.name }}
                <template v-if="props.data.name != '*'"> .{{ props.data.name }} </template>
              </h5>
            </div>
          </div>
          <div class="px-6 py-4 text-sm">
            <RecordForm
              :model-value="getRuntimeInputData(props.data.node.id, props.data.name)"
              @update:modelValue="(record) => setRuntimeInputData(props.data.node.id, props.data.name, record)"
              :spec="[props.data.spec]"
              @submit.prevent="
                $emit('submitInput', {
                  node: props.data.node,
                  data: getRuntimeInputData(props.data.node.id, props.data.name),
                })
              "
            />
          </div>
          <Handle type="source" :position="Position.Bottom" :is-valid-connection="isValidConnection" />
        </div>
      </template>
    </VueFlow>
  </div>
</template>
<script lang="ts" setup>
import RecordForm from "@/components/RecordForm.vue";
import { useFlowsStore, useMetaStore } from "@/stores";
import type { FlowInteractionData, FlowNode, FlowRuntimeData, FlowVersion } from "@/types/flows";
import { flowPorts, isPortSatisfied, isValidEdge, nodePorts } from "@/utils/flows";
import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  Handle,
  Position,
  useVueFlow,
  VueFlow,
  type Connection,
  type Edge,
  type Node,
} from "@braks/vue-flow";
import ELK from "elkjs";
import { watch } from "vue";
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

function getRuntimeInputData(nodeId: string, name: string): Record<string, any> {
  return props.runtimeData?.inputs?.[nodeId]?.[name] || {};
}

function setRuntimeInputData(nodeId: string, name: string, record: Record<string, any>) {
  const newRuntimeData = JSON.parse(JSON.stringify(props.runtimeData)) as FlowRuntimeData;
  if (newRuntimeData.inputs == null) {
    newRuntimeData.inputs = {};
  }
  if (newRuntimeData.inputs[nodeId] == null) {
    newRuntimeData.inputs[nodeId] = {};
  }
  newRuntimeData.inputs[nodeId][name] = record;

  emit("update:runtimeData", newRuntimeData);
}

const flowsStore = useFlowsStore();
const metaStore = useMetaStore();

function addEdge(connection: Connection) {
  emit("addNodeEdge", {
    source: flowsStore.flowNode(props.flow, connection.source),
    sourcePort: connection.sourceHandle || "*",
    target: flowsStore.flowNode(props.flow, connection.target),
    targetPort: connection.targetHandle || "*",
  });
}
const { fitView, setElements } = useVueFlow({});

function buildVueFlowGraph() {
  const nodes = [] as Omit<Node, "position">[];
  const edges = [] as Edge[];

  let nodeRect = { width: 200, height: 100 };
  // "real" function nodes from the flow
  const flowNodes = (props.flow.nodes || []).map((node) => {
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
        node,
        real: true,
        sourcePorts,
        targetPorts,
      },
    };
  });
  const flowNodeEdges = (props.flow.node_edges || []).map((edge) => ({
    id: edge.id,
    source: edge.dependency,
    target: edge.dependent,
    type: "smoothstep",
  }));
  nodes.push(...flowNodes);
  edges.push(...flowNodeEdges);

  // "virtual" inputs (artifacts or record inputs, only supporting record input for now)
  const virtualInputNodes = flowPorts(props.flow, metaStore.functionHandlersByName, "input")
    .filter((port) => !isPortSatisfied(props.flow, port))
    .map((port) => {
      const functionSpec = metaStore.functionHandlersByName[port.node.function_id];
      const inputSpec = functionSpec.base_spec.input_spec[port.name];
      return {
        id: `${port.node.id}.${port.type}s.${port.name}`,
        label: `${port.node.id}.${port.type}s.${port.name}`,
        type: "custom-virtual-input",
        width: nodeRect.width,
        height: nodeRect.height,
        sourcePosition: Position.Top,
        targetPosition: Position.Bottom,
        data: {
          node: port.node,
          name: port.name,
          spec: inputSpec,
        },
      };
    });
  const virtualInputEdges = virtualInputNodes.map((node) => ({
    id: node.id + "-edge",
    source: node.id,
    target: node.data.node.id,
    type: "smoothstep",
  }));
  nodes.push(...virtualInputNodes);
  edges.push(...virtualInputEdges);

  return { nodes, edges };
}

async function updateVueFlowGraph() {
  const { nodes, edges } = buildVueFlowGraph();
  const positionedNodes = await layoutNodes(nodes, edges);

  setElements([...positionedNodes, ...edges]);

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
      "spacing.nodeNodeBetweenLayers": "40",
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
