<script lang="ts" setup>
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import InstructionInterface from "@/components/InstructionInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SymbolType, type SymbolDefinition } from "@/gql/graphql";
import { EDITOR_STATE_KEY, type EditorState, type SymbolDefinitionHeader } from "@/utils/editor";
import { useQuery } from "@vue/apollo-composable";
import { computed, inject } from "vue";

const FileHeader = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    createdAt
    updatedAt
  }
`);

const props = defineProps<{ file: FragmentType<typeof FileHeader> }>();
const fileHeader = useFragment(FileHeader, props.file);

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query getFileById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        ...FileHeader
        definitions {
          id
          name
          type
          nameDotType
          createdAt
          updatedAt
          content {
            ...InstructionContent
            ...DatasetContent
            ...ExpectationContent
            ...TaskContent
          }
        }
      }
    }
  `),
  () => ({
    fileId: fileHeader.id,
  })
);

const definitions = computed(() => {
  return file.value?.file?.definitions || [];
});

type DefinitionInterface = {
  component: any;
};

const interfaces: Record<SymbolType, DefinitionInterface> = {
  [SymbolType.Dataset]: {
    component: DatasetInterface,
  },
  [SymbolType.Instruction]: {
    component: InstructionInterface,
  },
  [SymbolType.Expectation]: {
    component: ExpectationInterface,
  },
  [SymbolType.Task]: {
    component: TaskInterface,
  },
};

const editorState = inject<EditorState>(EDITOR_STATE_KEY);
const isFocused = computed(() => editorState?.focusedFile.value?.id == fileHeader.id);

function isDefinitionFocused(definition: Pick<SymbolDefinition, "id">) {
  return editorState?.focusedDefinition.value?.id == definition.id;
}

function focusDefinition(definition: SymbolDefinitionHeader) {
  // TODO @Cleanup: avoid direct editor state mutation
  if (editorState) {
    editorState.focusedDefinition.value = definition;
  }
}
</script>

<template>
  <div class="m-6 flex flex-col gap-6">
    <div
      v-for="definition in definitions"
      :key="definition.id"
      class="mx-auto w-full max-w-[1000px] rounded-sm border bg-white py-2 px-2"
      :class="{ 'border-orange-600 shadow-md shadow-orange-300': isDefinitionFocused(definition) }"
      @mousedown="focusDefinition(definition)"
    >
      <span class="m-1 text-sm text-gray-900">
        {{ definition.nameDotType }}
      </span>
      <div class="relative min-h-fit overflow-clip">
        <component
          v-if="interfaces[definition.type]"
          :is="interfaces[definition.type].component"
          :definition="definition"
          :content="definition.content"
        />
        <span class="text-red-500" v-else> cannot render {{ definition.type }} </span>
      </div>
    </div>
  </div>
</template>
