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
        <div
          v-if="modelSpec != null && runtimeModelSpec != null && modelSpec != runtimeModelSpec"
          class="flex justify-end"
        >
          <SButton variant="solid" color="slate" text="Reset spec" @click="restoreRuntimeSpec" />
        </div>
        <div class="flex flex-row gap-2" v-if="modelSpec?.input_spec != null && modelSpec?.output_spec != null">
          <RecordSpecDisplay class="flex-1" :spec="modelSpec.input_spec" />
          <RecordSpecDisplay class="flex-1" :spec="modelSpec.output_spec" />
        </div>
      </div>
      <CommitEditor
        :suggestedTitle="isFirst ? 'Create model' : 'Update model'"
        @commit="({ commitTitle, commitDescription }) => commit(commitTitle, commitDescription)"
      />
    </form>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import ModelHandlerSelect from "@/components/ModelHandlerSelect.vue";
import RecordForm from "@/components/RecordForm.vue";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore, useMetaStore } from "@/stores";
import type { ArtifactVersion, ModelHandlerSpec, ModelMetadata, ModelSpecEditable, ModelType } from "@/types";
import { computed, ref, watch, type Ref } from "vue";
import { useRouter } from "vue-router";
import SButton from "@/components/basic/SButton.vue";
import CommitEditor from "@/components/CommitEditor.vue";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";

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

const isFirst = computed(() => props.parent == null);
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
async function commit(commitTitle: string, commitDescription?: string) {
  if (selectedHandler.value == null) {
    throw new Error("no model handler selected");
  }

  const metadata: ModelMetadata = {
    handler_id: selectedHandler.value.id,
    config_arguments: modelConfigRecord.value,
    input_spec: modelSpec.value?.input_spec,
    output_spec: modelSpec.value?.output_spec,
  };
  const newVersion = {
    parents: props.parent ? [props.parent] : [],
    metadata,
    name: commitTitle,
    description: commitDescription || null,
  } as Partial<ArtifactVersion>;

  await artifactsStore.commitArtifactVersion(props.model, newVersion);
  router.push(`/models/${props.model}`);
}
</script>
