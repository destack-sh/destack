<template>
  <Sidebar>
    <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ model }}</h1>
        <ModelHandlerSelect v-model="selectedHandler" />
        <RecordForm
          class="mt-3"
          v-if="selectedHandler != null"
          :spec="Object.values(selectedHandler.config_spec)"
          v-model="modelConfigRecord"
        />
        <ModelSpecSelect v-model="modelSpec" />
        <div
          v-if="modelSpec != null && runtimeModelSpec != null && modelSpec != runtimeModelSpec"
          class="flex justify-end"
        >
          <button
            class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click.prevent="restoreRuntimeSpec"
          >
            Reset to runtime spec
          </button>
        </div>
        <div class="flex flex-row gap-2" v-if="modelSpec?.input_spec != null && modelSpec?.output_spec != null">
          <RecordSpecDisplay class="flex-1" :spec="modelSpec.input_spec" />
          <RecordSpecDisplay class="flex-1" :spec="modelSpec.output_spec" />
        </div>
      </div>
      <div>
        <h3 class="mt-3 text-lg font-medium leading-6 text-gray-900">Commit changes</h3>
        <div class="mt-2 sm:col-span-4">
          <label for="name" class="sr-only"> Model name </label>
          <div class="mt-1 flex rounded-md shadow-sm">
            <input
              v-model="commitTitle"
              type="text"
              name="name"
              id="name"
              required
              class="block w-full min-w-0 flex-1 rounded-none rounded-r-md border-gray-300 focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
            />
          </div>
        </div>
        <div class="sm:col-span-6">
          <label for="description" class="sr-only">
            Description <span class="font-normal text-gray-500">(optional)</span>
          </label>
          <div class="mt-1">
            <textarea
              v-model="commitDescription"
              id="description"
              name="description"
              rows="3"
              class="block w-full rounded-md border border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
              placeholder="Add an optional extended description..."
            />
          </div>
        </div>
        <div class="pt-3">
          <div class="flex justify-end">
            <!-- TODO @Feature: use proper form validation -->
            <button
              type="submit"
              class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
              @click.prevent="commit"
            >
              Commit
            </button>
          </div>
        </div>
      </div>
    </form>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import ModelHandlerSelect from "@/components/ModelHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore, useMetaStore } from "@/stores";
import type { ModelSpecEditable, ModelMetadata, ArtifactVersion, ModelHandlerSpec, ModelType } from "@/types";
import { computed, ref, watch, watchEffect, type Ref } from "vue";
import { useRouter } from "vue-router";
import ModelSpecSelect from "../components/ModelSpecSelect.vue";
import RecordSpecDisplay from "../components/RecordSpecDisplay.vue";

const props = defineProps({
  model: { type: String, required: true },
  parent: { type: String, required: false },
});

const selectedHandler: Ref<ModelHandlerSpec | null> = ref(null);
const modelConfigRecord: Ref<Record<string, any>> = ref({});
const modelSpec: Ref<ModelSpecEditable> = ref({} as ModelSpecEditable);

const { result: runtimeModelSpec } = computedAsync(async () => {
  if (props.parent == null) {
    return null;
  }
  return await api
    .get<ModelType>(`/artifacts/${props.model}/versions/${props.parent}/spec`)
    .then((response) => response.data);
});

function restoreRuntimeSpec() {
  if (runtimeModelSpec.value != null) {
    modelSpec.value = runtimeModelSpec.value;
  }
}

const commitTitle: Ref<string> = ref("Initial commit");
const commitDescription: Ref<string> = ref("");
const isInitial = computed(() => props.parent == null);

// update commit title / description depending on initial
watchEffect(() => {
  if (isInitial.value) {
    commitTitle.value = `Create model`;
  } else {
    commitTitle.value = "Update model";
  }
});

const artifactsStore = useArtifactsStore();
const { result: parentVersion } = computedAsync(() => {
  if (props.parent != null) {
    return artifactsStore.getVersion(props.model, props.parent);
  } else {
    return Promise.resolve(null);
  }
});

const metaStore = useMetaStore();
// sync parent (or current version) -> local state
watch(parentVersion, () => {
  if (parentVersion.value != null) {
    const parentMetadata = parentVersion.value.metadata as ModelMetadata;
    selectedHandler.value = metaStore.modelHandler(parentMetadata.handler_id);
    modelConfigRecord.value = parentMetadata.config_arguments;
    modelSpec.value = {
      input_spec: parentMetadata.input_spec,
      output_spec: parentMetadata.output_spec,
    };
  }
});

const router = useRouter();
async function commit() {
  if (selectedHandler.value == null) {
    throw new Error("no model handler selected");
  }

  const metadata: ModelMetadata = {
    handler_id: selectedHandler.value.id,
    config_arguments: modelConfigRecord.value,
    input_spec: modelSpec.value?.input_spec,
    output_spec: modelSpec.value?.output_spec,
  };
  await api.post<ArtifactVersion>(`/models/${props.model}/versions`, {
    parents: [],
    metadata,
    name: commitTitle.value,
    description: commitDescription.value || null,
  } as Partial<ArtifactVersion>);
  await artifactsStore.hydrate();
  router.push(`/models/${props.model}`);
}
</script>
