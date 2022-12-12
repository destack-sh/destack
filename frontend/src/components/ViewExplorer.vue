<script lang="ts" setup>
import { useActions } from "@/utils/actions";
import { useEditorState, type FileHeader } from "@/utils/editor";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";
import { ref } from "vue";

const props = defineProps<{ files: FileHeader[] }>();

const editor = useEditorState();

const renaming = ref(false);
const container = ref(null);
// rename file with f2
const keys = useMagicKeys();
whenever(keys["f2"], () => {
  renaming.value = true;
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

const actions = useActions();

async function onNameEnter(event: Event) {
  const newName = (event.target as HTMLInputElement).innerText;
  if (newName.length > 0) {
    (event.target as HTMLElement)?.blur();
    const fileId = editor.focusedFileId;
    const file = props.files.find((f) => f.id == fileId);
    if (!file) {
      console.warn("focused file not found in explorer" + fileId);
      return;
    }

    await actions.file.rename(file.id, file.name, newName);
  }
}
</script>
<template>
  <div ref="container">
    <!-- View header -->
    <div class="flex flex-row justify-between border-b border-gray-200 px-3 py-4">
      <span class="text-xs font-bold uppercase">Explorer</span>
    </div>
    <!-- View contents -->
    <div class="flex flex-1 flex-col">
      <!-- View: explorer -->
      <ul role="list" class="flex flex-col gap-1 text-sm">
        <li
          v-for="file in files"
          :key="file.id"
          class="relative border border-transparent px-3 hover:cursor-pointer"
          :class="{
            'bg-orange-100 font-bold text-orange-600': file.id == editor?.focusedFileId,
            'text-gray-700 hover:text-orange-600': file.id != editor?.focusedFileId,
            'border-orange-600': file.id == editor?.focusedElementId,
          }"
          @click="focus(file)"
        >
          <span
            :contenteditable="renaming && editor?.focusedElementId == file.id"
            maxlength="50"
            class="decoration-none inline rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
            :class="{
              'select-all': renaming && editor?.focusedElementId == file.id,
            }"
            @keydown.enter.prevent="onNameEnter"
          >
            {{ file.path }}</span
          >
          <span class="font-normal">.instruct</span>
        </li>
      </ul>
    </div>
  </div>
</template>
