<template>
  <Sidebar>
    <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
      <div>
        <div>
          <h3 class="text-lg font-medium leading-6 text-gray-900">Create a new model</h3>
          <p class="mt-1 text-sm text-gray-500">
            Choose from ready-to-use templates or connect your own
          </p>
        </div>

        <div class="mt-6 grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
          <div class="sm:col-span-4">
            <label for="name" class="block text-sm font-medium text-gray-700"> Model name </label>
            <div class="mt-1 flex rounded-md shadow-sm">
              <span
                class="inline-flex items-center rounded-l-md border border-r-0 border-gray-300 bg-gray-50 px-3 text-gray-500 sm:text-sm"
              >
                symbolx.co/models/
              </span>
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

          <div class="sm:col-span-6">
            <label for="description" class="block text-sm font-medium text-gray-700">
              Description <span class="font-normal text-gray-500">(optional)</span>
            </label>
            <div class="mt-1">
              <textarea
                v-model="description"
                id="description"
                name="description"
                rows="1"
                class="block w-full rounded-md border border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
              />
            </div>
          </div>
        </div>
      </div>
      <div>
        <ModelTemplateSelect v-model="selectedTemplate" />
        <RecordForm
          class="mt-3"
          v-if="!(selectedTemplate as EmptyTemplate).empty"
          :spec="(selectedTemplate as ModelTemplate).handler.config_spec"
          v-model="modelConfigRecord"
        />
      </div>

      <div class="pt-5">
        <div class="flex justify-end">
          <!-- TODO @Feature: use proper form validation -->
          <button
            type="submit"
            class="ml-3 inline-flex justify-center rounded-md border border-transparent bg-orange-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
            @click.prevent="submit"
          >
            Create
          </button>
        </div>
      </div>
    </form>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import { useArtifactsStore } from "@/stores";
import type {
  Artifact,
  ArtifactVersion,
  EmptyTemplate,
  ModelMetadata,
  ModelTemplate,
} from "@/types";
import { ref, type Ref } from "vue";
import { useRouter } from "vue-router";
import ModelTemplateSelect from "@/components/ModelTemplateSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";

const name: Ref<string> = ref("");
const description: Ref<string> = ref("");

const router = useRouter();
const artifactsStore = useArtifactsStore();

const selectedTemplate: Ref<ModelTemplate | EmptyTemplate> = ref({
  name: "No template",
  empty: true,
} as EmptyTemplate);
const modelConfigRecord: Ref<Record<string, any>> = ref({});

async function submit() {
  const artifact = await api
    .post<Artifact>("/models", {
      name: name.value,
      description: description.value,
    })
    .then((response) => response.data);

  if (selectedTemplate.value != null) {
    // TODO @Robustness: create model and initialize from template should be atomic
    // initialize repo
    const metadata: ModelMetadata = {
      handler_id: (selectedTemplate.value as ModelTemplate).handler.name,
      config_arguments: modelConfigRecord.value,
    };
    await api.post<ArtifactVersion>(`/models/${artifact.name}/versions`, {
      parents: [], // initial version
      metadata,
      name: "Create model",
      description: `Initialized from template ${selectedTemplate.value.name}`,
    } as Partial<ArtifactVersion>);
  }

  // clear and redirect to model
  artifactsStore.hydrate();
  router.push(`/models/${artifact.name}`);
}
</script>
