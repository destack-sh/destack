<script lang="ts" setup>
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import CodeInterface from "@/components/CodeInterface.vue";
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
    path
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
            ...CodeContent
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
  [SymbolType.Code]: {
    component: CodeInterface,
  },
  [SymbolType.Expectation]: {
    component: ExpectationInterface,
  },
  [SymbolType.Task]: {
    component: TaskInterface,
  },
};

const editorState = inject<EditorState>(EDITOR_STATE_KEY);
function isDefinitionFocused(definition: Pick<SymbolDefinition, "id">) {
  return editorState?.focusedDefinition.value?.id == definition.id;
}
</script>

<template>
  <div class="m-6 flex flex-col gap-6">
    <div
      v-for="definition in definitions"
      :key="definition.id"
      class="mx-auto w-full max-w-[1000px] rounded-sm border bg-white py-2 px-2 transition-all"
      :class="{ 'border-orange-600 shadow-md shadow-orange-300': isDefinitionFocused(definition) }"
      @mousedown="editorState?.focusDefinition(definition)"
    >
      <span
        class="m-1 text-sm"
        :class="{
          'text-orange-900': !isDefinitionFocused(definition),
          'text-orange-600': isDefinitionFocused(definition),
        }"
      >
        {{ definition.nameDotType }}
      </span>
      <div class="relative min-h-fit overflow-clip">
        <component
          v-if="interfaces[definition.type]"
          :is="interfaces[definition.type].component"
          :definition="definition"
          :content="definition.content"
          :focused="isDefinitionFocused(definition)"
        />
        <span class="text-red-500" v-else> cannot render {{ definition.type }} </span>
      </div>
    </div>
  </div>
</template>
