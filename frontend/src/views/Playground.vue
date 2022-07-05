<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Playground</h1>
      <button
        type="submit"
        class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
        @click.prevent="run"
      >
        Run
      </button>
    </div>

    <!-- Input -->
    <form class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="mb-3 border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Input</h3>
      </div>
      <div v-for="(field, i) in fields" :key="field.name">
        <label for="text" class="block text-sm font-medium text-gray-700">{{ field.name }}</label>
        <textarea
          rows="3"
          v-model="fieldValues[i]"
          :name="field.name"
          :id="field.name"
          class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
          :placeholder="'Enter ' + field.name"
        />
      </div>
    </form>

    <!-- Select models -->
    <div class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Models</h3>
        <div class="mt-3 sm:mt-0 sm:ml-4">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
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
              <p class="text-gray-500">{{ model.version }}</p>
            </div>
            <div class="flex-shrink-0 pr-2">
              <button
                type="button"
                class="inline-flex h-8 w-8 items-center justify-center rounded-full bg-transparent bg-white text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
              >
                <span class="sr-only">Open options</span>
                <DotsVerticalIcon class="h-5 w-5" aria-hidden="true" />
              </button>
            </div>
          </div>
        </li>
      </ul>
    </div>

    <!-- Executions & output -->
    <div class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Outputs</h3>
      </div>
    </div>
    <!-- TODO @Feature: show executions/output -->
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore } from "@/stores";
import type { Execution } from "@/types";
import type { FieldSpec } from "@/types/spec";
import { DotsVerticalIcon } from "@heroicons/vue/outline";
import { ref, type PropType, type Ref } from "vue";

const artifactsStore = useArtifactsStore();
const props = defineProps({ models: { type: Array as PropType<Array<string>>, required: true } });
const { result: models } = computedAsync(() =>
  Promise.all(
    props.models.map((model: string) => {
      const artifactId = model.split("@")[0];
      const version = model.split("@")[1];
      return artifactsStore.getVersionByTag(artifactId, version);
    })
  )
);
const fields: Array<FieldSpec> = [{ name: "text", description: "any text", type: "string" }];
const fieldValues: Array<any> = [""];
const executions: Ref<Array<Execution>> = ref([]);

function run() {
  const fieldValuesAsRecord: Record<string, any> = {};
  for (const field of fields) {
    fieldValuesAsRecord[field.name] = fieldValues[0];
  }

  const outputs = models.value?.map((model) =>
    api
      .post<Record<string, any>>(
        `/models/${model.artifact}/versions/${model.version}/predict`,
        fieldValuesAsRecord
      )
      .then((result) => result.data)
      .then((result) => console.log(result))
      .then(() => getExecutions(model.id))
  );
}

function getExecutions(model?: string, flow?: string) {
  api
    .get<Array<Execution>>(`/executions`, { params: { model, flow } })
    .then((result) => result.data)
    .then((result) => (executions.value = result));
}

function getDataset(dataset: string, version: string) {}
</script>
