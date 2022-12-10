<script lang="ts" setup>
import { graphql } from "@/gql";
import { useEditorState, type FileHeader } from "@/utils/editor";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";
import { ref } from "vue";

defineProps<{ files: FileHeader[] }>();

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

const { mutate: renameFile } = useMutation(
  graphql(/* GraphQL */ `
    mutation renameFile($id: GlobalID!, $name: String!) {
      renameFile(input: { id: $id, name: $name }) {
        ... on File {
          id
          name
        }
      }
    }
  `)
);

const operations = useOperationsStore();

async function onNameEnter(event: Event) {
  const newName = (event.target as HTMLInputElement).innerText;
  if (newName.length > 0) {
    (event.target as HTMLElement)?.blur();
    const file = editor.focusedFile;
    if (!file) return;

    const oldName = file.name;
    await operations.perform({
      type: "rename-file",
      apply: async () => {
        await renameFile({ id: file.id, name: newName });
      },
      undo: async () => {
        await renameFile({ id: file.id, name: oldName });
      },
    });
  }
}
</script>
<template>
  <div ref="container">
    <!-- View header -->
    <div class="flex flex-row justify-between border-b border-gray-200 px-3 py-4">
      <span class="text-xs font-bold uppercase">Explorer</span>
      <!-- TODO @Feature: select explorer get_view (by type, by task tree) -->
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
            'bg-orange-100 font-bold text-orange-600': file.id == editor?.focusedFile?.id,
            'text-gray-700 hover:text-orange-600': file.id != editor?.focusedFile?.id,
            'border-orange-600': file.id == editor?.focusedElement?.id,
          }"
          @click="focus(file)"
        >
          <span
            :contenteditable="renaming && editor?.focusedElement?.id == file.id"
            maxlength="50"
            class="decoration-none inline rounded-sm bg-transparent text-sm text-inherit placeholder-gray-400 outline-none"
            :class="{
              'select-all': renaming && editor?.focusedElement?.id == file.id,
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
