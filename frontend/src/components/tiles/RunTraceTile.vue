<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import type { RunStatus } from "@/gql/graphql";
import { useCurrentModule, useNavigation } from "@/state/module";
import { useRun } from "@/state/session";
import { ref, toRef } from "vue";

type TRACE_LAYOUT = "list" | "bartree";

const props = defineProps<{
  sessionId?: string;
  rootId: string;
  layout?: TRACE_LAYOUT;
  filter?: {
    statuses?: RunStatus[];
  };
  live?: boolean;
}>();

const layout = ref<TRACE_LAYOUT>(props.layout ?? "list");
const module = useCurrentModule();
const nav = useNavigation();

const { loading, nodes } = useRun(toRef(props, "rootId"), { live: props.live });
</script>
<template>
  <div v-if="loading">
    <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-400" />
  </div>
  <div v-else-if="layout == 'list'" class="flex flex-col">
    {{ nodes?.length }}
    <div v-for="node in nodes" :key="node.id">
      <span>{{ node.runnable?.name }}</span>
    </div>
  </div>
</template>
