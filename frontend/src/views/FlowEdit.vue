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
      class="px-4 pt-6 sm:gap-4 sm:px-6 md:px-8"
      :flow="flow"
      editable
      v-model:runtimeData="runtimeData"
      v-model:interactionData="interactionData"
      @submit-input="execute"
    />
    <div class="px-4 pt-6 sm:gap-4 sm:px-6 md:px-8" v-if="flow">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Execution history</h3>
      </div>
      <ExecutionsGrid :executions="executions" />
    </div>
  </Sidebar>
  <PopupDialog title="Select artifact" ref="artifactSelectDialog">
    <ArtifactSelect static @select="(model) => addModelsAndConnectInput([`${model.name}@HEAD`])" />
  </PopupDialog>
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
      @create="createFlowNodeFromSelection"
      @update="updateFlowNodeInPlace"
    />
  </Slideover>
</template>
<script lang="ts" setup>
import ArtifactSelect from "@/components/ArtifactSelect.vue";
import FlowGraphInterface from "@/components/FlowGraphInterface.vue";
import PopupDialog from "@/components/PopupDialog.vue";
import Sidebar from "@/components/Sidebar.vue";
import Slideover from "@/components/Slideover.vue";
import { useFlow } from "@/composables/useFlow";
import { useFlowExecution } from "@/composables/useFlowExecution";
import { useTimeFromNow } from "@/composables/useNow";
import { useArtifactsStore, useFlowsStore } from "@/stores";
import {
  makeInteractionData,
  type ArtifactConnection,
  type FlowInteractionData,
  type FlowNode,
  type FlowRuntimeData,
} from "@/types";
import { splitNameVersion } from "@/utils/versioning";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";
import ExecutionsGrid from "../components/ExecutionsGrid.vue";
import FlowNodeConfigInterface from "../components/FlowNodeConfigInterface.vue";

const props = defineProps<{ flow: string; playground?: boolean; models?: string[] }>();

const flowsStore = useFlowsStore();
const flow = computed(() => flowsStore.flow(props.flow)?.latest_version || null);
const flowManager = useFlow(flow);
const runtimeData: Ref<FlowRuntimeData> = ref({});
const interactionData: Ref<FlowInteractionData> = ref(makeInteractionData());

const artifactsStore = useArtifactsStore();
const { getTimeFromNowString } = useTimeFromNow();
const { executions, execute } = useFlowExecution(flow, runtimeData, ref(true));

const inputNode: Ref<FlowNode | null> = computed(
  () => flow.value?.nodes?.find((node) => node.name == "input-0") || null
);

function getNodeIndex(nodeName: string): number {
  return flow.value?.nodes?.filter((node) => node.name.startsWith(nodeName)).length || 0;
}

async function addModelsAndConnectInput(models: string[]) {
  if (inputNode.value == null) {
    throw new Error("input not initialized");
  }

  const modelNodes = await addModels(models);
  await flowManager.connectFlowNodes(inputNode.value, modelNodes, "input");
}

async function addModels(models: string[]) {
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
        flowManager.createFlowNode({
          name: modelNodeName + "-" + getNodeIndex(modelNodeName),
          function_id: "bench.model",
        })
      )
  );

  // create connections to models
  await Promise.all(
    modelNodes.map((modelNode, i) =>
      flowManager.connectFlowNodeArtifact(modelNode, {
        dependency: models[i],
        connection_type: "argument",
        connection_name: "model",
      })
    )
  );

  return modelNodes;
}

const editNodeSlideover = ref(null);
async function createFlowNodeFromSelection(v: {
  node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
  connectedArtifacts: ArtifactConnection[];
}) {
  // create node
  const node = await flowManager.createFlowNode(v.node);

  // connect to artifacts (only for now)
  for (const connection of v.connectedArtifacts) {
    await flowManager.connectFlowNodeArtifact(node, connection);
  }

  // connect to current selection of input/output nodes (* connection only for now)
  for (const inputNode of interactionData.value.selectedFromNodes) {
    await flowManager.connectFlowNodes(inputNode, [node], "input");
  }
  await flowManager.connectFlowNodes(node, interactionData.value.selectedToNodes, "input");

  (editNodeSlideover.value as any).hide();
}

async function updateFlowNodeInPlace(v: { node: FlowNode; connectedArtifacts: ArtifactConnection[] }) {
  // update node
  await flowManager.updateFlowNode(v.node);

  await flowManager.setFlowNodeArtifactConnections(v.node, v.connectedArtifacts);

  (editNodeSlideover.value as any).hide();
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
      await flowsStore.createFlowVersion(flowInstance.name, {
        name: "Initial commit",
        description: "Auto-generated.",
        parents: [],
      });

      nextTick(() => setupPlayground(props.models || []));
    }
  },
  { immediate: true }
);

async function setupPlayground(models: string[]) {
  // create main input node
  const inputNode = await flowManager.createFlowNode({
    name: "input-0",
    function_id: "bench.identity",
  });
  const modelNodes = await addModels(models);
  await flowManager.connectFlowNodes(inputNode, modelNodes, "input");
}
</script>
