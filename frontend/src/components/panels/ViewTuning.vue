<script lang="ts" setup>
import { graphql } from "@/gql";
import { BuildScope, SymbolType, type ProjectVersion } from "@/gql/graphql";
import { useCurrentInterpModule, useSymbolOps } from "@/state/runtime";
import { INTEGER_ZERO } from "@/utils/fractional";
import { WrenchIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ version: ProjectVersion; focused: boolean }>();

const interp = useCurrentInterpModule();
const autobuildFile = computed(() => interp.module.value?.files.find((f) => f.path == "__autobuild__"));
const { result: autobuildFileResult } = useQuery(
  graphql(/* GraphQL */ `
    query autobuildFileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        projectVersion {
          id
        }
        statements(filters: { isVisible: true }) {
          id
          name
          type
          symbolType
          orderKey
          parent {
            id
          }
          description
          commented
          buildSettings {
            id
            reactive
          }
          evaluateSettings {
            id
            weights
          }
        }
      }
    }
  `),
  computed(() => ({
    fileId: autobuildFile.value?.id,
  })) as any,
  {
    enabled: computed(() => !!autobuildFile.value && props.focused),
  }
);
const builds = computed(
  () =>
    autobuildFileResult.value?.file?.statements
      ?.filter((s) => s.symbolType == SymbolType.Build)
      .sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1)) ?? []
);
const symbolOps = useSymbolOps();

function getModelsFor(build: { id: string }) {
  return (
    autobuildFileResult.value?.file?.statements
      ?.filter((s) => s.parent?.id == build.id && s.symbolType == SymbolType.Model)
      .sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1)) ?? []
  );
}

function toggleReactive(build: { id: string; buildSettings: { reactive: boolean } }) {
  throw new Error("TODO @Incomplete: not implemented");
}

const HIGHLIGHTED_METRICS = ["performance", "speed"];
</script>
<template>
  <div class="">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
    >
      <span class="text-xs font-bold uppercase">
        Builds
        <span class="ml-1 rounded-lg bg-gray-200 px-1 font-normal text-gray-800" v-if="builds.length">
          {{ builds.length }}
        </span>
      </span>
    </div>
    <ul class="flex flex-col gap-4 py-2">
      <!-- Each build -->
      <li v-for="build in builds" :key="build.id" class="group px-3 py-1 text-sm hover:bg-orange-100">
        <!-- Basic info -->
        <span class="flex flex-row items-baseline justify-between">
          <h4 class="flex flex-row items-baseline text-black">
            {{ build.name }}
            <!-- Reactivity toggle -->
            <button
              class="ml-2 rounded-sm border px-1.5"
              @click="toggleReactive(build)"
              :class="{
                'border-orange-900 border-opacity-[15%] bg-orange-100 text-gray-700': build.buildSettings?.reactive,
                ' border-gray-300  text-gray-400': !build.buildSettings?.reactive,
              }"
            >
              {{ build.buildSettings?.reactive ? "live" : "manual" }}
            </button>
          </h4>
          <!-- Basic controls -->
          <span class="flex flex-row items-center">
            <button class="text-gray-400 hover:text-gray-800" @click="symbolOps.build(build, BuildScope.Selected)">
              <WrenchIcon class="-mb-1 h-4 w-4" />
            </button>
          </span>
        </span>
        <p class="text-gray-500">{{ build.description }}</p>
        <!-- Metrics -->
        <div class="mt-1 flex flex-col">
          <span
            v-for="metric of HIGHLIGHTED_METRICS"
            :key="metric"
            class="flex flex-row items-center gap-2 px-0.5 text-xs"
          >
            <span class="text-gray-900">{{ metric.slice(0, 1).toUpperCase() }}</span>
            <!-- blue on gray line with value of metric in build settings as percentage -->
            <div class="relative h-0.5 w-full rounded-sm bg-gray-300">
              <div
                :style="{ width: `${build.evaluateSettings?.weights[metric] * 100}%` }"
                class="absolute h-0.5 rounded-sm bg-orange-600"
              />
            </div>
          </span>
        </div>
        <!-- Available models -->
        <div class="mt-1 flex flex-row flex-wrap gap-2">
          <span
            v-for="model of getModelsFor(build)"
            :key="model.id"
            class="mt-1 rounded-sm border border-orange-900 border-opacity-[15%] bg-gray-100 px-1.5 py-0.5 text-sm"
          >
            {{ model.name }}
          </span>
        </div>
      </li>
    </ul>
  </div>
</template>
