<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { useBenchState, type FileHeader, type ViewId } from "@/state/bench";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{
  files?: FileHeader[];
}>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const bench = useBenchState();

const filesSorted = computed(() => {
  const files = props.files?.filter((f) => f.deletedAt == null && !f.directory && !f.generated) ?? [];
  return files.sort((a, b) => {
    return a.path.localeCompare(b.path);
  });
});
const focusedFileId = computed(() => filesSorted.value.find((f) => f.id == bench.focusedFileId)?.id);
const filesGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  filesSorted,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusFile(file: FileHeader) {
  const focusedViewId = bench.focusedViewId;
  bench.focusFile(file);
  bench.focusView(focusedViewId as ViewId); // keep focused view
}

function focusFileAndGoThere(file: FileHeader) {
  bench.focusFile(file);
}

// blur focused file if clicking outside file explorer
const listRef: Ref<HTMLDivElement | null> = ref(null);
const { focused: listRefFocused } = useFocusWithin(listRef);

function focus(target?: "first" | "last") {
  // focus currently focused file if nothing was directly selected (and thus focused)
  if (!target && focusedFileId.value && !listRefFocused.value) {
    nextTick(() => filesGrid.focus(focusedFileId.value, "name"));
  } else if (!listRefFocused.value && (props.files?.length ?? 0) > 0) {
    nextTick(() => filesGrid.focus(target == "first" ? 0 : -1, "name"));
  }
}

function blur() {
  filesGrid.blur();
}

defineExpose({
  count: computed(() => filesSorted.value.length),
  focus,
  blur,
});
</script>
<template>
  <!-- Panel: file explorer -->
  <ul ref="listRef" role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="file in filesSorted"
      :key="file.id"
      :ref="(ref) => filesGrid.registerColumnRef(file.id, 'name', ref)"
      tabindex="-1"
      @keydown.up.exact.prevent="filesGrid.navigateUp(file.id, 'name')"
      @keydown.down.exact.prevent="filesGrid.navigateDown(file.id, 'name')"
      class="relative max-w-full border border-transparent px-3 py-0.5 outline-none hover:cursor-pointer focus:border-orange-600"
      :class="{
        'bg-orange-100 text-orange-600': file.id == bench?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != bench?.focusedFileId,
        'border-l-2 border-l-gray-300 pl-2.5': file.generated && file.id != bench?.focusedFileId,
      }"
      @click="focusFile(file)"
      @keydown.enter.exact.prevent="focusFileAndGoThere(file)"
    >
      <!-- Path -->
      <span
        class="decoration-none inline select-none truncate text-ellipsis rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
      >
        {{ file.path.length > 0 ? file.path : "(Untitled)" }}
      </span>
    </li>
  </ul>
</template>
