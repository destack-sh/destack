<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">{{ modelName }}</h1>

      <div>
        <div class="mt-6 flow-root">
          <ul role="list" class="-my-5 divide-y divide-gray-200">
            <li v-for="version in versions" :key="version.id" class="py-4">
              <div class="flex items-center space-x-4">
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium text-gray-900">
                    {{ version.name }}
                  </p>
                  <p class="truncate text-sm text-gray-500">
                    {{ version.version }}
                  </p>
                </div>
                <div>
                  <a
                    href="#"
                    class="inline-flex items-center rounded-full border border-gray-300 bg-white px-2.5 py-0.5 text-sm font-medium leading-5 text-gray-700 shadow-sm hover:bg-gray-50"
                  >
                    View
                  </a>
                </div>
              </div>
            </li>
          </ul>
        </div>
        <div class="mt-6">
          <router-link
            :to="`/models/${modelName}/versions`"
            class="flex w-full items-center justify-center rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
          >
            View all
          </router-link>
        </div>
      </div>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore } from "@/stores";

const props = defineProps({ modelName: { type: String, required: true } });

const artifactsStore = useArtifactsStore();
const { result: versions } = computedAsync(() => artifactsStore.getVersions(props.modelName));
</script>
