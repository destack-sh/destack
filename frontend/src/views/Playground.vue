<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Playground</h1>
    </div>
    <!-- Model selection -->
    <!-- This example requires Tailwind CSS v2.0+ -->

    <div class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Models</h3>
        <div class="mt-3 sm:mt-0 sm:ml-4">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          >
            Add model
          </button>
        </div>
      </div>
      <ul role="list" class="mt-3 grid grid-cols-1 gap-5 sm:grid-cols-2 sm:gap-6 lg:grid-cols-4">
        <li v-for="model in models" :key="model.name" class="col-span-1 flex rounded-md shadow-sm">
          <div
            class="flex flex-1 items-center justify-between truncate rounded-r-md border-t border-r border-b border-gray-200 bg-white"
          >
            <div class="flex-1 truncate px-4 py-2 text-sm">
              <router-link
                :to="'/models/' + model.artifact?.name"
                class="font-medium text-gray-900 hover:text-gray-600"
                >{{ model.artifact?.name }}</router-link
              >
              <p class="text-gray-500">{{ model.version }}</p>
            </div>
            <div class="flex-shrink-0 pr-2">
              <button
                type="button"
                class="inline-flex h-8 w-8 items-center justify-center rounded-full bg-white bg-transparent text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
              >
                <span class="sr-only">Open options</span>
                <DotsVerticalIcon class="h-5 w-5" aria-hidden="true" />
              </button>
            </div>
          </div>
        </li>
      </ul>
    </div>
    <!-- Input -->
    <form class="sm:px--6 mx-auto max-w-7xl px-4 pt-6 md:px-8">
      <div class="mb-3 border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Input</h3>
        <div class="mt-3 sm:mt-0 sm:ml-4">
          <button
            type="submit"
            class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
            @click.prevent=""
          >
            Run
          </button>
        </div>
      </div>
      <div v-for="field in fields" :key="field.name">
        <label for="text" class="block text-sm font-medium text-gray-700">{{ field.name }}</label>
        <textarea
          rows="3"
          :name="field.name"
          :id="field.name"
          class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
          :placeholder="'Enter ' + field.name"
        />
      </div>
      <button
        type="submit"
        class="mt-3 inline-flex justify-center rounded-md border border-transparent bg-slate-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500 focus:ring-offset-2"
        @click.prevent=""
      >
        Run
      </button>
    </form>
    <!-- Executions & output -->
    <!-- TODO @Feature: show executions/output -->
  </Sidebar>
</template>
<script lang="ts" setup>
import Sidebar from "@/components/Sidebar.vue";
import type { ArtifactVersion } from "@/types/artifacts";
import type { FieldSpec } from "@/types/spec";
import { DotsVerticalIcon } from "@heroicons/vue/outline";

const props = defineProps({ models: Array });
const models: Array<ArtifactVersion> = [
  {
    id: "123",
    artifact: {
      id: "123",
      name: "spacy_ner",
      type: "model",
      created_at: "today",
    },
    artifact_id: "123",
    version: "0",
    parents: [],
    created_at: "today",
    metadata: {
      handler_id: "bench.spacy.bundled",
    },
  },
];
const fields: Array<FieldSpec> = [{ name: "text", description: "any text", type: "string" }];
</script>
