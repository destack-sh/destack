<template>
  <Combobox :model-value="modelValue" @update:modelValue="onSelect">
    <div class="relative mt-1 w-full">
      <ComboboxInput
        class="w-full rounded-md border border-gray-300 bg-white py-2 pl-3 pr-10 shadow-sm focus:border-orange-500 focus:outline-none focus:ring-1 focus:ring-orange-500 sm:text-sm"
        @change="query = $event.target.value"
        :display-value="(artifact: unknown) => (artifact as Artifact | null)?.name || ''"
        placeholder="Select artifact"
      />
      <ComboboxButton
        v-if="!props.static"
        class="absolute inset-y-0 right-0 flex items-center rounded-r-md px-2 focus:outline-none"
      >
        <SelectorIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
      </ComboboxButton>

      <ComboboxOptions
        :static="props.static"
        :class="props.static ? 'py-1' : 'absolute z-10 bg-white py-2 shadow-lg ring-1 ring-black ring-opacity-5'"
        class="z-10 mt-1 -mb-2 max-h-72 w-full scroll-py-2 overflow-auto overflow-y-auto rounded-md text-base text-gray-800 focus:outline-none sm:text-sm"
      >
        <ComboboxOption
          v-for="artifact in filteredArtifacts"
          :key="artifact.id"
          :value="artifact"
          as="template"
          v-slot="{ active }"
        >
          <li :class="['cursor-default select-none rounded-md px-4 py-2', active && 'bg-orange-600 text-white']">
            {{ artifact.name }}
          </li>
        </ComboboxOption>
        <div
          v-if="(query !== '' || relevantArtifacts.length == 0) && filteredArtifacts.length === 0"
          class="py-4 px-4 text-center sm:px-14"
        >
          <Manif class="mx-auto h-6 w-6 text-gray-400" aria-hidden="true" />
          <p class="mt-4 text-sm text-gray-900">No matching artifacts found matching these filters.</p>
          <p v-if="props.type" class="mt-4 text-sm text-gray-900">
            <template v-if="props.type._type == 'DatasetType'">
              Using spec filter
              <RecordSpecDisplay :spec="(props.type as DatasetType).record_spec" />
            </template>
          </p>
        </div>
      </ComboboxOptions>
    </div>
  </Combobox>
</template>

<script lang="ts" setup>
import { useArtifactsStore } from "@/stores";
import type { Artifact, ArtifactType, DatasetMetadata, DatasetType, RecordSpec } from "@/types";
import { specContains } from "@/utils/spec";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { ChipIcon, SelectorIcon } from "@heroicons/vue/outline";
import { computed, ref, type Ref } from "vue";
import RecordSpecDisplay from "./RecordSpecDisplay.vue";

const props = defineProps<{ type?: ArtifactType; modelValue?: Artifact; static?: boolean }>();

const query: Ref<string> = ref("");

function getDatasetRecordSpec(dataset: Artifact): RecordSpec | undefined {
  if (dataset.latest_version == undefined) {
    return undefined;
  }

  const datasetMetadata = dataset.latest_version.metadata as DatasetMetadata;
  return datasetMetadata.record_spec;
}

const artifactsStore = useArtifactsStore();
const relevantArtifacts = computed(() => {
  let relevantArtifacts: Artifact[];
  if (props.type?._type == "DatasetType") {
    relevantArtifacts = artifactsStore.datasets.filter((dataset) => {
      if (dataset.latest_version == null) {
        return false;
      }

      const datasetSpec = getDatasetRecordSpec(dataset);
      return datasetSpec != null && specContains(datasetSpec, (props.type as DatasetType).record_spec);
    });
  } else if (props.type?._type == "ModelType") {
    relevantArtifacts = artifactsStore.models;
  } else {
    relevantArtifacts = artifactsStore.artifacts;
  }
  return relevantArtifacts;
});

const filteredArtifacts = computed(() => {
  return relevantArtifacts.value.filter((artifact) => {
    if (!props.static && query.value.trim().length == 0) {
      return [];
    }
    return artifact.name.toLowerCase().includes(query.value.toLowerCase());
  });
});

const emit = defineEmits(["select", "update:modelValue"]);
function onSelect(artifact: Artifact) {
  emit("select", artifact);
  emit("update:modelValue", artifact);
}
</script>
