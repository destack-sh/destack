<template>
  <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
    <div>
      <FunctionHandlerSelect label="Function" v-model="selectedFunctionHandler" />
    </div>
    <div class="grid grid-cols-1 gap-y-6 gap-x-4 pt-4 sm:grid-cols-6">
      <div class="sm:col-span-4">
        <label for="name" class="block text-sm font-medium text-gray-700"> Node name </label>
        <div class="mt-1 flex rounded-md shadow-sm">
          <input
            v-model="name"
            type="text"
            name="name"
            id="name"
            autocomplete="name"
            minlength="3"
            maxlength="64"
            required
            class="block w-full min-w-0 flex-1 rounded-none rounded-r-md border-gray-300 focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
          />
        </div>
      </div>
    </div>
    <div class="pt-4" v-if="selectedFunctionHandler">
      <RecordForm :spec="selectedFunctionHandler.config_spec" v-model="nodeConfigRecord" />
    </div>
    <div class="pt-4">
      <div class="flex justify-end">
        <!-- TODO @Feature: use proper form validation -->
        <button
          type="submit"
          class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click.prevent="create"
        >
          Create
        </button>
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import type { ArtifactVersion, FlowNode, FunctionHandlerSpec } from "@/types";
import { ref, type Ref } from "vue";
import FunctionHandlerSelect from "./FunctionHandlerSelect.vue";
import RecordForm from "./RecordForm.vue";

const selectedFunctionHandler: Ref<FunctionHandlerSpec | null> = ref(null);

const name: Ref<string> = ref("");
const nodeConfigRecord: Ref<Record<string, any>> = ref({});
const metadata: Ref<Record<string, any>> = ref({});
const connectedArtifacts: Ref<Record<string, ArtifactVersion>> = ref({});

function create() {
  const node = {
    name: name.value,
    function_id: selectedFunctionHandler.value?.name,
    config_arguments: nodeConfigRecord.value,
    metadata: metadata.value,
  } as FlowNode;
  emit("create", { node, connectedArtifacts: connectedArtifacts.value });
}

const props = defineProps<{ modelValue?: FlowNode }>();
const emit = defineEmits<{
  (
    e: "create",
    value: {
      node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
      connectedArtifacts: Record<string, ArtifactVersion>;
    }
  ): void;
  (e: "update:modelValue", value: FlowNode): void;
  (e: "update:connectedArtifact", value: { name: string; artifact: ArtifactVersion | null }): void;
}>();
</script>
