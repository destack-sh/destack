<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import CodeInterfaceMeta from "@/components/CodeInterfaceMeta.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import DatasetInterfaceMeta from "@/components/DatasetInterfaceMeta.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import ExpectationInterface from "@/components/ExpectationInterface.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import SchemaInterface from "@/components/SchemaInterface.vue";
import SchemaInterfaceMeta from "@/components/SchemaInterfaceMeta.vue";
import TaskInterface from "@/components/TaskInterface.vue";
import TaskInterfaceMeta from "@/components/TaskInterfaceMeta.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { provideAction, useActions } from "@/utils/actions";
import {
  makeRunConfiguration,
  makeRunEditor,
  MODIFIER_SHORTNAME,
  SYMBOL_TYPE_BY_SHORTNAME,
  SYMBOL_TYPE_SHORTNAME,
  useEditorState,
} from "@/utils/editor";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/utils/fragments";
import { useIntelliSense } from "@/utils/intellisense";
import { useOperations } from "@/utils/operations";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { useFocus, useFocusWithin } from "@vueuse/core";
import { assert } from "ts-essentials";
import { computed, ref, watch, type Component, type ComputedRef } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  lineNumberBase: number;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => statement.value.content);
const reference = computed(() => useFragment(StatementHeaderType, statement.value?.reference));
const parameters = computed(() => statement.value?.parameters ?? []);
const arguments_ = computed(() => statement.value?.arguments ?? []);

const symbolTypeShortname = computed(() =>
  statement.value.symbolType ? SYMBOL_TYPE_SHORTNAME[statement.value.symbolType] : null
);
const modifierShortname = computed(() =>
  statement.value.modifier ? MODIFIER_SHORTNAME[statement.value.modifier] : null
);
const depthOffsetX = computed(() => props.depth * 20);

type FocusableComponent = Component & {
  focus: () => void;
  defocus: () => void;
};
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
  [SymbolType.Task]: {
    component: TaskInterfaceMeta,
  },
  // not yet defined symbol interfaces
  [SymbolType.Expectation]: undefined,
  [SymbolType.Model]: undefined,
};

const editorState = useEditorState();
const actions = useActions();
const operations = useOperations();
const sense = useIntelliSense();

const isDefinition = computed(() => statement.value?.type == StatementType.Definition);
const isReference = computed(() => statement.value?.type == StatementType.Reference);
const isImport = computed(() => statement.value?.type == StatementType.Import);
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented);
const isRunnable = computed(
  () =>
    !isImport.value &&
    (statement.value?.symbolType == SymbolType.Code || statement.value?.symbolType == SymbolType.Task)
);
const isFocused = computed(() => editorState.focusedElementId == statement.value?.id);
const isAncestorFocused = computed(() => isFocused.value || editorState.focusedElementId == statement.value.parent?.id);
const isEditing = computed(() => isFocused.value && editorState.editingElement);
const readonly = computed(() => editorState.readonly || statement.value?.compiled);
const isAlias = computed(() => (isReference.value || isImport.value) && reference.value?.name != statement.value.name);
const importPath = computed(() => {
  assert(isImport.value, "statement is import");
  if (reference.value?.file.projectVersion.id != file.value.projectVersion.id) {
    // absolute import to dependency
    const dependency = sense.dependenciesById[reference.value?.file.projectVersion.id];
    if (!dependency) {
      return null;
    } else {
      return dependency.project.path + "." + reference.value?.file.pathWithoutExtension;
    }
  } else {
    // relative import
    return "." + reference.value?.file.pathWithoutExtension;
  }
});

type MetaAction = {
  icon: Component;
  label: string;
  action: () => void;
};

const run = provideAction({
  id: "statement.run",
  label: "Run",
  shortcuts: ["ctrl+enter"],
  registered: computed(() => isRunnable.value && isFocused.value),
  apply: async () => {
    assert(isRunnable.value, "statement is runnable");
    const runConfiguration = makeRunConfiguration(statement.value);
    const runEditor = makeRunEditor(runConfiguration);
    editorState.openEditor(runEditor);
    editorState.focusEditor(runEditor);
  },
});

const metaActions: ComputedRef<MetaAction[]> = computed(() => {
  const metaActions = [];
  if (isRunnable.value) {
    metaActions.push({
      icon: PlayIcon,
      label: run.value.label,
      action: run.value.apply,
    });
  }
  return metaActions;
});

async function onNameEnter(newName: string) {
  if (newName.length > 0 && newName != statement.value.name) {
    await operations.statement.rename(statement.value.id, statement.value.name ?? "", newName);
  }
}

// manage focus and navigation

