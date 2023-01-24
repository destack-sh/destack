<script lang="ts" setup>
import CodeInterface from "@/components/CodeInterface.vue";
import CodeInterfaceMeta from "@/components/CodeInterfaceMeta.vue";
import DatasetInterface from "@/components/DatasetInterface.vue";
import DatasetInterfaceMeta from "@/components/DatasetInterfaceMeta.vue";
import DescriptionInterface from "@/components/DescriptionInterface.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import TypeInterface from "@/components/TypeInterface.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementModifier, StatementType, SymbolType, type InterpStatement } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import {
  MODIFIER_BY_KEYWORD,
  MODIFIER_KEYWORD,
  SYMBOL_TYPE_BY_KEYWORD,
  SYMBOL_TYPE_KEYWORD,
  useEditorState,
  type StatementHeader,
} from "@/state/editor";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { relativePath, InterpStatementContentType, localErrorsOf, useCurrentModuleRuntime } from "@/state/runtime";
import { Combobox, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus, useFocusWithin, useMagicKeys, useTextSelection, whenever } from "@vueuse/core";
import { computed, ref, toRef, watch, watchEffect, type Component, type ComputedRef, type Ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  reference: FragmentType<typeof StatementHeaderType> | null;
  depth: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  lineNumberBase: number;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const reference = computed(() => useFragment(StatementHeaderType, props.reference));

const symbolTypeKeyword = computed(() =>
  statement.value.symbolType ? SYMBOL_TYPE_KEYWORD[statement.value.symbolType] : null
);
const modifierKeyword = computed(() => (statement.value.modifier ? MODIFIER_KEYWORD[statement.value.modifier] : null));
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
  [SymbolType.Type]: {
    component: TypeInterface,
  },
  [SymbolType.Capability]: {
    component: DescriptionInterface,
  },
  [SymbolType.Task]: {
    component: DescriptionInterface,
  },
  [SymbolType.Expectation]: {
    component: DescriptionInterface,
  },
  [SymbolType.Dataset]: {
    component: DatasetInterface,
  },
  [SymbolType.Code]: {
    component: CodeInterface,
  },
  [SymbolType.Model]: undefined,
  [SymbolType.Requirement]: undefined,
  [SymbolType.Runconfig]: undefined,
  [SymbolType.Compilation]: undefined,
  [SymbolType.Value]: undefined,
};
const metaInterfaces: Record<SymbolType, MetaInterface | undefined> = {
  [SymbolType.Dataset]: {
    component: DatasetInterfaceMeta,
  },
  [SymbolType.Code]: {
    component: CodeInterfaceMeta,
  },
  // not yet defined symbol interfaces
  [SymbolType.Type]: undefined,
  [SymbolType.Capability]: undefined,
  [SymbolType.Task]: undefined,
  [SymbolType.Value]: undefined,
  [SymbolType.Expectation]: undefined,
  [SymbolType.Model]: undefined,
  [SymbolType.Requirement]: undefined,
  [SymbolType.Runconfig]: undefined,
  [SymbolType.Compilation]: undefined,
};

const editorState = useEditorState();
const actions = useActions();
const operations = useOperations();

const isFocused = computed(() => editorState.focusedElementId == statement.value?.id);
const isEditing = computed(() => isFocused.value && editorState.editingElement);
const readonly = computed(() => editorState.readonly || statement.value.compiled);

const isRedefinition = computed(() => statement.value?.type == StatementType.Redefinition);
const isDefinition = computed(() => statement.value?.type == StatementType.Definition || isRedefinition.value);
const isReference = computed(() => statement.value?.type == StatementType.Reference || isRedefinition.value);
const isParameter = computed(() => isReference.value && statement.value.modifier == StatementModifier.With);
const isArgument = computed(() => isDefinition.value && statement.value.modifier == StatementModifier.With);
const isImport = computed(() => statement.value?.type == StatementType.Import);
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented);
const isRunnable = computed(
  () =>
    !isImport.value &&
    (statement.value?.symbolType == SymbolType.Code || statement.value?.symbolType == SymbolType.Task)
);
const isAlias = computed(
  () => isImport.value && reference.value != null && reference.value?.name != statement.value.name
);
const importPath = computed(() => statement.value.importPath);

