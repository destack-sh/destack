<script lang="ts" setup>
import { EDITOR_STATE_KEY, type EditorState, type FileHeader } from "@/utils/editor";
import { inject } from "vue";

defineProps<{ files: FileHeader[] }>();

const state = inject<EditorState>(EDITOR_STATE_KEY);
</script>
<template>
  <div>
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
          class="relative py-0.5 px-3 hover:cursor-pointer"
          :class="
            file.id == state?.focusedFile.value?.id
              ? 'bg-orange-100 font-bold text-orange-700'
              : 'text-gray-700 hover:text-orange-700'
          "
          @click="state?.focusFile(file)"
        >
          <!-- There may be other types later, but currently it's all instruct -->
          {{ file.name }}<span class="font-normal">.instruct</span>
        </li>
      </ul>
    </div>
  </div>
</template>
