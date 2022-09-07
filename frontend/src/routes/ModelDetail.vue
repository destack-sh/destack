<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-semibold text-gray-900">{{ props.model }}</h1>
          <h3 class="text-lg text-gray-900">
            {{ model?.description }}
            <span class="italic text-gray-700" v-if="!model?.description">No description yet</span>
          </h3>
        </div>
        <div>
          <SButton variant="outline" color="slate" @click="_delete">
            Delete
            <TrashIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
          </SButton>
        </div>
      </div>
      <div class="mx-auto flex w-full justify-end">
        <div v-if="model?.head">
          <div class="mt-6">
            <router-link
              class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
              :to="`/models/${props.model}/versions/${model.head?.version}`"
            >
              {{ model.head.version }}
              {{ latestVersionDtFromNow }}
            </router-link>
            <router-link
              :to="`/models/${props.model}/versions`"
              class="ml-1 rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
            >
              {{ versionsPaginated?.count || 0 }} versions
            </router-link>
          </div>
        </div>
      </div>
    </div>
    <div v-if="versionsPaginated?.count == 0" class="mt-10 text-center">
      <CpuChipIcon class="mx-auto h-12 w-12 text-gray-400" />
      <h3 class="mt-2 text-sm font-medium text-gray-900">Empty model</h3>
      <p class="mt-1 text-sm text-gray-500">Get started by initializing from a template</p>
      <div class="mt-6">
        <SButton variant="solid" color="orange" :to="`/models/${props.model}/edit`">
          <PlusIcon class="-ml-1 mr-2 h-5 w-5" aria-hidden="true" />
          Select template
        </SButton>
      </div>
    </div>
    <div v-else class="mt-10 text-center">
      <!-- TODO @Feature: display model config & spec more attractively -->
      <SButton
        variant="solid"
        color="orange"
        :to="{
          path: `/models/${props.model}/edit`,
          query: { parent: model?.head?.version },
        }"
      >
        Edit
        <PencilIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
      </SButton>
      <h3 class="mt-2 text-sm font-medium text-gray-900">Config</h3>
      {{ latestMetadata?.handler_id }}
      {{ model?.head?.storage_uri }}
      {{ latestMetadata?.config_arguments }}
      <h3 class="mt-2 text-sm font-medium text-gray-900">Spec</h3>
      <div
        class="flex max-w-xl flex-row gap-2 self-center p-3"
        v-if="latestMetadata?.input_spec != null && latestMetadata?.output_spec != null"
      >
        <RecordSpecDisplay class="flex-1" :spec="latestMetadata?.input_spec" />
        <RecordSpecDisplay class="flex-1" :spec="latestMetadata?.output_spec" />
      </div>
    </div>
  </Sidebar>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import RecordSpecDisplay from "@/components/RecordSpecDisplay.vue";
import Sidebar from "@/components/Sidebar.vue";
import { computedAsync, useArtifactsStore } from "@/stores";
import type { ModelMetadata } from "@/types";
import { CpuChipIcon, PencilIcon, PlusIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref } from "@vue/reactivity";
import { DateTime } from "luxon";
import { useRouter } from "vue-router";
import SButton from "@/components/basic/SButton.vue";

const props = defineProps<{ model: string }>();

const artifactsStore = useArtifactsStore();
const model = computed(() => artifactsStore.artifact(props.model));
const latestMetadata = computed(() => {
  if (model.value?.head == null) {
    return null;
  } else {
    return model.value?.head.metadata as ModelMetadata;
  }
});

const { result: versionsPaginated } = computedAsync(() => artifactsStore.getVersions(props.model));
const latestVersionDtFromNow: Ref<string | null> = computed(() => {
  if (model.value?.head == null) return null;
  return DateTime.fromISO(model.value.head.created_at).toRelative({ locale: "en-US" });
});

const router = useRouter();
async function _delete() {
  await api.delete(`/models/${props.model}`);
  await artifactsStore.hydrate();
  router.push("/models");
}
</script>