type InlineAction = {
  icon: Component;
  label: string;
  action: () => void;
};

const inlineActions: ComputedRef<InlineAction[]> = computed(() => {
  const actions = [];
  if (isRunnable.value) {
    actions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => ({}),
    });
  }
  return actions;
});

// manage focus, declaration and navigation

const containerRef = ref<HTMLElement | null>(null);
const declarationRef = ref<HTMLElement | null>(null);
const aliasRef = ref<HTMLElement | null>(null);
const contentRef = ref<Component | InstanceType<typeof MonacoEditor> | null>(null);
const { focused: containerFocused } = useFocusWithin(containerRef);
const { focused: declarationFocused } = useFocus(declarationRef);
const { focused: aliasFocused } = useFocus(aliasRef);
const { focused: contentFocused } = useFocusWithin(contentRef as any);

function focus() {
  editorState.focusFile(file.value);
  editorState.focusElement(statement.value as StatementHeader);
}

function onClickContainer() {
  // focus on first click, edit on second click
  if (selectingReference.value) {
    // ignore click while selecting reference
  } else if (!isFocused.value) {
    focus();
    startEditing(); // immediately start editing
  }
}

// auto scroll into focus once the element is focused if outside of viewport
// (this isn't great because it always scrolls and doesn't consider the container size)
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
    if (containerFocused && !isEditing.value) {
      focus();
      startEditing();
    }
  }
);
onClickOutside(containerRef, () => {
  if (isEditing.value) {
    console.log("defocus because clicked outside");
    cancelCurrentEditing();
  }
});

// react to isEditing
watch(
  () => [isEditing.value, declarationRef.value, containerFocused.value],
  ([isEditing, declarationRef, containerFocused]) => {
    // if focused and editing started without any inner focus
    if (isFocused.value && !containerFocused && isEditing && !selectingReference.value) {
      if (declarationRef) {
        // focus declaration if exists
        console.log("auto-focus declaration");
        declarationFocused.value = true;
      } else {
        // otherwise focus content (e.g. for comments)
        console.log("auto-focus content");
        (contentRef.value as FocusableComponent)?.focus?.();
      }
    }
    // if focused and editing stopped, defocus
    if (!isEditing) {
      declarationFocused.value = false;
      aliasFocused.value = false;
      (contentRef.value as FocusableComponent)?.defocus?.();
    }
  }
);

function startEditing() {
  editorState.editElement(statement.value as StatementHeader);
}
function cancelCurrentEditing() {
  if (selectingReference.value) {
    selectingReference.value = false;
  } else {
    editorState.stopEditingElement(statement.value as StatementHeader);
  }
}

function navigateUp() {
  const hasDeclaration = statement.value.type != StatementType.Comment;
  if (declarationFocused.value || (!hasDeclaration && containerFocused.value)) {
    // declaration is focused, go to previous statement
    declarationFocused.value = false;
    actions.apply("statement.moveFocusUp");
  } else if (containerFocused.value) {
    // content is focused, go to declaration
    declarationFocused.value = true;
  }
}

function navigateDown() {
  if (selectingReference.value) {
    // ignore
  } else if (declarationFocused.value && contentRef.value) {
    // declaration is focused, go to content
    (contentRef.value as FocusableComponent).focus?.();
  } else if (containerFocused.value) {
    // content is focused already or not available, go to next statement
    actions.apply("statement.moveFocusDown");
  }
}

// manage declaration and alias

