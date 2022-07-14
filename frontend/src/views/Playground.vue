PaginatedResult
<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Playground</h1>
      <button
        type="submit"
        class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
        @click.prevent="run"
        :disabled="!canRun"
      >
        Run
      </button>
    </div>

    <!-- Input -->
    <form class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="mb-3 border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Input</h3>
      </div>
      <RecordForm v-model="modelInputRecord" :spec="modelInputSpec" />
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
        <li v-for="model in models" :key="model.name" class="col-span-1 flex rounded-md shadow-sm">
          <div
            class="flex flex-1 items-center justify-between truncate rounded-r-md border-t border-b border-r border-gray-200 bg-white"
          >
            <div class="flex-1 truncate px-4 py-2 text-sm">
              <router-link
                :to="'/models/' + artifactsStore.artifact(model.artifact)?.name"
                class="font-medium text-gray-900 hover:text-gray-600"
                >{{ artifactsStore.artifact(model.artifact)?.name }}</router-link
              >
              <p class="text-gray-500">
                {{ artifactsStore.isHead(model) ? "HEAD" : model.version }}
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
                          @click="removeModel(model)"
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
  <ArtifactSelect ref="artifactSelect" @select="addModel" />
</template>
<script lang="ts" setup>
import { api } from "@/api";
import Sidebar from "@/components/Sidebar.vue";
import { useArtifactsStore } from "@/stores";
import {
  mapArtifactNameVersion,
  type Artifact,
  type ArtifactVersion,
  type Execution,
  type PaginatedResult,
  type RecordSpec,
  type ValueType,
} from "@/types";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { DotsVerticalIcon } from "@heroicons/vue/outline";
import { computed, ref, watchEffect, type PropType, type Ref } from "vue";
import ArtifactSelect from "../components/ArtifactSelect.vue";
import ExecutionsGrid from "../components/ExecutionsGrid.vue";
import RecordForm from "../components/RecordForm.vue";

const artifactsStore = useArtifactsStore();
const props = defineProps({ models: { type: Array as PropType<Array<string>>, required: false } });
const models: Ref<ArtifactVersion[]> = ref([]);

// (re-)initialize models if prop models changes
watchEffect(async () => {
  const modelsInstances = await Promise.all(
    (props.models || [])
      .map((model) => mapArtifactNameVersion(model))
      .map(([artifactName, artifactVersion]) =>
        artifactsStore.getVersionByTag(artifactName, artifactVersion)
      )
  );
  models.value = modelsInstances;
});

// TODO @Feature: derive input spec from selected models
const modelInputSpec: RecordSpec = {
  _type: "FieldSpec",
  name: "Common model input spec",
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
const modelInputRecord = ref({});

const executions: Ref<Array<Execution>> = ref([]);

watchEffect(() => models.value?.forEach((model) => fetchExecutions(model.id)));

const canRun: Ref<boolean> = computed(() => (models.value?.length || 0) > 0);

function run() {
  console.log("post to model", models.value, modelInputRecord);
  models.value?.map((model) =>
    api
      .post<Record<string, any>>(
        `/models/${model.artifact}/versions/${model.version}/predict`,
        modelInputRecord.value
      )
      .then((result) => result.data)
      .then((result) => {
        executions.value = [result["execution"] as Execution, ...executions.value.slice(0, 9)];
        fetchExecutions(model.id);
      })
  );
}

function fetchExecutions(model?: string, flow?: string, limit = 10) {
  api
    .get<PaginatedResult<Execution>>(`/executions`, { params: { model, flow, limit } })
    .then((result) => result.data)
    .then((result) => (executions.value = result.results));
}

async function addModel(model: Artifact) {
  var modelVersion = model.latest_version;
  if (!modelVersion) {
    modelVersion = await artifactsStore.getVersionByTag(model.name, "HEAD");
  }
  const alreadyExists = models.value.find((version) => version.id == modelVersion?.id);
  if (!alreadyExists) {
    models.value.push(modelVersion);
  }
}

function removeModel(model: ArtifactVersion) {
  models.value = models.value.filter((m) => m.id != model.id);
}

const artifactSelect = ref(null);
function promptAddModel() {
  (artifactSelect.value as any).show();
}
</script>
