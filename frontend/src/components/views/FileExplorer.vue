<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { IssueKind } from "@/gql/graphql";
import { useBenchState, type FileHeader, type ViewId, type FileAction } from "@/state/bench";
import { useCurrentModule, type NodeBase, type InterpFile } from "@/state/module";
import { useOperations } from "@/state/operations";
import { ArrowsPointingOutIcon, DocumentDuplicateIcon } from "@heroicons/vue/24/outline";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const bench = useBenchState();
const module = useCurrentModule();
const ops = useOperations();

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
const contextMenuFile: Ref<InterpFile | null> = ref(null);
const contextMenuPosition: Ref<{ x: number; y: number } | null> = ref(null);
const contextMenuActions: Ref<FileAction[]> = computed(
  () =>
    (contextMenuFile.value == null
      ? []
      : [
          {
            label: "Open",
            icon: ArrowsPointingOutIcon,
            action: () => focusFileAndGoThere(contextMenuFile.value as FileHeader),
          },
          {
            label: "Delete",
            icon: DocumentDuplicateIcon,
            action: () => ops.file.softDelete(null, contextMenuFile.value?.id),
          },
        ]) as FileAction[]
);

function focusFile(file: FileHeader) {
  const focusedViewId = bench.focusedViewId;
  bench.focusFile(file as NodeBase);
  bench.focusView(focusedViewId as ViewId); // keep focused view
}

function focusFileAndGoThere(file: FileHeader) {
  bench.focusFile(file as NodeBase);
}

function closeContextMenu() {
  contextMenuFile.value = null;
  contextMenuPosition.value = null;
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
      class="relative max-w-full border border-transparent px-3 py-0.5 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'text-orange-600': file.id == bench?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != bench?.focusedFileId,
      }"
      @keydown.up.exact.prevent="filesGrid.navigateUp(file.id, 'name')"
      @keydown.down.exact.prevent="filesGrid.navigateDown(file.id, 'name')"
      @keydown.enter.exact.prevent="focusFileAndGoThere(file)"
      @click.left.prevent="focusFile(file)"
      @contextmenu.prevent="
        (e) => {
          contextMenuFile = file;
          contextMenuPosition = { x: e.clientX, y: e.clientY };
        }
      "
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
    <!-- File context menu -->
    <div v-if="contextMenuFile != null && contextMenuPosition != null">
      <!-- Invisible fixed overlay to prevent scrolling and capture clicks -->
      <div class="fixed left-0 top-0 z-40 h-full w-full overscroll-none" @click.stop="closeContextMenu" />
      <!-- Tab context menu popover (similar to action popover) -->
      <div
        class="fixed z-50 flex w-40 flex-col rounded-sm bg-white p-1 text-xs shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
      >
        <div
          v-for="(action, i) in contextMenuActions"
          :key="action.label"
          class="w-full"
          :class="[
            i > 0 && contextMenuActions[i - 1].groupId != action.groupId
              ? 'mt-0.5 border-t border-orange-900 border-opacity-[12%] pt-0.5'
              : '',
          ]"
          @click.prevent.stop="action.action(contextMenuFile), closeContextMenu()"
        >
          <button
            class="flex w-full flex-row items-center gap-2 rounded-sm px-1 py-1 hover:bg-orange-100 focus:outline-none"
            :class="[action.disabled || action.active ? 'cursor-not-allowed opacity-50' : '']"
          >
            <component :is="action.icon" class="h-4 w-4" />
            <span class="text-gray-700">{{ action.label }}</span>
          </button>
        </div>
      </div>
    </div>
  </ul>
</template>
