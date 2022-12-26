<script lang="ts" setup>
import { StatementType } from "@/gql/graphql";
import { useEditorState, type FileHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { PlusIcon } from "@heroicons/vue/20/solid";

const props = defineProps<{ file: FileHeader; index: number }>();
const editor = useEditorState();
const operations = useOperations();

async function addStatement(file: FileHeader, index: number) {
  const newStatement = await operations.statement.create(file.id, null, index, StatementType.Blank);
  editor.editElement(newStatement);
}
</script>
<template>
  <button
    class="group relative w-full cursor-pointer py-1.5 opacity-0 transition-opacity hover:opacity-100"
    @click="addStatement(props.file, props.index)"
  >
    <div class="absolute inset-0 flex items-center" aria-hidden="true">
      <div class="w-full border-t border-gray-200" />
    </div>
    <div class="relative flex justify-center">
      <span class="bg-gray-50 px-2">
        <PlusIcon class="h-4 w-4 text-gray-400 group-hover:text-gray-500" aria-hidden="true" />
      </span>
    </div>
  </button>
</template>
