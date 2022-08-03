<template>
  <div class="mx-auto max-w-7xl px-4 pt-4 sm:px-6 md:px-8">
    <ul role="list" class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <li
        v-for="flow in flowsStore.flows"
        :key="flow.id"
        class="relative col-span-1 divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow"
      >
        <div class="flex w-full items-center justify-between space-x-3 p-6">
          <router-link :to="'/flows/' + flow.name" class="focus:outline-none">
            <!-- Extend touch target to entire panel -->
            <span class="absolute inset-0" aria-hidden="true" />
          </router-link>
          <component :is="getIconForFlow(flow)" class="h-8 w-8 flex-shrink-0 text-orange-300" aria-hidden="true" />
          <div class="flex-1 truncate">
            <div class="flex items-center space-x-3">
              <h3 class="truncate text-sm font-medium text-gray-900">
                {{ flow.name }}
              </h3>
            </div>
            <p v-if="flow.description" class="mt-1 truncate text-sm text-gray-500">
              {{ flow.description }}
            </p>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>
<script lang="ts" setup>
import { useFlowsStore } from "@/stores/flows";
import type { Flow } from "@/types/flows";
import { VariableIcon } from "@heroicons/vue/outline";

const flowsStore = useFlowsStore();

function getIconForFlow(flow: Flow) {
  return VariableIcon;
}
</script>
