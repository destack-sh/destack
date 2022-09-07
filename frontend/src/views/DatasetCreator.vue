<template>
  <form class="mx-auto max-w-xl space-y-8 divide-y divide-gray-200 pt-8" action="">
    <div>
      <div>
        <h3 class="text-lg font-medium leading-6 text-gray-900">Create a new dataset</h3>
        <p class="mt-1 text-sm text-gray-500">Choose from ready-to-use templates or connect your own</p>
      </div>

      <div class="mt-6 grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
        <TextInput label="Name" v-model="name" class="sm:col-span-4" />
        <TextInput label="Description" v-model="description" optional class="sm:col-span-6" />
      </div>
    </div>
    <div class="pt-2">
      <DatasetHandlerSelect v-model="handler" />
      <RecordForm
        class="mt-3"
        v-if="handler != null"
        :spec="Object.values(handler.config_spec)"
        v-model="datasetConfigRecord"
      />
    </div>

    <div class="pt-2">
      <label class="block text-sm font-medium text-gray-700"> Dataset spec </label>
      <RecordSpecDisplay v-if="spec" :spec="spec" />
      <!-- TODO @Feature: spec editor -->
      <span v-else>spec not defined</span>
    </div>

    <div class="flex justify-end pt-5">
      <!-- TODO @Feature: use proper form validation -->
      <SButton type="submit" variant="solid" color="orange" @click.prevent="submit"> Create </SButton>
    </div>
  </form>
</template>
<script lang="ts" setup>
import DatasetHandlerSelect from "@/components/DatasetHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import { useArtifactsStore } from "@/stores";
import type { ArtifactVersion, DatasetHandlerSpec, DatasetMetadata, RecordSpec } from "@/types";
import { ref, toRef, watch, type Ref } from "vue";
import { useRouter } from "vue-router";
import TextInput from "@/components/basic/TextInput.vue";
import SButton from "@/components/basic/SButton.vue";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";

const props = defineProps<{ suggested?: { name?: string; description?: string; spec: RecordSpec } }>();

const name: Ref<string> = ref("");
const description: Ref<string> = ref("");
const spec: Ref<RecordSpec | null> = ref(null);
const handler: Ref<DatasetHandlerSpec | null> = ref(null);
const datasetConfigRecord: Ref<Record<string, any>> = ref({});

watch(
  toRef(props, "suggested"),
  (suggested) => {
    if (suggested?.name) {
      name.value = suggested.name;
    }
    if (suggested?.description) {
      description.value = suggested.description;
    }
    if (suggested?.spec) {
      spec.value = suggested.spec;
    }
  },
  { immediate: true, deep: true }
);

const router = useRouter();
const artifactsStore = useArtifactsStore();

async function submit() {
  if (handler.value != null) {
    const initialVersion = {
      parents: [], // initial version
      metadata: {
        handler_id: handler.value.id,
        config_arguments: datasetConfigRecord.value,
        record_spec: spec.value,
      } as DatasetMetadata,
      name: "Create dataset",
      description: `Create new ${handler.value.id} dataset`,
    } as Partial<ArtifactVersion>;

    const artifact = await artifactsStore.createArtifact(
      "dataset",
      { name: name.value, description: description.value },
      initialVersion
    );
    router.push(`/datasets/${artifact.name}`);
  }
}
</script>