const declarationContent = ref("");
const declarationComboboxRef = ref<InstanceType<typeof Combobox> | null>(null);
const declarationComboboxOptionsRef = ref<HTMLElement | null>(null);
const aliasContent = ref("");
const canCreateRef = computed(
  () =>
    statement.value.type != StatementType.Blank &&
    !isArgument.value &&
    (isImport.value || statement.value.parent?.id != null)
);
const selectingReference = ref(false);
const declarationSelection = useTextSelection();

// open selecting reference on ctrl+space
const keys = useMagicKeys();
whenever(keys["ctrl+space"], () => {
  if (isFocused.value && canCreateRef.value) {
    selectingReference.value = true;
    // select first option
    // TODO @Feature: select option in reference selection (open combo box)
    declarationComboboxOptionsRef.value?.focus?.(); // (this doesn't work right now?)
  }
});

// apply selected reference and close selection
function onSelectReference(referenceId: string | ":define") {
  if (referenceId == ":define") {
    morphToDefinition();
  } else {
    const reference = filteredSymbols.value.find((r) => r.id == referenceId);
    if (reference == null) throw new Error("reference not found: " + referenceId);
    morphSetReference(reference);
  }
  selectingReference.value = false;
}

// stop selecting if editing is cancelled
watchEffect(() => {
  if (!isEditing.value) {
    selectingReference.value = false;
  }
});

// possible reference targets & filters
const runtime = useCurrentModuleRuntime();
const availableSymbols: Ref<InterpStatement[]> = computed(() => {
  if (statement.value.type == StatementType.Reference || statement.value.type == StatementType.Definition) {
    // for references & definitions all definitions & imports within the file are available
    return runtime.module.value?.files
      .find((f) => f.globalId == file.value.id)
      ?.statements.map((s) => useFragment(InterpStatementContentType, s))
      .filter((s) => s.type == StatementType.Definition || s.type == StatementType.Import);
  } else if (statement.value.type == StatementType.Import) {
    // for imports all definitions outside this file are available (incl. deps)
    const dependenciesDefinitions = runtime.dependenciesIndex.value?.flatMap((d) =>
      Object.values(d.statementsByGlobalId).filter((s) => s.type == StatementType.Definition)
    );
    const otherFileDefinitions = (runtime.module.value?.files ?? [])
      .filter((f) => f.globalId != file.value.id)
      .flatMap((f) =>
        f.statements
          .map((s) => useFragment(InterpStatementContentType, s))
          .filter((s) => s.type == StatementType.Definition)
      );
    return [...dependenciesDefinitions, ...otherFileDefinitions];
  } else {
    return [];
  }
});
const filterText = ref("");
// filter up to cursor position in declaration (if cursor is in declaration)
// store in a variable to retain last cursor position when out of focus (e.g. when selecting reference)
watchEffect(() => {
  if (declarationFocused.value) {
    let cursorPosition = declarationFocused.value ? declarationSelection.selection.value?.focusOffset ?? 0 : 0;
    filterText.value = declarationContent.value.slice(0, cursorPosition).toLowerCase();
  }
});

const filteredSymbols = computed(() => {
  if (!selectingReference.value) return []; // only compute when selecting reference
  let symbols = availableSymbols.value;
  if (filterText.value) {
    symbols = symbols.filter((symbol) => symbol.name?.toLowerCase().includes(filterText.value));
  }
  if (statement.value.symbolType != null) {
    symbols = symbols.filter((symbol) => symbol.symbolType == statement.value.symbolType);
  }
  return symbols;
});

