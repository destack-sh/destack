<template>
  <Sidebar>
    <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
      <div>
        <div>
          <h3 class="text-lg font-medium leading-6 text-gray-900">Create a new model</h3>
          <p class="mt-1 text-sm text-gray-500">Choose from ready-to-use templates or connect your own</p>
        </div>

        <div class="mt-6 grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
          <TextInput label="Name" v-model="name" class="sm:col-span-4" />
          <TextInput label="Description" v-model="description" optional class="sm:col-span-6" />
        </div>
      </div>
      <div>
        <ModelHandlerSelect v-model="selectedHandler" />
        <RecordForm
          class="mt-3"
          v-if="selectedHandler != null"
          :spec="Object.values(selectedHandler.config_spec)"
          v-model="modelConfigRecord"
        />
      </div>

      <div class="flex justify-end pt-5">
        <!-- TODO @Feature: use proper form validation -->
        <SButton type="submit" variant="solid" color="orange" @click.prevent="submit"> Create </SButton>
      </div>
    </form>
  </Sidebar>
</template>
<script lang="ts" setup>
import ModelHandlerSelect from "@/components/ModelHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import { useArtifactsStore } from "@/stores";
import type { ArtifactVersion, ModelHandlerSpec, ModelMetadata } from "@/types";
import { ref, type Ref } from "vue";
import { useRouter } from "vue-router";
import TextInput from "@/components/basic/TextInput.vue";
import SButton from "../components/basic/SButton.vue";

const name: Ref<string> = ref("");
const description: Ref<string> = ref("");

const router = useRouter();
const artifactsStore = useArtifactsStore();

const selectedHandler: Ref<ModelHandlerSpec | null> = ref(null);
const modelConfigRecord: Ref<Record<string, any>> = ref({});

async function submit() {
  if (selectedHandler.value != null) {
    const metadata: ModelMetadata = {
      handler_id: selectedHandler.value.id,
      config_arguments: modelConfigRecord.value,
    };
    const initialVersion = {
      parents: [], // initial version
      metadata,
      name: "Create model",
      description: `Create new ${selectedHandler.value.id} model`,
    } as Partial<ArtifactVersion>;

    const artifact = await artifactsStore.createArtifact("model", name.value, description.value, initialVersion);
    // clear and redirect to model
    router.push(`/models/${artifact.name}`);
  }
}
</script>
