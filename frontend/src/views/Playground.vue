FlowExecutionPlan
<template>
  <Sidebar>
    <div
      class="mx-auto max-w-7xl justify-between px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8"
    >
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ flow?.flow }}</h1>
        <h3 class="text text-gray-700" v-if="flow">
          created {{ getTimeFromNowString(flow.created_at) }}
        </h3>
      </div>
      <div class="flex items-baseline gap-4">
        <fieldset class="space-y-5">
          <div class="relative flex items-start">
            <div class="flex h-5 items-center">
              <input
                v-model="poll"
                id="poll"
                aria-describedby="poll-description"
                name="poll"
                type="checkbox"
                class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-500"
              />
            </div>
            <div class="ml-3 text-sm">
              <label for="poll" class="font-medium text-gray-700">Poll</label>
            </div>
          </div>
        </fieldset>
        <button
          type="submit"
          class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
          @click.prevent="execute"
          :disabled="!canExecute"
        >
          Run
        </button>
        <button
          class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
          @click="initNewPlayground"
        >
          New
        </button>
      </div>
    </div>
    <div class="mx-auto flex max-w-7xl flex-col gap-2 py-4 px-4 sm:px-6 md:px-8">
      <FlowNodeInterface v-if="inputNode" label="Input" :node="inputNode">
        <RecordForm v-model="flowInputRecord" :spec="flowInputSpec" @submit.prevent="execute" />
      </FlowNodeInterface>

      <FlowNodeInterface
        v-for="node in augmentNodes"
        label="Augment"
        :node="node"
        :key="node.id"
        @edit="promptEditNode(node)"
        @delete="deleteFlowNode(node)"
      >
        {{ node.function_id }}
      </FlowNodeInterface>

      <div v-if="inputNode" class="self-center px-4">
        <button
          type="button"
          class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click="promptAddNode([inputNode], modelNodes)"
        >
          Add node
        </button>
      </div>

      <!-- Select models -->
      <div class="flex flex-auto items-center gap-4">
        <FlowNodeInterface
          class="flex-1"
          v-for="node in modelNodes"
          :key="node.id"
          label="Model"
          :node="node"
          @edit="promptEditNode(node)"
          @delete="deleteFlowNode(node)"
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
        </FlowNodeInterface>

        <div class="self-center px-4">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click="promptAddModel"
          >
            Add model
          </button>
        </div>
      </div>

      <div class="self-center px-4">
        <button
          type="button"
          class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click="promptAddNode(modelNodes, [])"
        >
          Add node
        </button>
      </div>

      <!-- Executions & output -->
      <div class="pt-6">
        <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
          <h3 class="text-lg font-medium leading-6 text-gray-900">Execution history</h3>
        </div>
        <ExecutionsGrid :executions="executions" />
      </div>
    </div>
  </Sidebar>
  <ArtifactSelect
    ref="artifactSelect"
    @select="(model) => addModelsAndConnectInput([`${model.name}@HEAD`])"
  />
  <Slideover
    ref="editNodeSlideover"
    :title="selectedNode == null ? 'Create flow node' : 'Edit flow node'"
    v-if="flow"
  >
    <FlowNodeConfigInterface
      :existing-node="selectedNode || undefined"
      :flow="flow"
      @create="createFlowNodeFromSelection"
      @update="updateFlowNodeInPlace"
    />
  </Slideover>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import ArtifactSelect from "@/components/ArtifactSelect.vue";
import ExecutionsGrid from "@/components/ExecutionsGrid.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import Slideover from "@/components/Slideover.vue";
import { useFlow } from "@/composables/useFlow";
import { computedAsync, useArtifactsStore, useFlowsStore } from "@/stores";
import {
  getAllConnectedArtifacts,
  isTerminal,
  type ArtifactVersion,
  type Execution,
  type ExecutionArtifactConnection,
  type FlowExecutionPlan,
  type FlowNode,
  type FlowNodeExecutionArgument,
  type FlowVersion,
  type LimitPaginatedResult,
  type RecordSpec,
  type ValueType,
} from "@/types";
import { mapNameVersion, splitNameVersion, toNameVersion } from "@/utils/versioning";
import { DateTime, Duration } from "luxon";
import { computed, onBeforeMount, onBeforeUnmount, ref, watch, type PropType, type Ref } from "vue";

import FlowNodeConfigInterface from "@/components/FlowNodeConfigInterface.vue";
import FlowNodeInterface from "@/components/FlowNodeInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { getRandomName } from "@/composables/useRandomName";

const props = defineProps({ models: { type: Array as PropType<Array<string>>, required: false } });

const flow: Ref<FlowVersion | null> = ref(null);
const {
  createFlowNode,
  updateFlowNode,
  deleteFlowNode,
  getFlowNode,
  connectFlowNodes,
  connectFlowNodeArtifact,
  artifactEdges,
} = useFlow(flow);
const flowStore = useFlowsStore();
const artifactsStore = useArtifactsStore();
const { getTimeFromNowString } = useTimeFromNow();

