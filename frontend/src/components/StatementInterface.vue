<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import CodeInterfaceMeta from "@/components/CodeInterfaceMeta.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import DatasetInterfaceMeta from "@/components/DatasetInterfaceMeta.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import SchemaInterface from "@/components/SchemaInterface.vue";
import SchemaInterfaceMeta from "@/components/SchemaInterfaceMeta.vue";
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

const props = defineProps<{
  file: FileHeader;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  isLastInRoot: boolean;
  lineNumberBase: number;
}>();
const statement = computed(() => useFragment(StatementContentType, props.statement));
const symbol = computed(() => useFragment(SymbolContentType, statement.value?.symbol));
const reference = computed(() => useFragment(SymbolContentType, statement.value?.reference));
const symbolOrReference = computed(() => symbol.value || reference.value);
const parameters = computed(() => symbolOrReference.value?.parameters ?? []);
const arguments_ = computed(() => symbol.value?.arguments ?? []);
const editorState = useEditorState();

function getArgument(name: string) {
  return arguments_.value?.find((a) => a.name === name);
}

const { getTimeFromNowString } = useTimeFromNow();

type SymbolInterface = {
  component: Component;
};
type MetaInterface = {
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
  [SymbolType.Schema]: {
    component: SchemaInterface,
  },
  [SymbolType.Task]: {
    component: TaskInterface,
  },
  // not yet defined symbol interfaces
  [SymbolType.Model]: undefined,
};
const metaInterfaces: Record<SymbolType, MetaInterface | undefined> = {
  [SymbolType.Dataset]: {
    component: DatasetInterfaceMeta,
  },
  [SymbolType.Code]: {
    component: CodeInterfaceMeta,
  },
  [SymbolType.Schema]: {
    component: SchemaInterfaceMeta,
  },
  // not yet defined symbol interfaces
  [SymbolType.Expectation]: undefined,
  [SymbolType.Task]: undefined,
  [SymbolType.Model]: undefined,
};

const isDefinition = computed(() => statement.value?.type == StatementType.Definition);
const isReference = computed(() => statement.value?.type == StatementType.Reference);
const isRunnable = computed(() => symbol.value?.type == SymbolType.Code || symbol.value?.type == SymbolType.Task);
const isFocused = computed(() => editorState.focusedElementId == statement.value?.id);
const isAncestorFocused = computed(() => isFocused.value || editorState.focusedElementId == statement.value.parent?.id);
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
  registered: computed(() => isRunnable.value && isFocused.value),
  apply: async () => {
    assert(symbol.value != null, "symbol is null");
    const runConfiguration = makeRunConfiguration(symbol.value);
    const runEditor = makeRunEditor(runConfiguration);
    editorState.openEditor(runEditor);
    editorState.focusEditor(runEditor);
  },
});

const metaActions: ComputedRef<MetaAction[]> = computed(() => {
  const metaActions = [];
  if (symbolOrReference.value != null && isRunnable.value) {
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
      assert(symbol.value != null, "symbol is null");
      await operations.symbol.rename(symbol.value.id, symbol.value.name, newName);
    } else if (statement.value.type == StatementType.Reference || statement.value.type == StatementType.Import) {
      assert(reference.value != null, "reference is null");
      await operations.symbol.rename(reference.value.id, reference.value.name, newName);
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
const depthOffsetX = computed(() => props.depth * 20);
</script>
<template>
  <div
    class="group relative border-x border-gray-200 transition-all"
    :class="{
      'border-gray-200 ': !isFocused,
      'border-l-orange-500': isAncestorFocused,
      'hover:border-l-orange-300': !isFocused,
      'rounded-t-sm border-t  border-gray-200': depth == 0, // group top
      'rounded-b-sm border-b border-gray-200 pb-3': isLastInRoot, // group bottom
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @mousedown="focus"
  >
    <!-- Imitate Monaco line numbers -->
    <span
      class="absolute top-[9px] w-6 select-none text-right font-mono text-sm"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-orange-200': !isFocused,
        'text-orange-400': isAncestorFocused,
        'font-bold text-orange-600': isFocused,
      }"
      >{{ lineNumberBase + 1 }}</span
    >
    <!-- Statement header & controls -->
    <div class="mx-3 flex flex-row items-center justify-between pt-1.5">
      <div class="flex flex-row items-baseline" v-if="symbolOrReference">
        <!--  declaration -->
        <span class="decoration-none inline-flex items-baseline text-sm text-black">
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
        <!-- Custom meta -->
        <component
          v-if="symbol != null && metaInterfaces[symbol.type] != null"
          :is="metaInterfaces[symbol.type]?.component"
          :symbol="symbol"
          :content="symbol.content"
          class="mr-1"
        />
        <!-- Statement meta info -->
        <span class="inline-flex flex-row items-baseline gap-2 px-1 text-xs">
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
    <!-- Symbol parameters & arguments -->
    <!-- Parameters (with argument if available) -->
    <div v-if="symbol" class="mx-3 flex flex-row gap-4 pb-1">
      <div class="flex flex-col" v-for="parameter in parameters" :key="parameter.name">
        <div class="-mb-0.5 flex flex-row items-baseline text-xs text-gray-700">
          <span>{{ parameter.name }}</span>
        </div>
        <div class="text-sm text-gray-900">
          <!-- Show argument if it's bound -->
          <template v-if="getArgument(parameter.name)">
            <span v-if="getArgument(parameter.name)?.value != null">
              {{ getArgument(parameter.name)?.value }}
            </span>
            <span class="text-black" v-else-if="getArgument(parameter.name)?.reference != null">
              {{ getArgument(parameter.name)?.reference?.name }}
            </span>
          </template>
          <!-- Otherwise show parameter type -->
          <span v-else class="text-gray-500">{{ parameter.type.toLowerCase() }}</span>
        </div>
      </div>
    </div>
    <!-- Symbol content (if statement defines a symbol) -->
    <div v-if="symbol != null" class="mx-3" :class="{ 'border-orange-600': isFocused }">
      <component
        v-if="interfaces[symbol.type] != undefined"
        :is="interfaces[symbol.type]?.component"
        :symbol="symbol"
        :content="symbol.content"
        :lineNumberBase="lineNumberBase"
        :xOffset="depthOffsetX"
        :generated="statement.generated"
        :commented="statement.commented"
        :focused="isFocused"
      />
      <span class="text-red-500" v-else> cannot render {{ symbol.type }} </span>
    </div>
  </div>
  <!-- Children -->
  <!-- <div v-if="statement.children?.length > 0">
      <StatementInterface
        v-for="(child, i) in statement.children"
        :key="child.id"
        :file="file"
        :statement="child"
        :depth="depth + 1"
        :lineNumberBase="lineNumberBase + i + 1"
      />
    </div> -->
</template>
