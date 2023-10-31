<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { useBenchState, useBenchVersioning, usePanelContext } from "@/state/bench";
import { IS_DEBUG } from "@/utils/globals";
import { computed } from "vue";

const props = defineProps<{
  thing?: { id: string; deletedAt?: string | null; projectVersion?: { id: string } | null } | null;
  name: string;
  error?: any | null;
  loading: boolean;
}>();
const emit = defineEmits<{
  (e: "restore"): void;
}>();

const now = useTimeFromNow();
const bench = useBenchState();
const versioning = useBenchVersioning();
const panel = usePanelContext();

const isDeleted = computed(() => props.thing?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    props.thing?.projectVersion != null &&
    bench.projectVersionId != null &&
    props.thing != null &&
    props.thing?.projectVersion?.id != bench.projectVersionId
);
const nameCamelCase = computed(() => props.name[0].toUpperCase() + props.name.slice(1));
</script>
<template>
  <!-- Deleted thing status and restore -->
  <div v-if="isDeleted && thing != null" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 py-0.5">
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-semibold text-white">
        This {{ name }} is in trash (was deleted {{ now.getTimeFromNowLongString(thing.deletedAt as string) }}).
      </div>
      <button
        class="text-sm text-white underline decoration-dashed underline-offset-4 hover:decoration-solid"
        @click="emit('restore')"
      >
        Restore
      </button>
    </div>
  </div>
  <!-- Other version thing -->
  <div
    v-else-if="!loading && !isDeleted && isOtherVersion"
    class="sticky top-0 z-10 -mr-12 w-full bg-yellow-600 py-0.5"
  >
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-semibold text-white">This {{ name }} is from another version.</div>
      <router-link
        class="text-sm text-white underline decoration-dashed underline-offset-2 hover:decoration-solid"
        :to="{ query: { version: thing?.projectVersion?.id } }"
      >
        Go there
      </router-link>
    </div>
  </div>
  <!-- Restore thing -->
  <div
    v-else-if="!loading && !isDeleted && !bench.isAtHead"
    class="sticky top-0 z-10 -mr-12 w-full bg-yellow-600 py-0.5"
  >
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-semibold text-white">
        This {{ name }} is from version {{ versioning.currentVersionName.value }}.
      </div>
      <button
        v-if="bench.canEdit && thing != null && false /* TODO @Feature: restore nodes :BE-399 */"
        class="text-sm text-white underline decoration-dashed underline-offset-2 hover:text-gray-200 hover:decoration-solid"
        @click="versioning.restoreNode(thing?.id as string)"
      >
        Restore {{ name }}
      </button>
    </div>
  </div>
  <!-- Thing failed to load -->
  <div
    v-else-if="!loading && thing == null"
    class="sticky top-0 z-10 -mr-12 w-full py-0.5"
    :class="error ? 'bg-red-600' : 'bg-yellow-600'"
  >
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div v-if="!error || !IS_DEBUG" class="text-sm font-semibold text-white">
        {{ nameCamelCase }} does not exist in this version.
      </div>
      <div v-else class="text-sm font-semibold text-white">
        {{ nameCamelCase }} failed to load: <span class="font-mono font-normal">{{ error }}</span>
      </div>
    </div>
  </div>
</template>
