<template>
  <div class="mx-auto max-w-7xl px-4 pt-4 sm:px-6 md:px-8">
    <ul role="list" class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <li
        v-for="dataset in artifactsStore.datasets"
        :key="dataset.id"
        class="relative col-span-1 divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
      >
        <div class="flex w-full items-center justify-between space-x-3 p-6">
          <router-link :to="'/datasets/' + dataset.name" class="focus:outline-none">
            <!-- Extend touch target to entire panel -->
            <span class="absolute inset-0" aria-hidden="true" />
          </router-link>
          <component
            :is="getIconForArtifact(dataset)"
            class="h-8 w-8 flex-shrink-0 text-orange-300"
            aria-hidden="true"
          />
          <div class="flex-1 truncate">
            <div class="flex items-center space-x-3">
              <h3 class="truncate text-sm font-medium text-gray-900">
                {{ dataset.name }}
              </h3>
            </div>
            <p v-if="dataset.description" class="mt-1 truncate text-sm text-gray-500">
              {{ dataset.description }}
            </p>
            <p v-if="dataset.latest_version" class="mt-1 truncate text-xs text-gray-500">
              {{ latestMetadata(dataset)?.handler_id }}
            </p>
          </div>
        </div>
        <div>
          <div class="-mt-px flex divide-x divide-gray-200"></div>
        </div>
      </li>
    </ul>
  </div>
</template>
<script lang="ts" setup>
import { useArtifactsStore } from "@/stores/artifacts";
import type { Artifact, DatasetMetadata } from "@/types/artifacts";
import { CircleStackIcon, CpuChipIcon } from "@heroicons/vue/24/outline";

const artifactsStore = useArtifactsStore();

function latestMetadata(dataset: Artifact): DatasetMetadata | null {
  if (dataset.latest_version == null) {
    return null;
  } else {
    return dataset.latest_version.metadata as DatasetMetadata;
  }
}

function getIconForArtifact(artifact: Artifact) {
  if (artifact.type == "model") {
    return CpuChipIcon;
  } else if (artifact.type == "dataset") {
    return CircleStackIcon;
  }
}
</script>
