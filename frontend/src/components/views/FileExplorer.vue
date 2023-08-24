<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { IssueKind } from "@/gql/graphql";
import { useBenchState, type FileHeader, type ViewId } from "@/state/bench";
import { useCurrentModule, type NodeBase } from "@/state/module";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const bench = useBenchState();
const module = useCurrentModule();

const filesSorted = computed(() => {
  if (module.idx.value == null) {
    return [];
  }
  const files = Object.values(module.idx.value.filesById)?.filter((f) => f.deletedAt == null);
  return files.sort((a, b) => {
    return a.name.localeCompare(b.name);
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
  bench.focusFile(file as NodeBase);
  bench.focusView(focusedViewId as ViewId); // keep focused view
}

function focusFileAndGoThere(file: FileHeader) {
  bench.focusFile(file as NodeBase);
}

// blur focused file if clicking outside file explorer
const listRef: Ref<HTMLDivElement | null> = ref(null);
const { focused: listRefFocused } = useFocusWithin(listRef);

function focus(target?: "first" | "last") {
  // focus currently focused file if nothing was directly selected (and thus focused)
  if (!target && focusedFileId.value && !listRefFocused.value) {
    nextTick(() => filesGrid.focus(focusedFileId.value, "name"));
  } else if (!listRefFocused.value && (filesSorted.value.length ?? 0) > 0) {
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
  <ul ref="listRef" role="list" class="flex flex-col text-sm">
    <li
      v-for="file in filesSorted"
      :key="file.id"
      :ref="(ref) => filesGrid.registerColumnRef(file.id, 'name', (ref as HTMLElement))"
      tabindex="-1"
      @keydown.up.exact.prevent="filesGrid.navigateUp(file.id, 'name')"
      @keydown.down.exact.prevent="filesGrid.navigateDown(file.id, 'name')"
      class="relative max-w-full border border-transparent px-3 py-0.5 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'text-orange-600': file.id == bench?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != bench?.focusedFileId,
      }"
      @click="focusFile(file)"
      @keydown.enter.exact.prevent="focusFileAndGoThere(file)"
    >
      <!-- Path -->
      <span
        class="decoration-none inline select-none truncate text-ellipsis rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
      >
        {{ file.name.length > 0 ? file.name : "(Unnamed)" }}
      </span>
      <!-- Extra info -->
      <span class="absolute right-2.5 top-0.5 flex flex-row-reverse gap-0.5">
        <!-- Issues -->
        <span v-if="module.issuesIn(file, { kind: IssueKind.Warning }).length > 0" class="text-yellow-700">
          {{ module.issuesIn(file, { kind: IssueKind.Warning }).length }}
        </span>
        <span v-if="module.issuesIn(file, { kind: IssueKind.Error }).length > 0" class="text-red-600">
          {{ module.issuesIn(file, { kind: IssueKind.Error }).length }}
        </span>
        <!-- Other clients -->
      </span>
    </li>
  </ul>
</template>
