<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl justify-between px-4 pt-6 sm:flex sm:items-center sm:gap-4 sm:px-6 md:px-8">
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ flow?.flow }}</h1>
        <h3 class="text text-gray-700" v-if="flow">created {{ getTimeFromNowString(flow.created_at) }}</h3>
      </div>
      <div>
        <router-link :to="`/flows/${props.flow}/edit`">
          <button
            type="button"
            class="inline-flex items-center rounded-md border border-transparent bg-orange-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          >
            Edit
            <PencilIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
          </button>
        </router-link>
      </div>
    </div>
    <FlowGraphInterface
      v-if="flow"
      class="px-4 pt-6 sm:gap-4 sm:px-6 md:px-8"
      :flow="flow"
      v-model:runtimeData="runtimeData"
      v-model:interactionData="interactionData"
      @submit-input="execute"
    />
    <!-- Executions & output -->
    <div class="px-4 pt-6 sm:gap-4 sm:px-6 md:px-8">
      <div class="border-b border-gray-200 pb-3 sm:flex sm:items-center sm:justify-between">
        <h3 class="text-lg font-medium leading-6 text-gray-900">Execution history</h3>
      </div>
      <ExecutionsGrid :executions="executions" class="" />
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import ExecutionsGrid from "@/components/ExecutionsGrid.vue";
import { useFlowExecution } from "@/composables/useFlowExecution";
import { useTimeFromNow } from "@/composables/useNow";
import { useFlowsStore } from "@/stores";
import { makeInteractionData, type FlowInteractionData, type FlowRuntimeData, type FlowVersion } from "@/types";
import { PencilIcon } from "@heroicons/vue/outline";
import { computed, ref, type Ref } from "vue";
import FlowGraphInterface from "../components/FlowGraphInterface.vue";
import Sidebar from "../components/Sidebar.vue";
const props = defineProps<{ flow: string }>();

const flowsStore = useFlowsStore();
const flow: Ref<FlowVersion | null> = computed(() => flowsStore.flow(props.flow)?.latest_version || null);
const runtimeData: Ref<FlowRuntimeData> = ref({});
const interactionData: Ref<FlowInteractionData> = ref(makeInteractionData());

const { getTimeFromNowString } = useTimeFromNow();
const { executions, execute } = useFlowExecution(flow, runtimeData, ref(true));
</script>
