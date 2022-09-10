<template>
  <Sidebar>
    <ArtifactHeader class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8" :artifact="props.dataset" />
    <div class="mx-auto mt-10 max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <div v-if="metadata?.record_spec">
        <h3 class="mt-2 text-sm font-medium text-gray-900">Spec</h3>
        <RecordSpecDisplay class="flex-1" :spec="metadata?.record_spec" />
      </div>
    </div>
    <div class="max-auto mt-10 max-w-7xl px-4 pt-6 sm:px-6 md:px-8" v-if="metadata?.record_spec">
      <RecordsTable :spec="metadata?.record_spec" :records="recordsInView" :style="'full'">
        <template v-slot:empty>
          <div class="flex justify-center py-4 px-4 text-sm font-normal">
            <SButton text="Create record" @click="promptCreateRecord" />
          </div>
        </template>
      </RecordsTable>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import Sidebar from "@/components/Sidebar.vue";
import { useArtifactsStore } from "@/stores";
import type { DatasetMetadata } from "@/types";
import { computed, ref } from "vue";
import ArtifactHeader from "@/components/ArtifactHeader.vue";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";
import RecordsTable from "../components/RecordsTable.vue";
import SButton from "../components/basic/SButton.vue";
import { computedAsync } from "@vueuse/core";

const props = defineProps<{ dataset: string }>();

const artifactsStore = useArtifactsStore();
const dataset = computed(() => artifactsStore.artifact(props.dataset));
const metadata = computed(() => {
  if (dataset.value?.head == null) {
    return null;
  } else {
    return dataset.value?.head.metadata as DatasetMetadata;
  }
});

const recordsLoading = ref(true);
const recordsInView = computedAsync(
  async () => (await artifactsStore.getDatasetRecords(props.dataset)).results,
  [],
  recordsLoading
);

function promptCreateRecord() {}
</script>
