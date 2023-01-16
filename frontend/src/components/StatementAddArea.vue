<script lang="ts" setup>
import { StatementType } from "@/gql/graphql";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { PlusIcon } from "@heroicons/vue/20/solid";

const props = defineProps<{ file: FileHeader; index: number }>();
const editor = useEditorState();
const operations = useOperations();

async function addStatement(file: FileHeader, index: number) {
  const newStatement = await operations.statement.create(file.id, null, index, StatementType.Blank);
  editor.editElement(newStatement as StatementHeader);
}
</script>
<template>
  <button
    class="group relative flex w-full cursor-default py-1 opacity-0 transition-opacity hover:opacity-100"
    @click="addStatement(props.file, props.index)"
  >
    <div class="justify-left relative flex align-top">
      <span class="bg-white px-2">
        <PlusIcon class="h-4 w-4 text-gray-300" aria-hidden="true" />
      </span>
    </div>
  </button>
</template>
