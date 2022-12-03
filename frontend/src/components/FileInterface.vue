<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SymbolType, type SymbolDefinition } from "@/gql/graphql";
import { EDITOR_STATE_KEY, type EditorState } from "@/utils/editor";
import { FileHeaderType } from "@/utils/fragments";
import { useQuery } from "@vue/apollo-composable";
import { computed, inject } from "vue";

const props = defineProps<{ file: FragmentType<typeof FileHeaderType> }>();
const fileHeader = useFragment(FileHeaderType, props.file);

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
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
          generated
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

const { getTimeFromNowString } = useTimeFromNow();

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
      class="relative mx-auto w-full max-w-[1000px] transition-all"
      @mousedown="editorState?.focusDefinition(definition)"
    >
      <div>
        <span
          class="m-1 text-sm"
          :class="{
            'text-gray-900': !isDefinitionFocused(definition),
            'text-orange-600': isDefinitionFocused(definition),
          }"
        >
          {{ definition.nameDotType }}
        </span>
        <span v-if="definition.generated">
          <span class="text-xs text-gray-500">(generated)</span>
        </span>
        <span class="text-xs text-gray-500"> edited {{ getTimeFromNowString(definition.updatedAt) }} </span>
      </div>

      <div
        class="relative overflow-clip rounded-sm border bg-white py-2 px-2"
        :class="{ 'border-orange-600 shadow-md shadow-orange-300': isDefinitionFocused(definition) }"
      >
        <!-- TODO @Cleanup: access symbol props via definition only (like generated) -->
        <component
          v-if="interfaces[definition.type]"
          :is="interfaces[definition.type].component"
          :definition="definition"
          :content="definition.content"
          :generated="definition.generated"
          :focused="isDefinitionFocused(definition)"
        />
        <span class="text-red-500" v-else> cannot render {{ definition.type }} </span>
      </div>
    </div>
  </div>
</template>
