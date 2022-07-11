<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">{{ modelName }}</h1>
      <h3 class="text-lg text-gray-900">
        {{ model?.description }}
        <span class="text-gray-700" v-if="!model?.description">No description yet</span>
      </h3>

      <div class="mx-auto flex w-full justify-end">
        <div v-if="latestVersion">
          <div class="mt-6">
            <router-link
              class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
              :to="`/models/${modelName}/versions/${latestVersion?.version}`"
            >
              {{ latestVersion.version }}
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
    <div v-if="versionsPaginated?.count == 0">
      <div class="mt-10 text-center">
        <ChipIcon class="mx-auto h-12 w-12 text-gray-400" />
        <h3 class="mt-2 text-sm font-medium text-gray-900">Empty model</h3>
        <p class="mt-1 text-sm text-gray-500">Get started by initializing from a template</p>
        <div class="mt-6">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-orange-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          >
            <PlusIcon class="-ml-1 mr-2 h-5 w-5" aria-hidden="true" />
            Select template
          </button>
        </div>
      </div>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore } from "@/stores";
import type { ArtifactVersion } from "@/types";
import { ChipIcon, PlusIcon } from "@heroicons/vue/outline";
import { computed, type Ref } from "@vue/reactivity";
import { DateTime } from "luxon";

const props = defineProps({ modelName: { type: String, required: true } });

const artifactsStore = useArtifactsStore();
const { result: versionsPaginated } = computedAsync(() =>
  artifactsStore.getVersions(props.modelName)
);
const model = computed(() => artifactsStore.artifact(props.modelName));

const versions = computed(() => versionsPaginated.value?.results);
const latestVersion: Ref<ArtifactVersion | null> = computed(() => {
  if (versions.value != null && versions.value.length > 0) {
    return versions.value[0];
  } else {
    return null;
  }
});
const latestVersionDtFromNow: Ref<string | null> = computed(() => {
  if (latestVersion.value == null) return null;
  return DateTime.fromISO(latestVersion.value.created_at).toRelative({ locale: "en-US" });
});
</script>
