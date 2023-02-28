<script lang="ts" setup>
import { provideGlobalAction } from "@/state/actions";
import { useEditorState, type FileHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { onClickOutside } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{
  files: FileHeader[];
}>();

const editor = useEditorState();

const filesSorted = computed(() => {
  const files = props.files.filter((f) => f.deletedAt == null && (editor.showGenerated || !f.generated));
  return files.sort((a, b) => {
    return a.path.localeCompare(b.path);
  });
});

function focus(file: FileHeader) {
  // focus file in editor
  editor?.focusFile(file);
  // and focus file as element if
  editor.focusElement(file);
}

const operations = useOperations();

// focused file actions
provideGlobalAction({
  id: "file.delete",
  label: "Delete file",
  shortcuts: ["backspace", "delete"],
  enabled: computed(() => !editor.editingElement && editor.focusedElementType == "File"),
  apply: async () => {
    if (editor.focusedElementId) {
      await operations.file.delete(editor.focusedElementId);
      if (editor.focusedFileId == editor.focusedElementId && editor.focusedEditor) {
        // close editor if focused file was deleted
        editor.closeEditor(editor.focusedEditor);
      }
    }
  },
});

// blur focused file if clicking outside file explorer
const listRef: Ref<HTMLDivElement | null> = ref(null);
onClickOutside(listRef, () => {
  if (editor.focusedElementType == "File") {
    editor.blurElement();
  }
});

defineExpose({
  count: computed(() => props.files.length),
});
</script>
<template>
  <!-- Panel: file explorer -->
  <ul ref="listRef" role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="file in filesSorted"
      :key="file.id"
      class="relative max-w-full px-3 py-0.5 hover:cursor-pointer"
      :class="{
        'bg-orange-100 text-orange-600': file.id == editor?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != editor?.focusedFileId,
        'bg-orange-600 text-white': file.id == editor?.focusedElementId,
        'border-l-2 border-l-orange-200 pl-2.5': file.generated,
      }"
      @click="focus(file)"
    >
      <!-- Icon? -->
      <!-- Path -->
      <span
        class="decoration-none inline truncate text-ellipsis rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
      >
        {{ file.name }}
      </span>
    </li>
  </ul>
</template>
