<template>
  <Sidebar>
    <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ modelName }}</h1>
        <ModelTemplateSelect v-model="selectedTemplate" />
        <RecordForm
          class="mt-3"
          v-if="!(selectedTemplate as EmptyTemplate).empty"
          :spec="(selectedTemplate as ModelTemplate).handler.config_spec"
          v-model="modelConfigRecord"
        />
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
import ModelTemplateSelect from "@/components/ModelTemplateSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore, useMetaStore } from "@/stores";
import type { ArtifactVersion, EmptyTemplate, ModelMetadata, ModelTemplate } from "@/types";
import { computed, ref, watch, watchEffect, type Ref } from "vue";
import { useRouter } from "vue-router";

const props = defineProps({
  modelName: { type: String, required: true },
  parent: { type: String, required: false },
});
const emptyTemplate = {
  name: "No template",
  empty: true,
} as EmptyTemplate;
const selectedTemplate: Ref<ModelTemplate | EmptyTemplate> = ref(emptyTemplate);
const modelConfigRecord: Ref<Record<string, any>> = ref({});
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
    return artifactsStore.getVersion(props.modelName, props.parent);
  } else {
    return Promise.resolve(null);
  }
});

const metaStore = useMetaStore();
// sync parentVersion -> modelConfigRecord
watch(parentVersion, () => {
  if (parentVersion.value != null) {
    const parentMetadata = parentVersion.value.metadata as ModelMetadata;
    selectedTemplate.value = metaStore.templateFor(parentMetadata.handler_id) || emptyTemplate;
    modelConfigRecord.value = parentMetadata.config_arguments;
  }
});

const router = useRouter();
async function commit() {
  const metadata: ModelMetadata = {
    handler_id: (selectedTemplate.value as ModelTemplate).handler.name,
    config_arguments: modelConfigRecord.value,
  };
  await api.post<ArtifactVersion>(`/models/${props.modelName}/versions`, {
    parents: [],
    metadata,
    name: commitTitle.value,
    description: commitDescription.value || null,
  } as Partial<ArtifactVersion>);
  router.push(`/models/${props.modelName}`);
}
</script>