// react to declaration content input
watch(
  () => declarationContent.value,
  async (input) => {
    if (statement.value.type == StatementType.Blank) {
      if (input == "#") {
        await morphToComment();
      }
      if (input == "import") {
        await morphToImport();
      }
      if (MODIFIER_BY_KEYWORD[input] != null) {
        await setModifier(MODIFIER_BY_KEYWORD[input]);
        declarationContent.value = "";
      }
    }
    if (statement.value.symbolType == null && SYMBOL_TYPE_BY_KEYWORD[input] != null) {
      // if it's an import, keep it an import
      if (statement.value.type == StatementType.Import) {
        await morphTo(StatementType.Import, SYMBOL_TYPE_BY_KEYWORD[input]);
      } else {
        await morphTo(StatementType.Reference, SYMBOL_TYPE_BY_KEYWORD[input]);
      }
    }
  }
);
// set declaration and alias content on statement change
watchEffect(() => {
  if (isEditing.value && containerFocused.value) {
    return;
  }
  // reset/init declaration content
  if (statement.value.type == StatementType.Definition || isParameter.value) {
    declarationContent.value = statement.value.name ?? "";
  } else if (statement.value.type == StatementType.Import || statement.value.type == StatementType.Reference) {
    if (reference.value != null) {
      if (statement.value.name != reference.value.name) {
        // we have an alias
        declarationContent.value = reference.value.name ?? "";
        aliasContent.value = statement.value.name ?? "";
      } else {
        // no alias
        declarationContent.value = statement.value.name ?? "";
        aliasContent.value = "";
      }
    } else {
      // no reference
      declarationContent.value = statement.value.name ?? "";
      aliasContent.value = "";
    }
  } else if (statement.value.symbolType == SymbolType.Requirement) {
    declarationContent.value = ""; // should be requirement path
  }
});

async function onDeclarationKeydown(event: KeyboardEvent) {
  // catch special chars
  // create definition if ':' is pressed
  if (event.key == ":") {
    event.preventDefault();
    if (statement.value.type != StatementType.Definition && declarationContent.value != "") {
      await morphToDefinition();
    }
  }
}

async function onDeclarationEnter() {
  if (declarationContent.value.length == 0) {
    // create blank statement and navigate down
    actions.apply("statement.insertBelowCurrent");
  } else if (statement.value.type == StatementType.Definition || isParameter.value) {
    // rename and focus content
    await operations.statement.rename(statement.value.id, statement.value.name ?? "", declarationContent.value);
    (contentRef.value as FocusableComponent)?.focus?.();
  }
}

async function resetAlias() {
  if (reference.value != null) {
    await operations.statement.rename(statement.value.id, statement.value.name ?? "", reference.value.name ?? "");
    declarationContent.value = reference.value.name ?? "";
  }
}

function deleteLeftOnMain() {
  console.log("delete left on main");
  // remove statement prefix (import, type, modifier)
  if (statement.value.type == StatementType.Import) {
    if (statement.value.symbolType != null) {
      morphTo(statement.value.type, undefined);
    } else {
      morphToBlank();
    }
  } else if (statement.value.symbolType != null) {
    morphToBlank();
  } else if (statement.value.modifier != null) {
    setModifier(null);
  } else if (statement.value.type == StatementType.Blank) {
    deleteSelf();
  }
  // TODO @Robustness: delete and insert new blank statement instead of morphing to blank
  //  If we have content, delete & swap is the easiest way to get proper undo/redo.
}

function deleteSelf() {
  // delete self, move focus up
  actions.apply("statement.moveFocusUp");
  operations.statement.delete(statement.value.id);
}

function deleteRightOnMain() {
  console.log("delete right on main");
  // remove define/redefine/arguments
  if (statement.value.type == StatementType.Definition) {
    morphTo(StatementType.Reference);
  }
}

function deleteLeftOnAlias() {
  // reset alias to reference name
  resetAlias();
}

async function setModifier(modifier: StatementModifier | null) {
  console.log("set modifier", modifier);
  await operations.statement.modify(statement.value.id, statement.value.modifier ?? null, modifier);
}

