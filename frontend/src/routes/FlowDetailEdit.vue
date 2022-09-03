<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl justify-between px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8">
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ props.flow }}</h1>
        <h3 class="text text-gray-700" v-if="flow">created {{ getTimeFromNowString(flow.created_at) }}</h3>
      </div>
      <div></div>
    </div>
    <FlowGraphInterface
      v-if="flow"
      class="pt-6 sm:gap-4"
      :flow="flow"
      editable
      show-minimap
      v-model:runtimeData="runtimeData"
      v-model:interactionData="interactionData"
      @add-node="promptAddNode"
      @edit-node="promptEditNode"
      @delete-node="promptDeleteNode"
      @add-node-edge="addNodeEdge"
      @submit-input="execute"
    />
    <div class="px-4 pt-6 sm:gap-4 sm:px-6 md:px-8" v-if="flow">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Execution history</h3>
      </div>
      <ExecutionsGrid :executions="executions" />
    </div>
  </Sidebar>
  <Slideover
    ref="editNodeSlideover"
    :title="interactionData.selectedNode == null ? 'Create flow node' : 'Edit flow node'"
    v-if="flow"
  >
    <FlowNodeConfigInterface
      :existing-node="interactionData.selectedNode || undefined"
      :flow="flow"
      v-model:runtimeData="runtimeData"
      v-model:interactionData="interactionData"
      @create="createFlowNode"
      @update="updateFlowNode"
    />
  </Slideover>
</template>
<script lang="ts" setup>
import FlowGraphInterface from "@/components/FlowGraphInterface.vue";
import Sidebar from "@/components/Sidebar.vue";
import Slideover from "@/components/container/Slideover.vue";
import { useFlowExecution } from "@/composables/useFlowExecution";
import { useTimeFromNow } from "@/composables/useNow";
import { useArtifactsStore, useFlowsStore } from "@/stores";
import {
  makeInteractionData,
  type ArtifactConnection,
  type FlowInteractionData,
  type FlowNode,
  type FlowRuntimeData,
  type FlowVersion,
} from "@/types";
import { splitNameVersion } from "@/utils/versioning";
import { computed, ref, toRef, watch, type Ref } from "vue";
import ExecutionsGrid from "../components/ExecutionsGrid.vue";
import FlowNodeConfigInterface from "../components/FlowNodeConfigInterface.vue";

const props = defineProps<{ flow: string; playground?: boolean; models?: string[] }>();

const flowsStore = useFlowsStore();
const flow = computed(() => flowsStore.flow(props.flow)?.latest_version || null);
const runtimeData: Ref<FlowRuntimeData> = ref({});
const interactionData: Ref<FlowInteractionData> = ref(makeInteractionData());

const artifactsStore = useArtifactsStore();
const { getTimeFromNowString } = useTimeFromNow();
const { executions, execute } = useFlowExecution(flow, runtimeData, ref(true));

const inputNode: Ref<FlowNode | null> = computed(
  () => flow.value?.nodes?.find((node) => node.name == "input-0") || null
);

function getFlow(): FlowVersion {
  if (!flow.value) {
    throw new Error("flow not initialized");
  }
  return flow.value;
}

function getNodeIndex(nodeName: string): number {
  return flow.value?.nodes?.filter((node) => node.name.startsWith(nodeName)).length || 0;
}

async function addModelsAndConnectInput(flow: FlowVersion, models: string[]) {
  if (inputNode.value == null) {
    throw new Error("input not initialized");
  }
  const modelNodes = await addModels(flow, models);
  await Promise.all(
    modelNodes.map((modelNode) =>
      flowsStore.createFlowNodeEdge(flow, {
        dependency: modelNode.id,
        dependent: (inputNode.value as FlowNode).id,
        connection_name_dependency: "*",
        connection_name_dependent: "*",
        connection_type: "input",
      })
    )
  );
}

