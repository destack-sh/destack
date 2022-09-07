<template>
  <Sidebar>
    <ArtifactHeader class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8" :artifact="props.model" />
    <div class="mx-auto mt-10 max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <!-- TODO @Feature: display model config & spec more attractively -->
      <h3 class="mt-2 text-sm font-medium text-gray-900">Config</h3>
      {{ metadata?.handler_id }}
      {{ model?.head?.storage_uri }}
      {{ metadata?.config_arguments }}
      <h3 class="mt-2 text-sm font-medium text-gray-900">Spec</h3>
      <div
        class="flex max-w-xl flex-row gap-2 self-center p-3"
        v-if="metadata?.input_spec != null && metadata?.output_spec != null"
      >
        <RecordSpecDisplay class="flex-1" :spec="metadata?.input_spec" />
        <RecordSpecDisplay class="flex-1" :spec="metadata?.output_spec" />
      </div>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import ArtifactHeader from "@/components/ArtifactHeader.vue";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";
import Sidebar from "@/components/Sidebar.vue";
import { useArtifactsStore } from "@/stores";
import type { ModelMetadata } from "@/types";
import { computed } from "@vue/reactivity";
import { useRouter } from "vue-router";

const props = defineProps<{ model: string }>();

const artifactsStore = useArtifactsStore();
const model = computed(() => artifactsStore.artifact(props.model));
const metadata = computed(() => {
  if (model.value?.head == null) {
    return null;
  } else {
    return model.value?.head.metadata as ModelMetadata;
  }
});

const router = useRouter();
</script>
