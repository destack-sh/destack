<template>
  <div class="mx-auto max-w-7xl px-4 pt-4 sm:px-6 md:px-8">
    <ul role="list" class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <li
        v-for="model in artifactsStore.models"
        :key="model.id"
        class="relative col-span-1 divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
      >
        <div class="flex w-full items-center justify-between space-x-3 p-6">
          <router-link :to="'/models/' + model.name" class="focus:outline-none">
            <!-- Extend touch target to entire panel -->
            <span class="absolute inset-0" aria-hidden="true" />
          </router-link>
          <component :is="getIconForArtifact(model)" class="h-8 w-8 flex-shrink-0 text-orange-300" aria-hidden="true" />
          <div class="flex-1 truncate">
            <div class="flex items-center space-x-3">
              <h3 class="truncate text-sm font-medium text-gray-900">
                {{ model.name }}
              </h3>
            </div>
            <p v-if="model.description" class="mt-1 truncate text-sm text-gray-500">
              {{ model.description }}
            </p>
            <p v-if="model.latest_version" class="mt-1 truncate text-xs text-gray-500">
              {{ latestMetadata(model)?.handler_id }}
            </p>
          </div>
        </div>
        <div>
          <div class="-mt-px flex divide-x divide-gray-200">
            <div class="flex w-0 flex-1">
              <router-link
                :to="{ name: 'playground', query: { models: [`${model.name}@HEAD`] } }"
                class="relative -mr-px inline-flex w-0 flex-1 items-center justify-center rounded-bl-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
              >
                <GlobeIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
                <span class="ml-2">Playground</span>
              </router-link>
            </div>
            <div class="-ml-px flex w-0 flex-1">
              <router-link
                :to="{ name: 'playground', query: { models: [`${model.name}@HEAD`] } }"
                class="relative inline-flex w-0 flex-1 items-center justify-center rounded-br-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
              >
                <BeakerIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
                <span class="ml-2">Laboratory</span>
              </router-link>
            </div>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>
<script lang="ts" setup>
import { useArtifactsStore } from "@/stores/artifacts";
import type { Artifact, ModelMetadata } from "@/types/artifacts";
import { BeakerIcon, ChipIcon, DatabaseIcon, GlobeIcon } from "@heroicons/vue/outline";

const artifactsStore = useArtifactsStore();

function latestMetadata(model: Artifact): ModelMetadata | null {
  if (model.latest_version == null) {
    return null;
  } else {
    return model.latest_version.metadata as ModelMetadata;
  }
}

function getIconForArtifact(artifact: Artifact) {
  if (artifact.type == "model") {
    return ChipIcon;
  } else if (artifact.type == "dataset") {
    return DatabaseIcon;
  }
}
</script>
