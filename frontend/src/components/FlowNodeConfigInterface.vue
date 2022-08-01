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
      <RecordForm
        :spec="reduceToFieldSpec(selectedFunctionHandler.config_spec)"
        v-model="configRecord"
      />
      <ConfigForm :spec="selectedFunctionHandler.config_spec" v-model="configRecord" />
    </div>
    <div class="pt-4">
      <div class="flex justify-end">
        <!-- TODO @Feature: use proper form validation -->
        <button
          type="submit"
          class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
          @click.prevent="submit"
        >
          {{ creating ? "Create" : "Update" }}
        </button>
      </div>
    </div>
  </form>
</template>
<script lang="ts" setup>
import ConfigForm from "@/components/ConfigForm.vue";
import FunctionHandlerSelect from "@/components/FunctionHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import { useMetaStore } from "@/stores";
import {
  reduceToFieldSpec,
  type ArtifactVersion,
  type FlowNode,
  type FlowVersion,
  type FunctionHandlerSpec,
} from "@/types";
import { computed, ref, watchEffect, type Ref } from "vue";

const selectedFunctionHandler: Ref<FunctionHandlerSpec | null> = ref(null);

const name: Ref<string> = ref("");
const configRecord: Ref<Record<string, any>> = ref({});
const metadata: Ref<Record<string, any>> = ref({});
const connectedArtifacts: Ref<Record<string, ArtifactVersion>> = ref({});

const props = defineProps<{ existingNode?: FlowNode; flow: FlowVersion }>();
const emit = defineEmits<{
  (
    e: "create",
    value: {
      node: Pick<FlowNode, "name" | "function_id" | "config_arguments" | "metadata">;
      connectedArtifacts: Record<string, ArtifactVersion>;
    }
  ): void;
  (
    e: "update",
    value: { node: FlowNode; connectedArtifacts: Record<string, ArtifactVersion> }
  ): void;
}>();

const creating = computed(() => props.existingNode == null);
// initialize forms if not creating
const metaStore = useMetaStore();
watchEffect(() => {
  if (props.existingNode == null) {
    return;
  }
  name.value = props.existingNode.name;
  selectedFunctionHandler.value = metaStore.functionHandlersByName[props.existingNode.function_id];
  configRecord.value = props.existingNode.config_arguments || {};
  // TODO @Broken: init/recover existing artifact connections when editing flow
});

function submit() {
  if (creating.value) {
    create();
  } else {
    update();
  }
}

function create() {
  const node = {
    name: name.value,
    function_id: selectedFunctionHandler.value?.name,
    config_arguments: configRecord.value,
    metadata: metadata.value,
  } as FlowNode;
  emit("create", { node, connectedArtifacts: connectedArtifacts.value });
}

function update() {
  const node = props.existingNode as FlowNode;
  const updatedNode = {
    ...node,
    name: name.value,
    function_id: selectedFunctionHandler.value?.name,
    config_arguments: configRecord.value,
    metadata: metadata.value,
  } as FlowNode;
  emit("update", { node: updatedNode, connectedArtifacts: connectedArtifacts.value });
}
</script>
