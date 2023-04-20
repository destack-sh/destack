<script lang="ts" setup>
import { graphql } from "@/gql";
import { SymbolType, type ProjectVersion } from "@/gql/graphql";
import { useCurrentInterpModule } from "@/state/runtime";
import { INTEGER_ZERO } from "@/utils/fractional";
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
    <ul class="flex flex-col gap-2 py-2">
      <li v-for="build in builds" :key="build.id" class="px-3 text-sm">
        <h4 class="font-bold">{{ build.name }}</h4>
        <p>{{ build.description }}</p>
      </li>
    </ul>
  </div>
</template>