const inputNode: Ref<FlowNode | null> = computed(
  () => flow.value?.nodes?.find((node) => node.name == "input-0") || null
);
const augmentNodes: Ref<FlowNode[]> = computed(
  () =>
    flow.value?.nodes?.filter(
      (node) => !node.name.startsWith("input-") && node.function_id != "bench.model"
    ) || []
);
const modelNodes: Ref<FlowNode[]> = computed(
  () => flow.value?.nodes?.filter((node) => node.function_id == "bench.model") || []
);
// models referenced by modelNodes
const { result: usedModelsByNV } = computedAsync(async () => {
  const referencedModels: string[] = modelNodes.value
    .map((node) => artifactEdges(node, "argument").pop()?.dependency)
    .filter((model) => model != undefined) as string[];
  const models = await Promise.all(
    referencedModels.map((artifact) => artifactsStore.getVersion(...mapNameVersion(artifact)))
  );
  const modelsByNV: Record<string, ArtifactVersion> = {};
  models.forEach((model) => (modelsByNV[toNameVersion(model)] = model));
  return modelsByNV;
});

function getNodeIndex(nodeName: string): number {
  return flow.value?.nodes?.filter((node) => node.name.startsWith(nodeName)).length || 0;
}

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
  const referencedModel = artifactEdges(modelNode, "argument").pop()?.dependency;
  return (usedModelsByNV.value || {})[referencedModel || ""];
}

async function addModelsAndConnectInput(models: string[]) {
  if (inputNode.value == null) {
    throw new Error("input not initialized");
  }

  const modelNodes = await addModels(models);
  await connectFlowNodes(inputNode.value, modelNodes, "input");
}

async function addModels(models: string[]) {
  // TODO @Cleanup: guard against models already exist when user adds model?
  //  Currently it will just error because the model name already exists.

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
        createFlowNode({
          name: modelNodeName + "-" + getNodeIndex(modelNodeName),
          function_id: "bench.model",
        })
      )
  );

  // create connections to models
  await Promise.all(
    modelNodes.map((modelNode, i) =>
      connectFlowNodeArtifact(modelNode, models[i], "argument", "model")
    )
  );

  return modelNodes;
}

async function initNewPlayground() {
  var foundNewName = false;
  var flowName = "";
  while (!foundNewName) {
    flowName = getRandomName();
    if (!flowStore.flowExists(flowName)) {
      foundNewName = true;
    }
  }
  initPlayground(flowName);
}

async function initPlayground(flowName: string) {
  let flowInstance = flowStore.flow(flowName);
  if (!flowInstance) {
    flowInstance = await flowStore.createFlow({ name: flowName });
  }
  flow.value = flowInstance.latest_version || null;
  if (!flow.value) {
    // create and initialize
    flow.value = await flowStore.createFlowVersion(flowName, {
      name: "Initial commit",
      description: "Auto-generated.",
      parents: [],
    });
    setupPlayground(props.models || []);
  }
}

async function setupPlayground(models: string[]) {
  // create main input node
  const inputNode = await createFlowNode({
    name: "input-" + getNodeIndex("input"),
    function_id: "bench.identity",
  });
  const modelNodes = await addModels(models);
  await connectFlowNodes(inputNode, modelNodes, "input");
}

// recover or setup playground
onBeforeMount(async () => {
  const flowToRecover = flowStore.lastOpenedFlow;
  if (flowToRecover != null) {
    flow.value = flowToRecover;
  } else {
    initNewPlayground();
  }
});

// TODO @Feature: derive input spec from selected models (or any other specs)
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

const selectedNode: Ref<FlowNode | null> = ref(null);
const selectedFromNodes: Ref<FlowNode[]> = ref([]);
const selectedToNodes: Ref<FlowNode[]> = ref([]);

const artifactSelect = ref(null);
function promptAddModel() {
  (artifactSelect.value as any).show();
}

const editNodeSlideover = ref(null);
function promptAddNode(fromNodes: FlowNode[], toNodes: FlowNode[]) {
  selectedNode.value = null;
  selectedFromNodes.value = fromNodes;
  selectedToNodes.value = toNodes;
  (editNodeSlideover.value as any).show();
}

function promptEditNode(node: FlowNode) {
  selectedNode.value = node;
  (editNodeSlideover.value as any).show();
}

async function createFlowNodeFromSelection(v: {
  node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
  connectedArtifacts: Record<string, ArtifactVersion>;
}) {
  // create node
  const node = await createFlowNode(v.node);

  // connect to argument artifacts (only for now)
  for (const connectionName in v.connectedArtifacts) {
    const artifact = toNameVersion(v.connectedArtifacts[connectionName]);
    await connectFlowNodeArtifact(node, artifact, "argument", connectionName);
  }

  // connect to current selection of input/output nodes (* connection only for now)
  for (const inputNode of selectedFromNodes.value) {
    await connectFlowNodes(inputNode, [node], "input");
  }
  await connectFlowNodes(node, selectedToNodes.value, "input");

  (editNodeSlideover.value as any).hide();
}

