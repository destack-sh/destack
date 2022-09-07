<template>
  <div>
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-semibold text-gray-900">{{ props.artifact }}</h1>
        <h3 class="text-lg text-gray-900">
          {{ artifact?.description }}
          <span class="italic text-gray-700" v-if="!artifact?.description">No description yet</span>
        </h3>
      </div>
      <div class="flex flex-row gap-2">
        <!-- <SButton variant="outline" color="slate" @click="_delete">
          Delete
          <TrashIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
        </SButton> -->
        <SButton variant="solid" color="slate" :to="`/${artifact.type}s/${props.artifact}/settings`">
          Settings
          <Cog6ToothIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
        </SButton>
        <SButton
          variant="solid"
          color="slate"
          :to="{
            path: `/${artifact.type}s/${props.artifact}/edit`,
            query: { parent: artifact?.head?.version },
          }"
        >
          Edit
          <PencilIcon class="ml-2 -mr-1 h-5 w-5" aria-hidden="true" />
        </SButton>
      </div>
    </div>
    <div class="mx-auto flex w-full">
      <div v-if="artifact?.head">
        <div class="mt-6">
          <router-link
            class="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
            :to="`/${artifact.type}s/${props.artifact}/versions/${artifact.head?.version}`"
          >
            {{ artifact.head.version }}
            {{ headDtFromNow }}
          </router-link>
          <router-link
            :to="`/${artifact.type}s/${props.artifact}/versions`"
            class="ml-1 rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50"
          >
            {{ versionsPaginated?.count || 0 }} versions
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { api } from "@/api";
import router from "@/router";
import { computedAsync, useArtifactsStore } from "@/stores";
import { Cog6ToothIcon, PencilIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { DateTime } from "luxon";
import { computed, type Ref } from "vue";
import SButton from "@/components/basic/SButton.vue";

const props = defineProps<{ artifact: string }>();

const artifactsStore = useArtifactsStore();
const artifact = computed(() => artifactsStore.artifact(props.artifact));

const { result: versionsPaginated } = computedAsync(() => artifactsStore.getVersions(props.model));
const headDtFromNow: Ref<string | null> = computed(() => {
  if (artifact.value?.head == null) return null;
  return DateTime.fromISO(artifact.value.head.created_at).toRelative({ locale: "en-US" });
});

async function _delete() {
  await api.delete(`/models/${props.artifact}`);
  await artifactsStore.hydrate();
  router.push("/models");
}
</script>
