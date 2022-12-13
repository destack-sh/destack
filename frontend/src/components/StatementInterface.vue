<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { provideAction } from "@/utils/actions";
import { makeRunConfiguration, makeRunEditor, useEditorState, type FileHeader } from "@/utils/editor";
import { StatementContentType, SymbolContentType } from "@/utils/fragments";
import { useOperations } from "@/utils/operations";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { assert } from "ts-essentials";
import { computed, type Component, type ComputedRef } from "vue";

const props = defineProps<{ file: FileHeader; statement: FragmentType<typeof StatementContentType> }>();
const statement = computed(() => useFragment(StatementContentType, props.statement));
const symbol = computed(() => useFragment(SymbolContentType, statement.value?.symbol));
const reference = computed(() => useFragment(SymbolContentType, statement.value?.reference));
const symbolOrReference = computed(() => symbol.value || reference.value);
const editorState = useEditorState();

const { getTimeFromNowString } = useTimeFromNow();

type SymbolInterface = {
  component: Component;
};

const interfaces: Record<SymbolType, SymbolInterface | undefined> = {
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
  // not yet defined symbol interfaces
  [SymbolType.Model]: undefined,
};

const isFocused = computed(() => editorState.focusedElementId == statement.value?.id);
function focus() {
  editorState.focusFile(props.file);
  editorState.focusElement(statement.value);
}

type MetaAction = {
  icon: Component;
  label: string;
  action: () => void;
};

const run = provideAction({
  id: "symbol.run",
  label: "Run",
  shortcuts: ["ctrl+enter"],
  registered: isFocused,
  enabled: computed(() => statement.value?.type == StatementType.Definition),
  apply: async () => {
    assert(symbol.value != null);
    const runConfiguration = makeRunConfiguration(symbol.value);
    const runEditor = makeRunEditor(runConfiguration);
    editorState.openEditor(runEditor);
    editorState.focusEditor(runEditor);
  },
});

const metaActions: ComputedRef<MetaAction[]> = computed(() => {
  const metaActions = [];
  if (symbolOrReference.value != null) {
    metaActions.push({
      icon: PlayIcon,
      label: run.value.label,
      action: run.value.apply,
    });
  }
  return metaActions;
});

const readonly = computed(() => editorState.readonly || statement.value?.generated);
const operations = useOperations();

async function onNameEnter(event: Event) {
  const newName = (event.target as HTMLInputElement).innerText;
  if (newName.length > 0) {
    (event.target as HTMLElement)?.blur();

    if (statement.value.type == StatementType.Definition) {
      assert(symbol.value != null);
      await operations.symbol.rename(statement.value.id, symbol.value.name, newName);
    } else if (statement.value.type == StatementType.Reference || statement.value.type == StatementType.Import) {
      assert(reference.value != null);
      await operations.symbol.rename(statement.value.id, reference.value.name, newName);
    }
  }
}
</script>
<template>
  <div class="relative" @mousedown="focus">
    <!-- Statement header & controls -->
    <div class="mx-1 my-1.5 flex flex-row items-center justify-between">
      <div class="flex flex-row items-baseline" v-if="symbolOrReference">
        <!--  declaration -->
        <span
          class="decoration-none px-0.5 text-sm tracking-wider"
          :class="{
            'text-black': !isFocused,
            'text-orange-600': isFocused,
          }"
        >
          <span>{{ statement.typeShortname }}</span>
          <span class="ml-1">{{ symbolOrReference.typeShortname }}</span>
          <span
            :contenteditable="!readonly"
            maxlength="100"
            class="ml-0.5 inline w-full select-all rounded-sm border border-transparent bg-transparent p-0.5 text-sm text-inherit placeholder-gray-400 outline-none hover:border-gray-300 focus:border-orange-500"
            @keydown.enter.prevent="onNameEnter"
          >
            {{ symbolOrReference.name }}
          </span>
        </span>
        <!-- Statement meta info -->
        <span class="inline-flex flex-row items-baseline gap-1 px-1 text-xs">
          <span class="text-gray-500"> {{ getTimeFromNowString(statement.updatedAt) }} </span>
          <span v-if="statement.generated" class="text-gray-500">generated</span>
        </span>
      </div>
      <!-- Symbol meta controls -->
      <span class="inline-flex flex-row gap-1">
        <button
          v-for="action in metaActions"
          :key="action.label"
          class="rounded-sm p-0.5 hover:bg-gray-100 hover:text-gray-700"
          :class="isFocused ? 'text-gray-500' : 'text-gray-400'"
          @click.prevent="action.action"
        >
          <component :is="action.icon" class="h-4 w-4" />
        </button>
      </span>
    </div>

    <!-- Symbol content (if statement defines a symbol) -->
    <div
      v-if="symbol != null"
      class="rounded-sm border bg-white px-2 py-2"
      :class="{ 'border-orange-600 shadow-orange-300': isFocused }"
    >
      <component
        v-if="interfaces[symbol.type] != undefined"
        :is="interfaces[symbol.type]?.component"
        :symbol="symbol"
        :content="symbol.content"
        :generated="statement.generated"
        :focused="isFocused"
      />
      <span class="text-red-500" v-else> cannot render {{ symbol.type }} </span>
    </div>
    <!-- Children -->
    <div v-if="statement.children?.length > 0" class="ml-4">
      <StatementInterface v-for="child in statement.children" :key="child.id" :statement="child" :file="file" />
    </div>
  </div>
</template>