async function updateFlowNodeInPlace(v: {
  node: FlowNode;
  connectedArtifacts: Record<string, ArtifactVersion>;
}) {
  // update node
  await updateFlowNode(v.node);

  // TODO @Broken: update connected artifacts

  (editNodeSlideover.value as any).hide();
}

const canExecute: Ref<boolean> = computed(() => flowInputRecord.value != {});
const executions: Ref<Array<Execution>> = ref([]);

// fetch executions whenever the flow instance changes
watch(
  flow,
  (flow, oldFlow) => {
    if (oldFlow == null || flow == null || flow?.id != oldFlow?.id) {
      if (flow == null) {
        executions.value = [];
      } else {
        fetchExecutions(flow.id);
      }
    }
  },
  { immediate: true }
);

function getEagerExecutionConnections(plan: FlowExecutionPlan, execution: Execution) {
  const argumentsValues = [] as FlowNodeExecutionArgument[];
  Object.values(plan.arguments).forEach((nodeArguments: FlowNodeExecutionArgument[]) =>
    argumentsValues.push(...nodeArguments)
  );

  // TODO @Cleanup: faux/eager execution connections are tightly coupled to execution grid
  const eagerConnections = argumentsValues
    .filter((argument) => argument.records != null)
    .map((argument: FlowNodeExecutionArgument) => {
      // determine name of soon-to-be artifact according to well known backend schema
      const nodeName = getFlowNode(argument.node)?.name;
      var artifact;
      if (argument.name == "*") {
        artifact = `${flow.value?.flow}.${nodeName}.${argument.type}s`;
      } else {
        artifact = `${flow.value?.flow}.${nodeName}-${argument.type}s.${argument.name}`;
      }
      // create pretend execution artifact connection with known data
      return {
        execution: execution.id,
        artifact: `${artifact}@0`,
        connection_type: argument.type,
        connection_name: argument.name,
        view_inline: { start: -argument.records.length, end: 0 },
        dataset_preview: {
          count: argument.records.length,
          limit: 3,
          results: argument.records,
        },
      } as ExecutionArtifactConnection;
    });
  return eagerConnections;
}

async function execute() {
  if (flow.value == null || inputNode.value == null) {
    throw new Error("cannot execute: flow is not initialized");
  }

  console.log("execute flow with input", flow.value, flowInputRecord);
  const plan: FlowExecutionPlan = {
    arguments: {
      [inputNode.value?.id]: [
        {
          type: "input",
          node: inputNode.value.id,
          name: "*",
          records: [flowInputRecord.value],
        },
      ],
    },
    options: {
      blocking: false,
      validate: "lazy",
    },
  };
  await api
    .post<Execution>(`/flows/${flow.value.flow}/versions/${flow.value.version}/execute`, plan)
    .then((response) => response.data)
    .then((execution) => {
      // automatically insert preview datasets for arguments we passed in to give quicker feedback
      if (plan.arguments == null || getAllConnectedArtifacts(execution).length > 0) {
        // bail if there were no arguments or if the backend already manifested them on the execution
        return execution;
      }
      const eagerConnections = getEagerExecutionConnections(plan, execution);
      execution.connected_artifacts = eagerConnections;
      return execution;
    })
    .then((execution) => (executions.value = [execution, ...executions.value]));
}

function fetchExecutions(flow: string, limit = 10) {
  api
    .get<LimitPaginatedResult<Execution>>(`/executions`, { params: { type: "flow", flow, limit } })
    .then((result) => result.data)
    .then((result) => (executions.value = result.results));
}

const poll: Ref<boolean> = ref(true);
const pollIntervalMillis = 250;
const waitIntervalMillis = 250;
const pollExecutionsInterval = setInterval(pollUnterminatedExecutions, pollIntervalMillis);
onBeforeUnmount(() => clearInterval(pollExecutionsInterval));

async function pollUnterminatedExecutions() {
  if (!poll.value) {
    return;
  }

  const pendingExecutions = executions.value
    .filter((execution) => !isTerminal(execution.state))
    .filter(
      (execution) =>
        DateTime.fromISO(execution.updated_at).diffNow().milliseconds < -waitIntervalMillis
    );
  if (pendingExecutions.length == 0) {
    return;
  }

  const updatedExecutions = await api
    .get<LimitPaginatedResult<Execution>>(`/executions`, {
      params: {
        // TODO @Performance: filter for id__in when polling pending executions
        //  Currently not doing this as it messes with django-filters array conversion somehow.
        // id__in: pendingExecutions.map((execution) => execution.id),
        updated_at__gt: DateTime.utc()
          .minus(Duration.fromMillis(pollIntervalMillis * 5))
          .toISO({ includeOffset: false }),
        type: "flow",
        flow: flow.value?.id,
        limit: pendingExecutions.length,
      },
    })
    .then((response) => response.data.results);

  // replace current executions with updated ones
  updatedExecutions.forEach((updatedExecution) => {
    const index = executions.value.findIndex((ex) => ex.id == updatedExecution.id);
    executions.value[index] = updatedExecution;
  });
}
</script>
