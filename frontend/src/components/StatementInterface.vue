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
import { LinkIcon, PlayIcon } from "@heroicons/vue/24/outline";
import { assert } from "ts-essentials";
import { computed, type Component, type ComputedRef } from "vue";

const props = defineProps<{ file: FileHeader; statement: FragmentType<typeof StatementContentType>; depth: number }>();
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

const isDefinition = computed(() => statement.value?.type == StatementType.Definition);
const isReference = computed(() => statement.value?.type == StatementType.Reference);
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

const modifierShortname = computed(() => {
  // map modifier to lower case
  if (statement.value?.modifier) {
    return statement.value.modifier.toLowerCase();
  }
  throw new Error("statement has no modifier");
});
</script>
<template>
  <div class="relative" :class="depth == 0 ? 'rounded-sm border-b border-gray-200 bg-white' : ''">
    <!-- Self -->
    <div
      class="group relative border-x border-t border-b transition-all"
      :class="{ 'border-y-orange-400': isFocused, 'border-gray-200 border-b-white': !isFocused }"
      :style="{ paddingLeft: props.depth + 'rem' }"
      @mousedown="focus"
    >
      <!-- Statement header & controls -->
      <div class="mx-3 my-1 flex flex-row items-center justify-between">
        <div class="flex flex-row items-baseline" v-if="symbolOrReference">
          <!--  declaration -->
          <span class="decoration-none inline-flex items-baseline text-sm tracking-wider text-black">
            <span class="mr-1 text-orange-600" v-if="statement.modifier">{{ modifierShortname }}</span>
            <span class="text-orange-600">{{ symbolOrReference.typeShortname }}</span>
            <span
              :contenteditable="!readonly"
              maxlength="100"
              class="ml-0.5 inline w-full select-all rounded-sm border border-transparent bg-transparent py-0.5 text-sm text-inherit placeholder-gray-400 outline-none hover:border-gray-300 focus:border-orange-500"
              @keydown.enter.prevent="onNameEnter"
            >
              {{ symbolOrReference.name }}
            </span>
            <LinkIcon v-if="isReference" class="inline h-3 w-3 text-gray-400" />
            <span v-if="isDefinition" class="-ml-0.5 text-orange-600">:</span>
          </span>
        </div>
        <span
          class="inline-flex flex-row items-center"
          :class="{
            'opacity-0 group-hover:opacity-100': !isDefinition && !isFocused,
            'text-gray-400': !isFocused,
            'text-gray-500': isFocused,
          }"
        >
          <!-- Statement meta info -->
          <span class="inline-flex flex-row items-baseline gap-1 px-1 text-xs">
            <span> {{ getTimeFromNowString(statement.updatedAt) }} </span>
            <span v-if="statement.generated">generated</span>
          </span>
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
        </span>
      </div>
      <!-- Symbol content (if statement defines a symbol) -->
      <div v-if="symbol != null" class="mx-3 my-1.5" :class="{ 'border-orange-600': isFocused }">
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
    </div>
    <!-- Children -->
    <div v-if="statement.children?.length > 0">
      <StatementInterface
        v-for="child in statement.children"
        :key="child.id"
        :file="file"
        :statement="child"
        :depth="depth + 1"
      />
    </div>
  </div>
</template>
