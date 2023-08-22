<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { useBenchState, usePanelContext } from "@/state/bench";
import { computed } from "vue";

const props = defineProps<{
  thing?: { id: string; deletedAt?: string | null; projectVersion: { id: string } | null } | null;
  name: string;
  isLoading: boolean;
}>();
const emit = defineEmits<{
  (e: "restore"): void;
}>();

const now = useTimeFromNow();
const bench = useBenchState();
const panel = usePanelContext();

const isDeleted = computed(() => props.thing?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    bench.projectVersionId != null && props.thing != null && props.thing?.projectVersion?.id != bench.projectVersionId
);
const nameCamelCase = computed(() => props.name[0].toUpperCase() + props.name.slice(1));
</script>
<template>
  <!-- Deleted thing status and restore -->
  <div v-if="isDeleted && thing != null" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 py-2">
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-bold text-white">
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
    v-else-if="!isLoading && !isDeleted && isOtherVersion"
    class="sticky top-0 z-10 -mr-12 w-full bg-yellow-600 py-2"
  >
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-bold text-white">This {{ name }} is from another Bench version.</div>
      <router-link
        class="text-sm text-white underline decoration-dashed underline-offset-4 hover:decoration-solid"
        :to="{ query: { version: thing?.projectVersion?.id } }"
      >
        Go there
      </router-link>
    </div>
  </div>
  <!-- Thing failed to load -->
  <div v-else-if="!isLoading && thing == null" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 py-2">
    <div
      class="mx-auto flex flex-row items-center justify-center gap-2"
      :style="panel.panel.value.contentWidthAsMaxWidth"
    >
      <div class="text-sm font-bold text-white">{{ nameCamelCase }} failed to load.</div>
    </div>
  </div>
</template>
