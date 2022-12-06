<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SymbolType, type SymbolDefinition } from "@/gql/graphql";
import {
  makeRunConfiguration,
  makeRunEditor,
  useEditorState,
  type RunConfiguration,
  type SymbolDefinitionHeader,
} from "@/utils/editor";
import { FileHeaderType } from "@/utils/fragments";
import { ArrowPathIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Component } from "vue";

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
          typeShortname
          nameDotType
          typeNameDeclaration
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
  component: Component;
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

const editorState = useEditorState();
function isDefinitionFocused(definition: Pick<SymbolDefinition, "id">) {
  return editorState.focusedDefinition?.id == definition.id;
}

type MetaAction = {
  icon: Component;
  label: string;
  action: (definition: SymbolDefinition) => void;
};

function getSymbolMetaActions(definition: SymbolDefinitionHeader & Pick<SymbolDefinition, "generated">): MetaAction[] {
  const actions: MetaAction[] = [];
  if (definition.type == SymbolType.Task) {
    actions.push({
      icon: WrenchIcon,
      label: "Compile",
      action: () => ({}),
    });
    actions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => ({}),
    });
  } else if (definition.type == SymbolType.Code) {
    if (definition.generated) {
      actions.push({
        icon: ArrowPathIcon,
        label: "Re-compile",
        action: () => ({}),
      });
    }
    actions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => {
        const runConfiguration = makeRunConfiguration(fileHeader, definition);
        const runEditor = makeRunEditor(runConfiguration);
        editorState.openEditor(runEditor);
        editorState.focusEditor(runEditor);
      },
    });
  }
  return actions;
}
</script>

<template>
  <div class="mx-8 my-5 flex flex-col gap-6">
    <div
      v-for="definition in definitions"
      :key="definition.id"
      class="relative mx-auto w-full max-w-[1000px] transition-all"
      @mousedown="editorState?.focusDefinition(fileHeader, definition)"
    >
      <!-- Symbol definition header & controls -->
      <div class="mx-1 my-1.5 flex flex-row items-center justify-between">
        <div class="flex flex-row items-baseline">
          <!-- Symbol declaration -->
          <span
            class="text-sm tracking-wide"
            :class="{
              'text-black': !isDefinitionFocused(definition),
              'text-orange-600': isDefinitionFocused(definition),
            }"
          >
            <span>{{ definition.typeShortname }}</span> <span class="">{{ definition.name }}</span>
          </span>
          <!-- Symbol meta info -->
          <span class="inline-flex flex-row items-baseline gap-1 px-2 text-xs">
            <span class="text-gray-500"> {{ getTimeFromNowString(definition.updatedAt) }} </span>
            <span v-if="definition.generated" class="text-gray-500">generated</span>
          </span>
        </div>
        <!-- Symbol meta controls -->
        <span class="-mb-1 inline-flex flex-row gap-1">
          <button
            v-for="action in getSymbolMetaActions(definition)"
            :key="action.label"
            class="rounded-sm p-0.5 hover:bg-gray-100 hover:text-gray-700"
            :class="isDefinitionFocused(definition) ? 'text-gray-500' : 'text-gray-400'"
            @click.prevent="action.action"
          >
            <component :is="action.icon" class="h-4 w-4" />
          </button>
        </span>
      </div>

      <!-- Symbol content -->
      <div
        class="rounded-sm border bg-white py-2 px-2"
        :class="{ 'border-orange-600 shadow-orange-300': isDefinitionFocused(definition) }"
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