async function morphToComment() {
  await operations.statement.morph(
    statement.value.id,
    { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
    { type: StatementType.Comment }
  );
  declarationContent.value = "";
  (contentRef.value as FocusableComponent)?.focus?.();
}

async function morphToImport() {
  if (statement.value.modifier != null) {
    await setModifier(null);
  }
  await operations.statement.morph(statement.value.id, { type: statement.value.type }, { type: StatementType.Import });
  declarationContent.value = "";
}

async function morphToDefinition() {
  if (statement.value.type == StatementType.Definition) {
    return;
  }
  // apply name and morph to definition
  if (statement.value.name != declarationContent.value) {
    await operations.statement.rename(statement.value.id, statement.value.name ?? null, declarationContent.value);
  }
  await morphTo(StatementType.Definition, statement.value.symbolType ?? undefined);
  declarationContent.value = statement.value.name ?? "";
  // focus content
  (contentRef.value as FocusableComponent)?.focus?.();
}

async function morphSetReference(newReference: { id: string; name: string; symbolType: SymbolType }) {
  let targetType = statement.value.type == StatementType.Definition ? StatementType.Reference : statement.value.type;
  // TODO @Robustness: should morphs be atomic (rename + setReference + morph type)
  if (statement.value.type != targetType || newReference.symbolType != statement.value.symbolType) {
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: targetType, symbolType: newReference.symbolType ?? undefined }
    );
  }
  if (newReference.name != statement.value.name) {
    await operations.statement.rename(statement.value.id, statement.value.name ?? null, newReference.name ?? null);
    declarationContent.value = newReference.name ?? "";
  }
  if (newReference.id != reference.value?.id) {
    await operations.statement.setReference(statement.value.id, reference.value?.id, newReference.id);
  }
}

