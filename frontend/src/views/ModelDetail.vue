<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-semibold text-gray-900">{{ modelName }}</h1>
          <h3 class="text-lg text-gray-900">
            {{ model?.description }}
            <span class="italic text-gray-700" v-if="!model?.description">No description yet</span>
          </h3>
        </div>
        <div>
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-orange-100 px-4 py-2 text-sm font-medium text-orange-700 hover:bg-orange-200 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click="_delete"
          >
            Delete
            <TrashIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
          </button>
        </div>
      </div>
      <div class="mx-auto flex w-full justify-end">
        <div v-if="model?.latest_version">
          <div class="mt-6">
            <router-link
              class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
              :to="`/models/${modelName}/versions/${model.latest_version?.version}`"
            >
              {{ model.latest_version.version }}
              {{ latestVersionDtFromNow }}
            </router-link>
            <router-link
              :to="`/models/${modelName}/versions`"
              class="ml-1 rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
            >
              {{ versionsPaginated?.count || 0 }} versions
            </router-link>
          </div>
        </div>
      </div>
    </div>
    <div v-if="versionsPaginated?.count == 0" class="mt-10 text-center">
      <ChipIcon class="mx-auto h-12 w-12 text-gray-400" />
      <h3 class="mt-2 text-sm font-medium text-gray-900">Empty model</h3>
      <p class="mt-1 text-sm text-gray-500">Get started by initializing from a template</p>
      <div class="mt-6">
        <router-link :to="`/models/${modelName}/edit`">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-orange-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          >
            <PlusIcon class="-ml-1 mr-2 h-5 w-5" aria-hidden="true" />
            Select template
          </button>
        </router-link>
      </div>
    </div>
    <div v-else class="mt-10 text-center">
      <!-- TODO @Feature: display model config & spec more attractively -->
      <router-link
        :to="{
          path: `/models/${modelName}/edit`,
          query: { parent: model?.latest_version?.version },
        }"
      >
        <button
          type="button"
          class="inline-flex items-center rounded-md border border-transparent bg-orange-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
        >
          Edit
          <PencilIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
        </button>
      </router-link>
      <h3 class="mt-2 text-sm font-medium text-gray-900">Config</h3>
      {{ latestMetadata?.handler_id }}
      {{ model?.latest_version?.storage_uri }}
      {{ latestMetadata?.config_arguments }}
      <h3 class="mt-2 text-sm font-medium text-gray-900">Spec</h3>
      <div
        class="p-3"
        v-if="latestMetadata?.input_spec != null && latestMetadata?.output_spec != null"
      >
        <RecordSpecDisplay :spec="latestMetadata?.input_spec" />
        =>
        <RecordSpecDisplay :spec="latestMetadata?.output_spec" />
      </div>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore } from "@/stores";
import type { ModelMetadata } from "@/types";
import { ChipIcon, PencilIcon, PlusIcon, TrashIcon } from "@heroicons/vue/outline";
import { computed, type Ref } from "@vue/reactivity";
import { DateTime } from "luxon";
import { useRouter } from "vue-router";

const props = defineProps({ modelName: { type: String, required: true } });

const artifactsStore = useArtifactsStore();
const model = computed(() => artifactsStore.artifact(props.modelName));
const latestMetadata = computed(() => {
  if (model.value?.latest_version == null) {
    return null;
  } else {
    return model.value?.latest_version.metadata as ModelMetadata;
  }
});

const { result: versionsPaginated } = computedAsync(() =>
  artifactsStore.getVersions(props.modelName)
);
const latestVersionDtFromNow: Ref<string | null> = computed(() => {
  if (model.value?.latest_version == null) return null;
  return DateTime.fromISO(model.value.latest_version.created_at).toRelative({ locale: "en-US" });
});

const router = useRouter();
async function _delete() {
  await api.delete(`/models/${props.modelName}`);
  await artifactsStore.hydrate();
  router.push("/models");
}
</script>
