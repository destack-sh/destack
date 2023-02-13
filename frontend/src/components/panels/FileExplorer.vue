<script lang="ts" setup>
import { provideAction, provideGlobalAction } from "@/state/actions";
import { useEditorState, type FileHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { onClickOutside } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  files: FileHeader[];
}>();

const filesSorted = computed(() => {
  const files = [...props.files];
  return files.sort((a, b) => {
    return a.path.localeCompare(b.path);
  });
});

const editor = useEditorState();

const renaming = ref(false);
const container = ref(null);

provideAction({
  id: "file.renameCurrent",
  label: "Rename file",
  shortcuts: ["f2", "shift+f6"],
  enabled: computed(() => !renaming.value),
  apply: async () => {
    renaming.value = true;
  },
});
onClickOutside(container, () => {
  renaming.value = false;
});

function focus(file: FileHeader) {
  // focus file in editor
  editor?.focusFile(file);
  // and focus file as element if
  editor.focusElement(file);
}

const operations = useOperations();

async function onNameEnter(event: Event) {
  const newName = (event.target as HTMLInputElement).innerText;
  if (newName.length > 0) {
    (event.target as HTMLElement)?.blur();
    renaming.value = false;

    const fileId = editor.focusedFileId;
    const file = props.files.find((f) => f.id == fileId);
    if (!file) {
      console.warn("focused file not found in explorer" + fileId);
      return;
    }

    await operations.file.rename(file.id, file.name, newName);
  }
}

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

defineExpose({
  count: computed(() => props.files.length),
});
</script>
<template>
  <!-- Panel: file explorer -->
  <ul role="list" class="flex flex-col gap-1 py-1 text-sm">
    <li
      v-for="file in filesSorted"
      :key="file.id"
      class="relative max-w-full border border-transparent px-3 hover:cursor-pointer"
      :class="{
        'bg-orange-100 text-orange-600': file.id == editor?.focusedFileId,
        'text-gray-700 hover:text-orange-600': file.id != editor?.focusedFileId,
        'border-orange-600': file.id == editor?.focusedElementId,
      }"
      @click="focus(file)"
    >
      <span
        :contenteditable="renaming && editor?.focusedElementId == file.id"
        maxlength="50"
        class="decoration-none inline truncate text-ellipsis rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
        :class="{
          'select-all': renaming && editor?.focusedElementId == file.id,
        }"
        @keydown.enter.exact.prevent="onNameEnter"
      >
        {{ file.name }}
      </span>
      <span>.x</span>
    </li>
  </ul>
</template>
