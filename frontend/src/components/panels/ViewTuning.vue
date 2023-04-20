<script lang="ts" setup>
import { graphql } from "@/gql";
import { SymbolType, type ProjectVersion, BuildScope } from "@/gql/graphql";
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
    enabled: computed(() => !!autobuildFile.value),
  }
);
const builds = computed(
  () =>
    autobuildFileResult.value?.file?.statements
      ?.filter((s) => s.symbolType == SymbolType.Build)
      .sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1)) ?? []
);
const symbolOps = useSymbolOps();

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
    <ul class="flex flex-col gap-5 py-2">
      <li v-for="build in builds" :key="build.id" class="px-3 text-sm">
        <!-- Basic info -->
        <span class="flex flex-row items-center justify-between">
          <h4 class="font-bold text-gray-900">{{ build.name }}</h4>
          <!-- Basic controls -->
          <span>
            <button class="text-gray-400" @click="symbolOps.build(build, BuildScope.Selected)">
              <WrenchIcon class="h-4 w-4" />
            </button>
          </span>
        </span>
        <p class="text-gray-700">{{ build.description }}</p>
        <!-- Metrics -->
        <div class="mt-1 flex flex-col">
          <span
            v-for="metric of HIGHLIGHTED_METRICS"
            :key="metric"
            class="font-bol2 flex flex-row items-center gap-2 text-xs"
          >
            {{ metric.slice(0, 1).toUpperCase() }}
            <!-- blue on gray line with value of metric in build settings as percentage -->
            <div class="relative h-1 w-full rounded-sm bg-gray-300">
              <div
                :style="{ width: `${build.evaluateSettings?.weights[metric] * 100}%` }"
                class="absolute h-1 rounded-sm bg-sky-400"
              ></div>
            </div>
          </span>
        </div>
        <!-- Available models -->
        <!-- TODO @Incomplete -->
      </li>
    </ul>
  </div>
</template>