async function addModels(flow: FlowVersion, models: string[]) {
  // TODO @Cleanup: guard against models already exist when user adds model?
  //  Currently it will just error because the model node name already exists.

  // resolve to actual models (versions) instances to get real version reference to use
  models = models.map((model) => {
    let [name, version] = splitNameVersion(model);
    // TODO @Feature handle tags other than HEAD in added models to flow
    if (version == "HEAD") {
      version = artifactsStore.artifact(name)?.latest_version?.version;
    }
    return `${name}@${version}`;
  });

  // create model nodes and connections to main input & models for models
  const modelNodes = await Promise.all(
    models
      .map((model) => `model-${splitNameVersion(model)[0]}`)
      .map((modelNodeName) =>
        flowsStore.createFlowNode(flow, {
          name: modelNodeName + "-" + getNodeIndex(modelNodeName),
          function_id: "bench.model",
        })
      )
  );

  // create connections to models
  await Promise.all(
    modelNodes.map((modelNode, i) =>
      flowsStore.createFlowArtifactEdge(flow, {
        dependent: modelNode.id,
        dependency: models[i],
        connection_type: "argument",
        connection_name: "model",
      })
    )
  );

  return modelNodes;
}

const editNodeSlideover = ref<InstanceType<typeof Slideover> | null>(null);

async function addNodeEdge(v: { source: FlowNode; sourcePort: string; target: FlowNode; targetPort: string }) {
  if (flow.value == null) {
    throw new Error("flow is not initialized");
  }
  await flowsStore.createFlowNodeEdge(flow.value, {
    dependency: v.source.id,
    dependent: v.target.id,
    connection_name_dependency: v.sourcePort,
    connection_name_dependent: v.targetPort,
    connection_type: "input",
  });
}

function promptAddNode(v: { inputNode?: FlowNode[]; outputNodes?: FlowNode[] }) {
  interactionData.value.selectedNode = null;
  editNodeSlideover.value?.show();
}

function promptEditNode(node: FlowNode) {
  interactionData.value.selectedNode = node;
  editNodeSlideover.value?.show();
}

async function promptDeleteNode(node: FlowNode) {
  if (flow.value == null) {
    throw new Error("flow is not initialized");
  }
  // TODO @Feature: prompt before delete
  await flowsStore.deleteFlowNode(flow.value, node);
}

async function createFlowNode(v: {
  node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
  connectedArtifacts: ArtifactConnection[];
  fromNodes?: FlowNode[];
  toNodes?: FlowNode[];
}) {
  const flow = getFlow();
  // create node
  const node = await flowsStore.createFlowNode(flow, v.node);

  // connect to artifacts (only for now)
  for (const connection of v.connectedArtifacts) {
    await flowsStore.createFlowArtifactEdge(flow, { dependent: node.id, ...connection });
  }

  // connect to current selection of input/output nodes (* connection only for now)
  for (const inputNode of v.fromNodes || []) {
    await flowsStore.connectFlowNode(flow, inputNode, {
      connection_type: "input",
      connection_name_dependency: "*",
      connection_name_dependent: "*",
      dependency: node.id,
    });
  }
  for (const dependentNode of v.toNodes || []) {
    await flowsStore.connectFlowNode(flow, dependentNode, {
      connection_type: "input",
      connection_name_dependency: "*",
      connection_name_dependent: "*",
      dependency: node.id,
    });
  }

  editNodeSlideover.value?.hide();
}

async function updateFlowNode(v: { node: FlowNode; connectedArtifacts: ArtifactConnection[] }) {
  const flow = getFlow();
  // update node
  await flowsStore.updateFlowNode(flow, v.node);

  await flowsStore.setFlowNodeArtifactConnections(flow, v.node, v.connectedArtifacts);

  editNodeSlideover.value?.hide();
}

// Automatically create new flow if in playground mode.
watch(
  [flow, toRef(props, "playground")],
  async () => {
    if (!props.playground) {
      return;
    }

    if (flow.value == null) {
      // create flow instance if needed
      let flowInstance = flowsStore.flow(props.flow);
      if (!flowInstance) {
        flowInstance = await flowsStore.createFlow({ name: props.flow });
      }

      // init playground
      const flow = await flowsStore.createFlowVersion(flowInstance, {
        name: "Initial commit",
        description: "Auto-generated.",
        parents: [],
      });

      await setupPlayground(flow, props.models || []);
    }
  },
  { immediate: true }
);

async function setupPlayground(flow: FlowVersion, models: string[]) {
  await addModels(flow, models);
}
</script>
