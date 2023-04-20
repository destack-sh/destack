<script lang="ts" setup>
import type { ProjectVersion } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";

const props = defineProps<{ version: ProjectVersion }>();

const editor = useEditorState();
const builds = [];
</script>
<template>
  <div class="">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
    >
      <span class="text-xs font-bold uppercase">
        Tuning
        <span class="ml-1 rounded-lg bg-gray-200 px-1 font-normal text-gray-800" v-if="builds.length">
          {{ builds.length }}
        </span>
      </span>
    </div>
    tuning some stuff
  </div>
</template>
