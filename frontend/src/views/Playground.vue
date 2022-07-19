FlowExecutionPlan
<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Playground</h1>
      <button
        type="submit"
        class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
        @click.prevent="updatedExecution"
        :disabled="!canExecute"
      >
        Run
      </button>
    </div>

    <!-- Input -->
    <form class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="mb-3 border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Input</h3>
      </div>
      <RecordForm v-model="flowInputRecord" :spec="flowInputSpec" />
    </form>

    <!-- Select models -->
    <div class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Models</h3>
        <div class="mt-3 sm:mt-0 sm:ml-4">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click="promptAddModel"
          >
            Add model
          </button>
        </div>
      </div>
      <ul role="list" class="mt-3 grid grid-cols-1 gap-5 sm:grid-cols-2 sm:gap-6 lg:grid-cols-4">
        <li
          v-for="modelNode in modelNodes"
          :key="modelNode.name"
          class="col-span-1 flex rounded-md shadow-sm"
        >
          <div
            class="flex flex-1 items-center justify-between truncate rounded-r-md border-t border-b border-r border-gray-200 bg-white"
          >
            <div class="flex-1 truncate px-4 py-2 text-sm">
              <router-link
                :to="'/models/' + modelForNode(modelNode)?.artifact"
                class="font-medium text-gray-900 hover:text-gray-600"
                >{{ modelForNode(modelNode)?.artifact }}</router-link
              >
              <p class="text-gray-500">
                {{ modelForNode(modelNode)?.version }}
              </p>
            </div>
            <!-- TODO @UI @Bug model menu is clipped by parent container  -->
            <div class="flex-shrink-0 pr-2">
              <Menu as="div" class="relative inline-block text-left">
                <div>
                  <MenuButton
                    class="flex items-center rounded-full text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2 focus:ring-offset-gray-100"
                  >
                    <span class="sr-only">Open options</span>
                    <DotsVerticalIcon class="h-5 w-5" aria-hidden="true" />
                  </MenuButton>
                </div>

                <transition
                  enter-active-class="transition ease-out duration-100"
                  enter-from-class="transform opacity-0 scale-95"
                  enter-to-class="transform opacity-100 scale-100"
                  leave-active-class="transition ease-in duration-75"
                  leave-from-class="transform opacity-100 scale-100"
                  leave-to-class="transform opacity-0 scale-95"
                >
                  <MenuItems
                    class="absolute left-0 z-10 mt-2 w-56 origin-top-left rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
                  >
                    <div class="py-1">
                      <MenuItem v-slot="{ active }">
                        <button
                          href="#"
                          :class="[
                            active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
                            'block px-4 py-2 text-sm',
                          ]"
                          @click="removeModel(modelNode)"
                        >
                          Remove from playground
                        </button>
                      </MenuItem>
                    </div>
                  </MenuItems>
                </transition>
              </Menu>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <!-- Executions & output -->
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Outputs</h3>
      </div>
      <ExecutionsGrid :executions="executions" />
    </div>
  </Sidebar>
  <ArtifactSelect
    ref="artifactSelect"
    @select="(model) => addModelsAndConnectInput([`${model.name}@HEAD`])"
  />
</template>
<script lang="ts" setup>
import { api } from "@/api";
import ArtifactSelect from "@/components/ArtifactSelect.vue";
import ExecutionsGrid from "@/components/ExecutionsGrid.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import { useFlow } from "@/composables/useFlow";
import { computedAsync, useArtifactsStore, useFlowsStore } from "@/stores";
import {
  isTerminal,
  type ArtifactVersion,
  type Execution,
  type FlowExecutionPlan,
  type FlowNode,
  type FlowVersion,
  type LimitPaginatedResult,
  type RecordSpec,
  type ValueType,
} from "@/types";
import { mapNameVersion, splitNameVersion, toNameVersion } from "@/utils/versioning";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { DotsVerticalIcon } from "@heroicons/vue/outline";
import { DateTime, Duration } from "luxon";
import { computed, onBeforeMount, onBeforeUnmount, ref, watch, type PropType, type Ref } from "vue";

const props = defineProps({ models: { type: Array as PropType<Array<string>>, required: false } });

const flow: Ref<FlowVersion | null> = ref(null);
const { createFlowNode, deleteFlowNode, connectFlowNodes, connectFlowNodeArtifact, artifactEdges } =
  useFlow(flow);
const flowStore = useFlowsStore();
const artifactsStore = useArtifactsStore();

const inputNode: Ref<FlowNode | null> = computed(
  () => flow.value?.nodes?.find((node) => node.name == "input-0") || null
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

async function removeModel(modelNode: FlowNode) {
  await deleteFlowNode(modelNode);
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

// setup or recover playground
onBeforeMount(async () => {
  const flowName = "playground-" + DateTime.now().toISODate();
  // reload or create flow with version
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

async function updatedExecution() {
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
    },
  };
  await api
    .post<Execution>(`/flows/${flow.value.flow}/versions/${flow.value.version}/execute`, plan)
    .then((response) => response.data)
    .then((execution) => (executions.value = [execution, ...executions.value]));
}

function fetchExecutions(flow: string, limit = 10) {
  api
    .get<LimitPaginatedResult<Execution>>(`/executions`, { params: { type: "flow", flow, limit } })
    .then((result) => result.data)
    .then((result) => (executions.value = result.results));
}

const pollIntervalMillis = 200;
const pollExecutionsInterval = setInterval(pollUnterminatedExecutions, pollIntervalMillis);
onBeforeUnmount(() => clearInterval(pollExecutionsInterval));

async function pollUnterminatedExecutions(flow: string) {
  const pendingExecutions = executions.value.filter((execution) => !isTerminal(execution.state));
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
        flow,
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

const artifactSelect = ref(null);
function promptAddModel() {
  (artifactSelect.value as any).show();
}
</script>
