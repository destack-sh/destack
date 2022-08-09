<template>
  <div class="h-1/2 w-full">
    <VueFlow
      class="relative border-t border-b border-gray-300"
      :nodes-draggable="false"
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
          />
          <Handle
            v-for="sourcePort in props.data.targetPorts"
            :key="sourcePort"
            type="source"
            class="w-10"
            :position="Position.Bottom"
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
import type { FunctionSpec, RecordSpec, ValueType } from "@/types/spec";
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
  (e: "addEdge", value: { source: FlowNode; sourcePort: string; target: FlowNode; targetPort: string }): void;
  (e: "submitInput", value: { node: FlowNode; data: Record<string, any> }): void;
  (e: "update:runtimeData", value: FlowRuntimeData): void;
  (e: "update:interactionData", value: FlowInteractionData): void;
}>();

function addEdge(connection: Connection) {
  emit("addEdge", {
    source: flowsStore.flowNode(props.flow, connection.source),
    sourcePort: connection.sourceHandle || "*",
    target: flowsStore.flowNode(props.flow, connection.target),
    targetPort: connection.targetHandle || "*",
  });
}

const flowsStore = useFlowsStore();
const metaStore = useMetaStore();

const { setNodes, setEdges, fitView } = useVueFlow({});

function getFunctionSpec(nodeId: string): FunctionSpec {
  const node = flowsStore.flowNode(props.flow, nodeId);
  const functionSpec = metaStore.functionHandlersByName[node.function_id];
  if (functionSpec == null) {
    throw new Error("could not get function spec for handler: " + node.function_id);
  }
  // TODO @Feature: use current function spec rather than base
  return functionSpec.base_spec;
}

function buildVueFlowGraph() {
  // "real" function nodes from the flow
  const flowNodes = (props.flow.nodes || []).map((node) => {
    let nodeRect = { width: 200, height: 100 };
    const targetPorts = Object.keys(getFunctionSpec(node.id).input_spec);
    const sourcePorts = Object.keys(getFunctionSpec(node.id).output_spec);
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
  console.log(positionedNodes);
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

const artifactsStore = useArtifactsStore();
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
