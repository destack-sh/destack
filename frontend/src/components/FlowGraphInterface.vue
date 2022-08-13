<template>
  <div class="h-2/3 w-full">
    <VueFlow
      class="relative border-t border-b border-gray-300"
      :nodes-draggable="false"
      :pan-on-drag="true"
      :pan-on-scroll="false"
      :min-zoom="0.5"
      :max-zoom="1.0"
      :default-zoom="1.0"
      :connect-on-click="editable"
      :nodes-connectable="editable"
      :edges-updatable="editable"
      :connection-mode="ConnectionMode.Strict"
      @connect="addConnection"
    >
      <MiniMap :height="100" v-if="showMinimap" node-color="rgb(249 115 22)" />
      <Background :variant="BackgroundVariant.Dots" pattern-color="#bbbbbb" :size="0.6" :gap="12" />
      <!-- Custom controls -->
      <!-- Not sure if controls should be here or in FlowEdit/wrapper -->
      <div class="absolute top-2 right-2 z-10" v-if="editable">
        <button
          type="button"
          class="inline-flex items-center rounded-full border border-transparent bg-orange-600 p-1 text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click="emit('addNode', {})"
        >
          <PlusSmIcon class="h-5 w-5" aria-hidden="true" />
        </button>
      </div>
      <!-- Real nodes -->
      <template #node-custom-real="props">
        <template v-if="props.data?.node">
          <FlowNodeInterface
            :flow="flow"
            :node="props.data.node"
            :editable="editable"
            @edit="$emit('editNode', props.data.node)"
            @delete="$emit('deleteNode', props.data.node)"
          />
          <Handle
            v-for="targetPort in props.data.targetPorts"
            :key="targetPort"
            :id="targetPort.name"
            type="target"
            :style="{
              left: (props.data as PositionedNodeData).targetPortsOffsets[targetPort.id].x + 'px',
              top: (props.data as PositionedNodeData).targetPortsOffsets[targetPort.id].y + 'px',
            }"
            :position="Position.Top"
            :is-valid-connection="isValidConnection"
          />
          <Handle
            v-for="sourcePort in props.data.sourcePorts"
            :key="sourcePort"
            :id="sourcePort.name"
            type="source"
            :style="{
              left: (props.data as PositionedNodeData).sourcePortsOffsets[sourcePort.id].x + 'px',
              top: (props.data as PositionedNodeData).sourcePortsOffsets[sourcePort.id].y + 'px',
            }"
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
          <div class="flex flex-row items-center justify-between px-4 py-2">
            <div class="flex flex-row items-center gap-2">
              <span class="ring-3 inline-flex rounded-lg bg-orange-50 p-2 text-orange-700 ring-white">
                <PencilIcon class="h-6 w-6" aria-hidden="true" />
              </span>
              <div>
                <h3 class="text-md font-medium leading-6 text-gray-900">Input</h3>
                <h5 class="text-xs font-normal text-gray-500">
                  {{ props.data.virtualForNode.name }}
                  <template v-if="props.data.virtualForPort.name != '*'"
                    >.{{ props.data.virtualForPort.name }}
                  </template>
                </h5>
              </div>
            </div>
          </div>
          <div class="px-6 py-4 text-sm">
            <RecordForm
              :model-value="getRuntimeInputData(props.data)"
              @update:modelValue="(record) => setRuntimeInputData(props.data, record)"
              :spec="[props.data.spec]"
              @submit.prevent="
                $emit('submitInput', {
                  node: props.data.virtualForNode,
                  data: getRuntimeInputData(props.data),
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
import FlowNodeInterface from "@/components/FlowNodeInterface.vue";
import RecordForm from "@/components/RecordForm.vue";
import { useMetaStore } from "@/stores";
import type {
  FlowInteractionData,
  FlowNode,
  FlowNodeEdge,
  FlowNodePort,
  FlowRuntimeData,
  FlowVersion,
} from "@/types/flows";
import type { FieldSpec } from "@/types/spec";
import { flowPorts, isPortSatisfied, isValidEdge, makeFlowNodePort, nodeEdgePorts, nodePorts } from "@/utils/flows";
import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  Handle,
  Position,
  MiniMap,
  useVueFlow,
  VueFlow,
  type Connection,
  type Edge,
  type Node,
} from "@braks/vue-flow";
import { PencilIcon } from "@heroicons/vue/outline";
import { PlusSmIcon } from "@heroicons/vue/solid";
import ELK, { type ElkEdge, type ElkNode } from "elkjs";
import { ref, watch, type Ref } from "vue";

const props = defineProps<{
  flow: FlowVersion;
  runtimeData?: FlowRuntimeData;
  interactionData?: FlowInteractionData;
  editable?: boolean;
  showMinimap?: boolean;
}>();
const emit = defineEmits<{
  (e: "addNode", value: { inputNodes?: FlowNode[]; outputNodes?: FlowNode[] }): void;
  (e: "editNode", value: FlowNode): void;
  (e: "deleteNode", value: FlowNode): void;
  (
    e: "addNodeEdge",
    value: { source: FlowNode; sourcePort: string; target: FlowNode; targetPort: string; type: "input" | "argument" }
  ): void;
  (e: "submitInput", value: { node: FlowNode; data: Record<string, any> }): void;
  (e: "update:runtimeData", value: FlowRuntimeData): void;
  (e: "update:interactionData", value: FlowInteractionData): void;
}>();

function getRuntimeInputData(data: VirtualNodeData): Record<string, any> {
  return props.runtimeData?.inputs?.[data.virtualForNode.id]?.[data.virtualForPort.name] || {};
}

function setRuntimeInputData(data: VirtualNodeData, record: Record<string, any>) {
  const newRuntimeData = JSON.parse(JSON.stringify(props.runtimeData)) as FlowRuntimeData;
  if (newRuntimeData.inputs == null) {
    newRuntimeData.inputs = {};
  }
  if (newRuntimeData.inputs[data.virtualForNode.id] == null) {
    newRuntimeData.inputs[data.virtualForNode.id] = {};
  }
  newRuntimeData.inputs[data.virtualForNode.id][data.virtualForPort.name] = record;

  console.log("update runtime data");
  emit("update:runtimeData", newRuntimeData);
}

const metaStore = useMetaStore();

const vueFlow = useVueFlow({});
const currentVueFlowNodes: Ref<Record<string, Node>> = ref({});
const currentVueFlowEdges: Ref<Record<string, Edge>> = ref({});

type NodeData = {
  real: boolean;
  sourcePorts: FlowNodePort[];
  targetPorts: FlowNodePort[];
};

type RealNodeData = NodeData & {
  real: true;
  node: FlowNode;
};

type VirtualNodeData = NodeData & {
  real: false;
  virtualForPort: FlowNodePort;
  virtualForNode: FlowNode;
  spec: FieldSpec;
};

type PositionedNodeData = {
  sourcePortsOffsets: Record<string, { x: number; y: number }>;
  targetPortsOffsets: Record<string, { x: number; y: number }>;
};

type EdgeData = {
  edge?: FlowNodeEdge;
  sourcePort: FlowNodePort;
  targetPort: FlowNodePort;
};

function buildVueFlowGraph() {
  const nodes = [] as Omit<Node, "position">[];
  const edges = [] as Edge[];

  // TODO @UI: vue flow node rect should be calculated dynamically based on element size
  // To do this, before becoming visible we could render all nodes with show=false
  //  and then update their size prior to layouting and showing everything.
  let nodeRect = { width: 250, height: 150 };
  // "real" function nodes from the flow
  const flowNodes = (props.flow.nodes || []).map((node) => {
    const targetPorts = nodePorts(node, metaStore.functionHandlersById, "input");
    const sourcePorts = nodePorts(node, metaStore.functionHandlersById, "output");
    return {
      id: node.id,
      label: node.name,
      type: "custom-real",
      width: nodeRect.width,
      height: nodeRect.height,
      data: {
        node,
        real: true,
        sourcePorts,
        targetPorts,
      } as RealNodeData,
    };
  });
  const flowNodeEdges = (props.flow.node_edges || []).map((edge) => {
    const ports = nodeEdgePorts(props.flow, edge);
    return {
      id: edge.id,
      source: edge.dependency,
      sourceHandle: edge.connection_name_dependency,
      target: edge.dependent,
      targetHandle: edge.connection_name_dependent,
      type: "smoothstep",
      data: {
        edge,
        type: edge.connection_type,
        sourcePort: ports.source,
        targetPort: ports.target,
      } as EdgeData,
    };
  });
  nodes.push(...flowNodes);
  edges.push(...flowNodeEdges);

  // "virtual" inputs (artifacts or record inputs, only supporting record input for now)
  const virtualInputNodes = flowPorts(props.flow, metaStore.functionHandlersById, "input")
    .filter((port) => !isPortSatisfied(props.flow, port))
    .map((port) => {
      const functionSpec = metaStore.functionHandlersById[port.node.function_id];
      const inputSpec = functionSpec.base_spec.input_spec[port.name];
      return {
        id: port.id + "-virtual-input-node",
        label: port.id,
        type: "custom-virtual-input",
        width: nodeRect.width,
        height: nodeRect.height,
        data: {
          real: false,
          virtualForPort: port,
          virtualForNode: port.node,
          sourcePorts: [makeFlowNodePort(port.name, { ...port.node, id: port.id + "-virtual-input-node" }, "output")],
          targetPorts: [],
          spec: inputSpec,
        } as VirtualNodeData,
      };
    });
  const virtualInputEdges = virtualInputNodes.map((node) => ({
    id: node.id + "-edge",
    source: node.id,
    target: node.data.virtualForNode.id,
    targetHandle: node.data.virtualForPort.name,
    type: "smoothstep",
    data: {
      type: "input",
      sourcePort: node.data.sourcePorts[0],
      targetPort: node.data.virtualForPort,
    } as EdgeData,
  }));
  nodes.push(...virtualInputNodes);
  edges.push(...virtualInputEdges);

  return { nodes, edges };
}

async function updateVueFlowGraph() {
  const newGraph = buildVueFlowGraph();
  const positioned = await elkLayout(newGraph.nodes, newGraph.edges);
  const vueFlowNodes: Record<string, Node> = {};
  for (const node of positioned.nodes) {
    vueFlowNodes[node.id] = node;
  }
  const vueFlowEdges: Record<string, Edge> = {};
  for (const edge of newGraph.edges) {
    vueFlowEdges[edge.id] = edge;
  }

  // TODO @UI: transition update elements and viewport smoothly
  vueFlow.setElements([...positioned.nodes, ...positioned.edges]);
  vueFlow.fitView({ padding: 0.2 });

  // update local copy of current elements
  currentVueFlowNodes.value = vueFlowNodes;
  currentVueFlowEdges.value = vueFlowEdges;
}

// sync vue-flow state from flow
watch(
  () => [props.flow.nodes, props.flow.node_edges, props.flow.artifact_edges],
  async () => {
    await updateVueFlowGraph();
  },
  { immediate: true, deep: true }
);

async function elkLayout(nodes: Omit<Node, "position">[], edges: Edge[]): Promise<{ nodes: Node[]; edges: Edge[] }> {
  const elk = new ELK();
  const elkNodes: Array<ElkNode> = nodes.map((n) => {
    const elkPorts = [];
    const nodeData = n.data as NodeData;
    elkPorts.push(
      ...nodeData.targetPorts.map((port) => ({
        id: port.id,
        layoutOptions: {
          "port.side": "NORTH",
        },
      })),
      ...nodeData.sourcePorts.map((port) => ({
        id: port.id,
        layoutOptions: {
          "port.side": "SOUTH",
        },
      }))
    );

    return {
      id: n.id,
      width: n.width,
      height: n.height,
      ports: elkPorts,
      layoutOptions: {
        portConstraints: "FIXED_SIDE",
      },
    } as ElkNode;
  });
  const elkEdges: Array<ElkEdge> = edges.map((e) => ({
    id: e.id,
    source: e.source,
    sourcePort: (e.data as EdgeData).sourcePort.id,
    target: e.target,
    targetPort: (e.data as EdgeData).targetPort.id,
  }));
  const elkGraph = {
    id: "root",
    children: elkNodes,
    edges: elkEdges,
  };

  // see elkjs docs at https://github.com/kieler/elkjs
  // see ELK layered options at https://www.eclipse.org/elk/reference/algorithms/org-eclipse-elk-layered.html
  const positionedElkGraph = await elk.layout(elkGraph as ElkNode, {
    logging: true,
    measureExecutionTime: true,
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "DOWN",
      "spacing.nodeNodeBetweenLayers": "40",
      "spacing.portPort": "20",
    },
  });

  // map nodes to positions
  const positionedNodes = nodes.map((node) => {
    const elkNode = positionedElkGraph.children?.find((n) => n.id == node.id);
    if (elkNode == null) {
      throw new Error("could not find node in positioned ELK graph: " + node.id);
    }

    const position = { x: elkNode.x as number, y: elkNode.y as number };
    const sourcePortsOffsets = {} as Record<string, { x: number; y: number }>;
    const targetPortsOffsets = {} as Record<string, { x: number; y: number }>;

    elkNode.ports?.forEach((elkPort) => {
      const sourcePort = (node.data as NodeData).sourcePorts.find((p) => p.id == elkPort.id);
      if (sourcePort != null) {
        sourcePortsOffsets[sourcePort.id] = { x: elkPort.x as number, y: elkPort.y as number };
        return;
      }
      const targetPort = (node.data as NodeData).targetPorts.find((p) => p.id == elkPort.id);
      if (targetPort != null) {
        targetPortsOffsets[targetPort.id] = { x: elkPort.x as number, y: elkPort.y as number };
        return;
      }
      throw new Error("could not find port in positioned ELK graph: " + elkPort.id);
    });

    const positionedNodeData = { ...node.data, sourcePortsOffsets, targetPortsOffsets } as PositionedNodeData;
    return { ...node, position, data: positionedNodeData };
  });

  return { nodes: positionedNodes, edges: edges };
}

function addConnection(connection: Connection) {
  const sourceNode = currentVueFlowNodes.value[connection.source];
  const targetNode = currentVueFlowNodes.value[connection.target];

  if (sourceNode.data.real) {
    emit("addNodeEdge", {
      source: (sourceNode.data as RealNodeData).node,
      sourcePort: connection.sourceHandle || "*",
      target: (targetNode.data as RealNodeData).node,
      targetPort: connection.targetHandle || "*",
      type: "input",
    });
  } else {
    // TODO @Feature: connect virtual input record to multiple ports (manually)
    console.warn("connecting virtual input record manually is not yet supported");
  }
}

function isValidConnection(connection: Connection): boolean {
  const sourceNode = currentVueFlowNodes.value[connection.source];
  if (sourceNode.data.real) {
    return isValidEdge(props.flow, {
      dependent: connection.source,
      connection_name_dependent: connection.sourceHandle || "*",
      dependency: connection.target,
      connection_name_dependency: connection.targetHandle || "*",
      connection_type: "input",
    }).valid;
  } else {
    // connecting virtual input nodes manually is not yet supported
    return false;
  }
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