async function morphTo(type: StatementType, symbolType?: SymbolType) {
  await operations.statement.morph(
    statement.value.id,
    { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
    { type, symbolType }
  );
  declarationContent.value = "";
  (contentRef.value as FocusableComponent)?.focus?.();
}

async function morphToBlank() {
  if (statement.value.type != StatementType.Definition) {
    await operations.statement.morph(
      statement.value.id,
      { type: StatementType.Comment },
      { type: StatementType.Blank }
    );
    declarationContent.value = "";
    declarationFocused.value = true;
  } else {
    throw new Error("cannot morph definition to blank");
  }
}

// errors
const localErrors = localErrorsOf(toRef(props, "statement"));
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);
</script>
<template>
  <div
    ref="containerRef"
    class="group relative border-x-0 border-gray-200 transition-colors"
    :class="{
      // 'border-gray-200 ': !isFocused,
      // 'border-l-orange-500': isFamilyFocused,
      'hover:border-l-orange-300': !isFocused,
      'pb-0.5': true,
      'font-mono': !isComment, // not sure if everything should be mono, but it's more consistent..
      italic: isCommented,
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @click="onClickContainer"
  >
    <!-- Debug info -->
    <span
      v-if="editorState.debug"
      class="absolute top-2 -right-1 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm lowercase"
    >
      <template v-if="isFocused">f({{ declarationFocused ? "d" : "" }}{{ contentFocused ? "c" : "" }}) </template>
      <template v-if="isEditing">e</template>
      <template v-if="isFirstInGroup">[</template>
      <template v-if="isLastInGroup">]</template>
      <template v-if="isCommented">#</template>
      {{ statement.modifier }}
      {{ statement.type }}
      <template v-if="statement.symbolType">{{ statement.symbolType }}:</template>
      <template v-if="statement.name != null">{{ statement.name }}</template>
      r:{{ statement.revision }} i:{{ statement.index }} d:{{ depth }}
    </span>
    <!-- Commented overlay -->
    <div v-if="isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-50" />
    <!-- Monaco-like line numbers on the left margin -->
    <span
      class="absolute top-[6px] w-6 select-none text-right font-mono text-sm not-italic"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-orange-200': !isFocused && !(isComment || isCommented),
        'text-gray-200': !isFocused && (isComment || isCommented),
        'font-bold text-orange-600': isFocused && !(isComment || isCommented),
        'font-bold text-gray-400': isFocused && (isComment || isCommented),
      }"
      >{{ lineNumberBase + 1 }}</span
    >
    <!-- Gutter indicators on the right margin -->
    <span
      class="absolute right-0 top-[6px] select-none text-left text-sm font-bold not-italic"
      :style="{ right: -12 + 'px' }"
    >
      <span class="text-red-700" v-if="hasLocalErrors">
        {{ localErrors?.length }}
      </span>
    </span>

    <!-- Statement focus indicator (left side if not editing) -->
    <div
      class="absolute top-0 left-0 h-full w-1"
      :class="isFocused && !isEditing ? 'bg-orange-100' : 'bg-transparent'"
    />
    <!-- Statement focus indicator (top and bottom if editing) -->
    <div class="absolute top-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <div class="absolute bottom-0 left-0 h-0.5 w-full" :class="isEditing ? 'bg-orange-100' : 'bg-transparent'" />
    <!-- Blank statement dots -->
    <div
      v-if="statement.type == StatementType.Blank && declarationContent == ''"
      class="absolute top-0 mx-3 h-full w-full text-gray-300 group-hover:opacity-100"
      :class="{ 'opacity-100': isFocused, 'opacity-0': !isFocused }"
    >
      ...
    </div>
    <!-- Statement header & controls -->
    <div class="mx-3 flex flex-row items-center justify-between pt-1">
      <!--  Declaration -->
      <!-- TODO @Cleanup: factor out statement declaration component (the mess is above) -->
      <div
        v-if="!isComment"
        class="decoration-none text-no-wrap relative flex flex-row items-baseline justify-start py-0.5 text-sm text-black"
      >
        <!-- Decorations for statement (on declaration) -->
        <!-- Squiggly error line -->
        <span v-if="hasLocalErrors" class="absolute -bottom-1 left-0 h-2 w-full text-red-700">
          <svg viewBox="0 0 100 1" preserveAspectRatio="none">
            <path d="M0 0h100v100h-100z" fill="currentColor" />
          </svg>
        </span>
        <!-- Statement prefixxes (types & modifiers) -->
        <span class="mr-1 text-orange-600" v-if="isImport">import</span>
        <span class="mr-1 text-orange-600" v-if="statement.modifier">{{ modifierKeyword }}</span>
        <span class="mr-1 text-orange-600" v-if="statement.symbolType">{{ symbolTypeKeyword }}</span>
        <!-- Editable statement main part -->
        <div class="relative inline-flex">
          <EditableSpan
            ref="declarationRef"
            maxlength="100"
            class="text-nowrap whitespace-nowrap !border-none p-0 text-sm !shadow-none !outline-none !ring-0"
            :readonly="readonly"
            v-model="declarationContent"
            @deleteLeft="deleteLeftOnMain"
            @deleteRight="deleteRightOnMain"
            @navigateUp="navigateUp"
            @navigateDown="navigateDown"
            @escape="cancelCurrentEditing"
            @click="startEditing"
            @enter="onDeclarationEnter"
            @keydown="onDeclarationKeydown"
          />
          <!-- Doubles as a combobox to look up symbols -->
          <Combobox
            ref="declarationComboboxRef"
            :modelValue="reference?.id || ':define'"
            @update:modelValue="onSelectReference"
            nullable
            v-if="(canCreateRef || reference != null) && isEditing"
          >
            <ComboboxOptions
              ref="declarationComboboxOptionsRef"
              static
              as="ul"
              class="absolute top-4 z-10 mt-1 max-h-60 w-96 overflow-auto border border-orange-400 bg-white text-sm shadow-md"
              v-show="selectingReference"
            >
              <!-- References to select -->
              <ComboboxOption
                v-for="symbol in filteredSymbols"
                as="template"
                :key="symbol.id"
                :value="symbol.id"
                v-slot="{ active, selected }"
              >
                <li
                  class="decoration-none group/li relative flex flex-row justify-between p-1 hover:cursor-pointer"
                  :class="{ 'bg-orange-100': selected }"
                >
                  <span class="group-hover/li:text-orange-600" :class="{ 'text-orange-600': active }">
                    <template v-if="statement.symbolType == null && symbol.symbolType != null">
                      <!-- specify type of reference if we haven't narrowed down yet -->
                      {{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}
                    </template>
                    {{ symbol.name }}
                  </span>
                  <span class="truncate text-gray-500">
                    {{ relativePath(statement, symbol) }}
                  </span>
                </li>
              </ComboboxOption>
              <!-- Nothing found -->
              <ComboboxOption key=":none" value=":none" as="template" v-if="filteredSymbols.length == 0" disabled>
                <li class="decoration-none group/li relative flex flex-row justify-between p-1 hover:cursor-pointer">
                  <span class="group-hover/li:text-orange-600 text-gray-500">{{ declarationContent }} not found</span>
                </li>
              </ComboboxOption>
              <!-- Define locally (if not an import) -->
              <ComboboxOption
                key=":define"
                value=":define"
                as="template"
                v-if="statement.type != StatementType.Import && declarationContent.length > 0"
                v-slot="{ active, selected }"
              >
                <li
                  class="decoration-none group/li relative flex flex-row justify-between p-1 hover:cursor-pointer"
                  :class="{ 'bg-orange-100': selected }"
                >
                  <span class="group-hover/li:text-orange-600" :class="{ 'text-orange-600': active }">
                    {{ declarationContent }}:
                  </span>
                  <span class="text-gray-500"> (define) </span>
                </li>
              </ComboboxOption>
            </ComboboxOptions>
          </Combobox>
        </div>
        <!-- Statement postfixes (alias & import location) -->
        <span v-if="isDefinition" class="-ml-0.5 font-bold text-orange-600">:</span>
        <span v-if="isAlias" class="mx-1 text-orange-600">as</span>
        <!-- Editable alias -->
        <EditableSpan
          v-if="isAlias"
          ref="aliasRef"
          maxlength="100"
          class="text-inherit outline-none"
          :readonly="readonly"
          v-model="aliasContent"
          @deleteLeft="deleteLeftOnAlias"
          @escape="cancelCurrentEditing"
          @click="startEditing"
        />
        <!-- Import postfix (not editable since derived from selected main) -->
        <span v-if="isImport" class="mx-1 text-orange-600">from</span>
        <span v-if="isImport && importPath != null">{{ importPath }}</span>
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
          v-if="statement.symbolType != null && metaInterfaces[statement.symbolType] != null"
          :is="metaInterfaces[statement.symbolType]?.component"
          :file="file"
          :statement="statement"
          class="mr-1"
        />
        <!-- Symbol meta controls -->
        <span class="inline-flex flex-row gap-1">
          <button
            v-for="action in inlineActions"
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
    <div v-if="statement.symbolType != null && statement.type == StatementType.Definition" class="mx-3">
      <component
        ref="contentRef"
        v-if="interfaces[statement.symbolType] != undefined"
        :is="interfaces[statement.symbolType]?.component"
        :file="file"
        :statement="statement"
        :lineNumberBase="lineNumberBase"
        :xOffset="depthOffsetX"
        :focused="isFocused"
        :editing="isEditing"
        :readonly="readonly"
        @navigateUp="navigateUp"
        @navigateDown="navigateDown"
        @escape="cancelCurrentEditing"
      />
    </div>
    <!-- Comment content -->
    <!-- TODO @Cleanup: comment content should probably be just another component -->
    <!-- TODO @Cleanup: use proper comment styling instead of opacity -->
    <div v-else-if="isComment" class="mx-3 my-1 py-[0.5px]">
      <MonacoEditor
        ref="contentRef"
        :modelValue="statement.text || ''"
        language="markdown"
        @deleteIfEmpty="morphToBlank"
        @navigateUp="navigateUp"
        @navigateDown="navigateDown"
        @escape="cancelCurrentEditing"
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
