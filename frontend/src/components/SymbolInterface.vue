<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { SymbolType, type Symbol } from "@/gql/graphql";
import {
  makeRunConfiguration,
  makeRunEditor,
  useEditorState,
  type FileHeader,
  type SymbolHeader,
} from "@/utils/editor";
import { SymbolContentType } from "@/utils/fragments";
import { useOperationsStore } from "@/utils/operations";
import { ArrowPathIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { useMutation } from "@vue/apollo-composable";
import { computed, type Component } from "vue";

const props = defineProps<{ file: FileHeader; symbol: FragmentType<typeof SymbolContentType> }>();
const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
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

function isDefinitionFocused(symbol: Pick<Symbol, "id">) {
  return editorState.focusedElement?.id == symbol.id;
}

type MetaAction = {
  icon: Component;
  label: string;
  action: (symbol: Symbol) => void;
};

function getSymbolMetaActions(symbol: SymbolHeader & Pick<Symbol, "generated">): MetaAction[] {
  const actions: MetaAction[] = [];
  if (symbol.type == SymbolType.Task) {
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
  } else if (symbol.type == SymbolType.Code) {
    if (symbol.generated) {
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
        const runConfiguration = makeRunConfiguration(props.file, symbol);
        const runEditor = makeRunEditor(runConfiguration);
        editorState.openEditor(runEditor);
        editorState.focusEditor(runEditor);
      },
    });
  } else if (symbol.type == SymbolType.Dataset) {
    if (symbol.generated) {
      actions.push({
        icon: ArrowPathIcon,
        label: "Re-compile",
        action: () => console.error("re-compile not implemented yet"),
      });
    }
  }
  return actions;
}

const readonly = computed(() => editorState.readonly || symbol.value.generated);

const { mutate: renameSymbol } = useMutation(
  graphql(/* GraphQL */ `
    mutation renameSymbol($id: GlobalID!, $name: String!) {
      renameSymbol(input: { id: $id, name: $name }) {
        ... on Symbol {
          id
          name
          typeNameDeclaration
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

    const oldName = symbol.value.name;
    await operations.perform({
      type: "rename-symbol",
      apply: async () => {
        await renameSymbol({ id: symbol.value.id, name: newName });
      },
      undo: async () => {
        await renameSymbol({ id: symbol.value.id, name: oldName });
      },
    });
  }
}
</script>
<template>
  <div class="relative transition-all" @mousedown="editorState?.focusDefinition(file, symbol)">
    <!-- Symbol symbol header & controls -->
    <div class="mx-1 my-1.5 flex flex-row items-center justify-between">
      <div class="flex flex-row items-baseline">
        <!-- Symbol declaration -->
        <span
          class="px-0.5 text-sm tracking-wide"
          :class="{
            'text-black': !isDefinitionFocused(symbol),
            'text-orange-600': isDefinitionFocused(symbol),
          }"
        >
          <span>{{ symbol.typeShortname }}</span>
          <span
            :contenteditable="!readonly"
            maxlength="100"
            class="decoration-none ml-0.5 inline w-full select-all rounded-sm border border-transparent bg-transparent p-0.5 text-sm text-inherit placeholder-gray-400 outline-none hover:border-gray-300 focus:border-orange-500"
            @keydown.enter.prevent="onNameEnter"
          >
            {{ symbol.name }}
          </span>
        </span>
        <!-- Symbol meta info -->
        <span class="inline-flex flex-row items-baseline gap-1 px-1 text-xs">
          <span class="text-gray-500"> {{ getTimeFromNowString(symbol.updatedAt) }} </span>
          <span v-if="symbol.generated" class="text-gray-500">generated</span>
        </span>
      </div>
      <!-- Symbol meta controls -->
      <span class="inline-flex flex-row gap-1">
        <button
          v-for="action in getSymbolMetaActions(symbol)"
          :key="action.label"
          class="rounded-sm p-0.5 hover:bg-gray-100 hover:text-gray-700"
          :class="isDefinitionFocused(symbol) ? 'text-gray-500' : 'text-gray-400'"
          @click.prevent="action.action"
        >
          <component :is="action.icon" class="h-4 w-4" />
        </button>
      </span>
    </div>

    <!-- Symbol content -->
    <div
      class="rounded-sm border bg-white px-2 py-2"
      :class="{ 'border-orange-600 shadow-orange-300': isDefinitionFocused(symbol) }"
    >
      <!-- TODO @Cleanup: access symbol props via symbol only (like generated) -->
      <component
        v-if="interfaces[symbol.type]"
        :is="interfaces[symbol.type].component"
        :symbol="symbol"
        :content="symbol.content"
        :generated="symbol.generated"
        :focused="isDefinitionFocused(symbol)"
      />
      <span class="text-red-500" v-else> cannot render {{ symbol.type }} </span>
    </div>
  </div>
</template>
