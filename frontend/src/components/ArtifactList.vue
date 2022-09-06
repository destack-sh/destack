<template>
  <div class="mx-auto max-w-7xl px-4 pt-4 sm:px-6 md:px-8">
    <ul role="list" class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <li
        v-for="artifact in relevantArtifacts"
        :key="artifact.id"
        class="relative col-span-1 divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
      >
        <div class="flex w-full items-center justify-between space-x-3 p-6">
          <router-link :to="`/${artifact.type}s/` + artifact.name" class="focus:outline-none">
            <!-- Extend touch target to entire panel -->
            <span class="absolute inset-0" aria-hidden="true" />
          </router-link>
          <component
            :is="getIconForArtifact(artifact)"
            class="h-8 w-8 flex-shrink-0 text-orange-300"
            aria-hidden="true"
          />
          <div class="flex-1 truncate">
            <div class="flex items-center space-x-3">
              <h3 class="truncate text-sm font-medium text-gray-900">
                {{ artifact.name }}
              </h3>
            </div>
            <p v-if="artifact.description" class="mt-1 truncate text-sm text-gray-500">
              {{ artifact.description }}
            </p>
            <p v-if="artifact.head" class="mt-1 truncate text-xs text-gray-500">
              {{ metadata(artifact)?.handler_id }}
            </p>
            <span
              class="mt-1 inline-flex items-center rounded bg-yellow-100 px-2 py-0.5 text-xs font-medium text-yellow-800"
              v-for="tag in artifact.tags"
              :key="tag"
            >
              {{ tag }}
            </span>
          </div>
        </div>
        <div v-if="artifact.type == 'model'" class="-mt-px flex divide-x divide-gray-200">
          <div class="flex w-0 flex-1">
            <router-link
              :to="{ name: 'playground', query: { new: 1, artifacts: [`${artifact.name}@HEAD`] } }"
              class="relative -mr-px inline-flex w-0 flex-1 items-center justify-center rounded-bl-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
            >
              <GlobeAltIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
              <span class="ml-2">Playground</span>
            </router-link>
          </div>
          <div class="-ml-px flex w-0 flex-1">
            <router-link
              :to="{ name: 'playground', query: { artifacts: [`${artifact.name}@HEAD`] } }"
              class="relative inline-flex w-0 flex-1 items-center justify-center rounded-br-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
            >
              <BeakerIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
              <span class="ml-2">Laboratory</span>
            </router-link>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>
<script lang="ts" setup>
import { useArtifactsStore } from "@/stores/artifacts";
import type { Artifact, DatasetMetadata, ModelMetadata } from "@/types/artifacts";
import { BeakerIcon, CircleStackIcon, CpuChipIcon, GlobeAltIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{ type: "model" | "dataset" }>();
const artifactsStore = useArtifactsStore();

const relevantArtifacts = computed(() => {
  if (props.type == "model") {
    return artifactsStore.models;
  } else if (props.type == "dataset") {
    return artifactsStore.datasets;
  } else {
    throw new Error("unknown artifact type: " + props.type);
  }
});

function metadata(artifact: Artifact): ModelMetadata | DatasetMetadata | null {
  return (artifact.head?.metadata as ModelMetadata | DatasetMetadata | undefined) || null;
}

function modelMetadata(artifact: Artifact): ModelMetadata | null {
  return (artifact.head?.metadata as ModelMetadata | undefined) || null;
}

function datasetMetadata(artifact: Artifact): DatasetMetadata | null {
  return (artifact.head?.metadata as DatasetMetadata | undefined) || null;
}

function getIconForArtifact(artifact: Artifact) {
  if (artifact.type == "model") {
    return CpuChipIcon;
  } else if (artifact.type == "dataset") {
    return CircleStackIcon;
  }
}
</script>
