<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import { useEditorState, type FileHeader, type ViewId } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { onClickOutside, useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{
  files?: FileHeader[];
}>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const editor = useEditorState();

const filesSorted = computed(() => {
  const files = props.files?.filter((f) => f.deletedAt == null && (editor.showGenerated || !f.generated)) ?? [];
  return files.sort((a, b) => {
    return a.path.localeCompare(b.path);
  });
});
const focusedFileId = computed(() => filesSorted.value.find((f) => f.id == editor.focusedFileId)?.id);
const filesGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  filesSorted,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusFile(file: FileHeader) {
  const focusedViewId = editor.focusedViewId;
  editor.focusFile(file);
  editor.focusView(focusedViewId as ViewId); // keep focused view
}

function focusFileAndGoThere(file: FileHeader) {
  editor.focusFile(file);
}

const operations = useOperations();

// blur focused file if clicking outside file explorer
const listRef: Ref<HTMLDivElement | null> = ref(null);
const { focused: listRefFocused } = useFocusWithin(listRef);
onClickOutside(listRef, () => {
  if (editor.focusedElementType == "File") {
    editor.blurElement();
  }
});

function focus() {
  // focus currently focused file if nothing was directly selected (and thus focused)
  if (focusedFileId.value && !listRefFocused.value) {
    nextTick(() => filesGrid.focus(focusedFileId.value, "name"));
  } else if (!listRefFocused.value && (props.files?.length ?? 0) > 0) {
    nextTick(() => filesGrid.focus(0, "name"));
  }
}

function blur() {
  filesGrid.blur();
}

defineExpose({
  count: computed(() => props.files?.length),
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
        'bg-orange-100 text-orange-600': file.id == editor?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != editor?.focusedFileId,
        'border-l-2 border-l-orange-200 pl-2.5': file.generated,
      }"
      @click="focusFile(file)"
      @keydown.enter.exact.prevent="focusFileAndGoThere(file)"
    >
      <!-- Path -->
      <span
        class="decoration-none inline select-none truncate text-ellipsis rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
      >
        {{ file.name }}
      </span>
    </li>
  </ul>
</template>
