<template>
  <div class="mx-auto max-w-7xl px-4 pt-4 sm:px-6 md:px-8">
    <ul role="list" class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <li
        v-for="model in artifactsStore.models"
        :key="model.id"
        class="relative col-span-1 divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
      >
        <div class="flex w-full items-center justify-between space-x-6 p-6">
          <router-link :to="'/models/' + model.name" class="focus:outline-none">
            <!-- Extend touch target to entire panel -->
            <span class="absolute inset-0" aria-hidden="true" />
          </router-link>
          <div class="flex-1 truncate">
            <div class="flex items-center space-x-3">
              <h3 class="truncate text-sm font-medium text-gray-900">
                {{ model.name }}
              </h3>
              <span
                class="inline-block flex-shrink-0 rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-800"
                >{{ model.type }}</span
              >
            </div>
            <p class="mt-1 truncate text-sm text-gray-500">
              {{ model.description }}
            </p>
          </div>
          <component
            :is="getIconForModel(model)"
            class="h-10 w-10 flex-shrink-0 text-orange-300"
            aria-hidden="true"
          />
        </div>
        <div>
          <div class="-mt-px flex divide-x divide-gray-200">
            <div class="flex w-0 flex-1">
              <router-link
                :to="'/playground/' + model.name"
                class="relative -mr-px inline-flex w-0 flex-1 items-center justify-center rounded-bl-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
              >
                <GlobeIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
                <span class="ml-2">Playground</span>
              </router-link>
            </div>
            <div class="-ml-px flex w-0 flex-1">
              <router-link
                :to="'/laboratory/' + model.name"
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
import type { Artifact } from "@/types/artifacts";
import { BeakerIcon, DocumentTextIcon, GlobeIcon } from "@heroicons/vue/outline";

const artifactsStore = useArtifactsStore();

function getIconForModel(model: Artifact) {
  if (model.type == "model") {
    return DocumentTextIcon;
  }
}
</script>
