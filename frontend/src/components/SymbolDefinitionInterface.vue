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
  type FileHeader,
  type SymbolDefinitionHeader,
} from "@/utils/editor";
import { SymbolDefinitionContentType } from "@/utils/fragments";
import { ArrowPathIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { useMutation } from "@vue/apollo-composable";
import { computed, type Component } from "vue";

const props = defineProps<{ file: FileHeader; definition: FragmentType<typeof SymbolDefinitionContentType> }>();
const definition = computed(() => useFragment(SymbolDefinitionContentType, props.definition));
const editorState = useEditorState();

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
      action: () => console.error("compile not implemented yet"),
    });
    actions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => console.error("run not implemented yet"),
    });
  } else if (definition.type == SymbolType.Code) {
    if (definition.generated) {
      actions.push({
        icon: ArrowPathIcon,
        label: "Re-compile",
        action: () => console.error("re-compile not implemented yet"),
      });
    }
    actions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => {
        const runConfiguration = makeRunConfiguration(props.file, definition);
        const runEditor = makeRunEditor(runConfiguration);
        editorState.openEditor(runEditor);
        editorState.focusEditor(runEditor);
      },
    });
  } else if (definition.type == SymbolType.Dataset) {
    if (definition.generated) {
      actions.push({
        icon: ArrowPathIcon,
        label: "Re-compile",
        action: () => console.error("re-compile not implemented yet"),
      });
    }
  }
  return actions;
}

const readonly = computed(() => editorState.readonly || definition.value.generated);

const { mutate: updateName, loading: running } = useMutation(
  graphql(/* GraphQL */ `
    mutation updateSymbolDefinitionName($id: GlobalID!, $name: String!) {
      updateSymbolDefinition(input: { id: $id, name: $name }) {
        __typename
      }
    }
  `)
);

async function onNameEnter(event: Event) {
  const newName = (event.target as HTMLInputElement).innerText;
  if (newName.length > 0) {
    console.log("update name to ", newName);
    event.target?.blur();
    await updateName({ id: definition.value.id, name: newName });
  }
}
</script>
<template>
  <div class="relative transition-all" @mousedown="editorState?.focusDefinition(file, definition)">
    <!-- Symbol definition header & controls -->
    <div class="mx-1 my-1.5 flex flex-row items-center justify-between">
      <div class="flex flex-row items-baseline">
        <!-- Symbol declaration -->
        <span
          class="px-0.5 text-sm tracking-wide"
          :class="{
            'text-black': !isDefinitionFocused(definition),
            'text-orange-600': isDefinitionFocused(definition),
          }"
        >
          <span>{{ definition.typeShortname }}</span>
          <span
            :contenteditable="!readonly"
            maxlength="100"
            class="decoration-none ml-0.5 inline w-full select-all rounded-sm border border-transparent bg-transparent p-0.5 text-sm text-inherit placeholder-gray-400 outline-none selection:bg-yellow-200 hover:border-gray-300 focus:border-orange-500"
            @keydown.enter.prevent="onNameEnter"
          >
            {{ definition.name }}
          </span>
        </span>
        <!-- Symbol meta info -->
        <span class="inline-flex flex-row items-baseline gap-1 px-1 text-xs">
          <span class="text-gray-500"> {{ getTimeFromNowString(definition.updatedAt) }} </span>
          <span v-if="definition.generated" class="text-gray-500">generated</span>
        </span>
      </div>
      <!-- Symbol meta controls -->
      <span class="inline-flex flex-row gap-1">
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
</template>