const containerRef = ref<HTMLElement | null>(null);
const declarationRef = ref<HTMLElement | null>(null);
const contentRef = ref<Component | HTMLElement | null>(null);
const { focused: containerFocused } = useFocusWithin(containerRef);
const { focused: declarationFocused } = useFocus(declarationRef);
const { focused: contentFocused } = useFocus(contentRef);

function focus() {
  editorState.focusFile(file.value);
  editorState.focusElement(statement.value);
}

// this isn't great because it always scrolls and doesn't consider the container size
// auto scroll into focus once the element is focused if outside of viewport
watch(
  () => isFocused.value,
  (isFocused) => {
    if (isFocused && containerRef.value) {
      containerRef.value.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  }
);

// if anything inside the container is focused, enable editing mode
watch(
  () => containerFocused.value,
  (containerFocused) => {
    if (containerFocused) {
      focus();
      startEditing();
    } else {
      stopEditing();
    }
  }
);

// react to isEditing
watch(
  () => isEditing.value,
  (isEditing) => {
    // if focused and editing started without any inner focus, focus declaration
    if (isFocused.value && !containerFocused.value && isEditing && declarationRef.value) {
      declarationFocused.value = true;
    }
    // if focused and editing stopped, defocus
    if (!isEditing && declarationRef.value) {
      declarationFocused.value = false;
      (contentRef.value as FocusableComponent)?.defocus?.();
    }
  }
);

function startEditing() {
  editorState.editElement(statement.value);
}
function stopEditing() {
  editorState.stopEditingElement();
}

function navigateUp() {
  if (declarationFocused.value) {
    // declaration is focused, go to previous statement
    declarationFocused.value = false;
    actions.statement.moveFocusUp.value.apply();
  } else if (containerFocused.value) {
    // content is focused, go to declaration
    declarationFocused.value = true;
  }
}

function navigateDown() {
  if (declarationFocused.value && content.value) {
    // declaration is focused, go to content
    (contentRef.value as FocusableComponent).focus?.();
  } else if (containerFocused.value) {
    // content is focused already or not available, go to next statement
    actions.statement.moveFocusDown.value.apply();
  }
}

const blankRef = ref<HTMLElement | null>(null);
const blankContent = ref("");

watch(
  () => blankContent.value,
  async (input) => {
    console.log("input", input, typeof input);
    // check if input starts with # and a space
    if (input == "#") {
      await morphToComment();
    } else if (input == "import") {
      await morphToImport();
    } else if (input != "model" && SYMBOL_TYPE_BY_SHORTNAME[input] != null) {
      console.log("morph to ref/def of symbol", input);
    }
  }
);

async function morphToComment() {
  console.log("morph to comment");
  await operations.statement.morph(statement.value.id, { type: StatementType.Blank }, { type: StatementType.Comment });
  contentRef.value?.focus?.();
}

async function morphToImport() {
  console.log("morph to import");
  await operations.statement.morph(statement.value.id, { type: StatementType.Blank }, { type: StatementType.Import });
}

async function morphToBlank() {
  console.log("morph to blank");
  if (statement.value.type == StatementType.Comment) {
    await operations.statement.morph(
      statement.value.id,
      { type: StatementType.Comment },
      { type: StatementType.Blank }
    );
    blankContent.value = "";
    blankRef.value?.focus?.();
  }
}
</script>
<template>
  <div
    ref="containerRef"
    class="group relative border-x border-gray-200 tracking-tight transition-colors"
    :class="{
      'border-gray-200 ': !isFocused,
      'border-l-orange-500': isAncestorFocused,
      'hover:border-l-orange-300': !isFocused,
      'rounded-t-sm border-t border-gray-200': isFirstInGroup, // group top
      'pb-1': depth > 0, // inside group
      'rounded-b-sm border-b border-gray-200': isLastInGroup, // group bottom
      'pb-2.5': isLastInGroup && !isFirstInGroup, // group bottom with other top
      'pb-1.5': isLastInGroup && isFirstInGroup, // group top and bottom
      'font-mono': !isComment, // not sure if everything should be mono, but it's more consistent..
      italic: isCommented,
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @click="focus"
  >
    <!-- Debug info -->
    <span v-if="editorState.debug" class="absolute top-0 right-0 z-20 text-sm">
      i:{{ statement.index }} d:{{ depth }}
      <template v-if="isFirstInGroup">gs</template>
      <template v-if="isLastInGroup">ge</template>
      <template v-if="isCommented">c</template>
    </span>
    <!-- Commented overlay -->
    <div v-if="isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-50" />
    <!-- Monaco-like line numbers on the left margin -->
    <span
      class="absolute top-[7px] w-6 select-none text-right font-mono text-sm"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-orange-200': !isFocused,
        'text-orange-400': isAncestorFocused,
        'font-bold text-orange-600': isFocused,
      }"
      >{{ lineNumberBase + 1 }}</span
    >
    <!-- Statement focus indicator (left side if not editing) -->
    <div
      class="absolute top-0 left-0 h-full w-1"
      :class="isFocused && !isEditing ? 'bg-orange-100' : 'bg-transparent'"
    />
    <!-- Statement focus indicator (top and bottom if editing) -->
    <div class="absolute top-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <div class="absolute bottom-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <!-- Statement header & controls -->
    <div class="mx-3 flex flex-row items-center justify-between pt-1">
      <!--  Declaration -->
      <div class="flex flex-row items-baseline" v-if="statement.symbolType != null">
        <span class="decoration-none text-nowrap inline-flex items-baseline text-sm text-black">
          <span class="mr-1 text-orange-600" v-if="isImport">import</span>
          <span class="mr-1 text-orange-600" v-if="statement.modifier">{{ modifierShortname }}</span>
          <span class="mr-1 text-orange-600">{{ symbolTypeShortname }}</span>
          <!-- <span v-if="isAlias" class="text-nowrap flex-shrink-0 text-black">{{ reference?.name }}</span> -->
          <!-- <span v-if="isAlias" class="mx-1 text-orange-600">as</span> -->
          <EditableSpan
            ref="declarationRef"
            class="select-all rounded-sm p-0.5 text-sm text-inherit outline-none hover:bg-yellow-50 focus:bg-yellow-100"
            maxlength="100"
            :readonly="readonly"
            :modelValue="statement.name"
            @enter="onNameEnter"
            @navigateUp="navigateUp"
            @navigateDown="navigateDown"
            @escape="stopEditing"
            @click="startEditing"
          />
          <span v-if="isDefinition" class="-ml-0.5 font-bold text-orange-600">:</span>
          <span v-if="isImport" class="mx-1 text-orange-600">from</span>
          <span v-if="isImport">{{ importPath }}</span>
        </span>
      </div>
      <div
        class="relative my-1 flex w-full flex-row items-baseline text-sm"
        v-else-if="statement.type == StatementType.Blank"
      >
        <EditableSpan
          ref="blankRef"
          maxlength="100"
          class="w-full text-inherit outline-none hover:bg-yellow-50 focus:bg-yellow-100"
          :readonly="readonly"
          v-model="blankContent"
        />
        <span v-if="blankContent.length == 0" class="absolute text-gray-500 opacity-20 group-hover:opacity-100"
          >...</span
        >
      </div>
      <!-- Meta & controls (top right) -->
      <span
        class="inline-flex flex-row items-center font-sans"
        :class="{
          'opacity-0 group-hover:opacity-100': !isDefinition && !isFocused,
          'text-gray-400': !isFocused,
          'text-gray-500': isFocused,
        }"
      >
        <!-- Custom meta -->
        <component
          v-if="content != null && statement.symbolType != null && metaInterfaces[statement.symbolType] != null"
          :is="metaInterfaces[statement.symbolType]?.component"
          :file="file"
          :statement="statement"
          :content="content"
          class="mr-1"
        />
        <!-- Statement meta info -->
        <span class="inline-flex flex-row items-baseline gap-2 px-1 text-xs">
          <!-- <span> {{ getTimeFromNowString(statement.updatedAt) }} </span> -->
          <span v-if="statement.compiled">compiled</span>
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
    <div v-if="content != null && statement.symbolType != null" class="mx-3">
      <component
        ref="contentRef"
        v-if="interfaces[statement.symbolType] != undefined"
        :is="interfaces[statement.symbolType]?.component"
        :file="file"
        :statement="statement"
        :content="content"
        :lineNumberBase="lineNumberBase"
        :xOffset="depthOffsetX"
        :compiled="statement.compiled"
        :commented="statement.commented"
        :focused="isFocused"
        @navigateUp="navigateUp"
        @navigateDown="navigateDown"
        @escape="stopEditing"
      />
      <span class="text-red-500" v-else> cannot render {{ statement.symbolType }} </span>
    </div>
    <!-- Comment content -->
    <div v-else-if="isComment" class="mx-3 my-1 py-[0.5px]">
      <MonacoEditor
        ref="contentRef"
        :modelValue="statement.text || ''"
        language="markdown"
        @deleteIfEmpty="morphToBlank"
        @navigateUp="navigateUp"
        @navigateDown="navigateDown"
        @escape="stopEditing"
        hide-line-numbers
        :focused="isFocused"
        :readonly="readonly"
        :lineNumberOffset="0"
        class="italic opacity-60"
        :style="{ marginLeft: -24 + 'px' }"
      />
    </div>
  </div>
</template>
