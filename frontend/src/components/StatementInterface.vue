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
import { StatementModifier, StatementType, SymbolType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import {
  MODIFIER_BY_SHORTNAME,
  MODIFIER_SHORTNAME,
  SYMBOL_TYPE_BY_SHORTNAME,
  SYMBOL_TYPE_SHORTNAME,
  useEditorState,
  type StatementHeader,
} from "@/state/editor";
import { FileHeaderType, StatementContentType, StatementHeaderType } from "@/state/fragments";
import { useStatementMetadata } from "@/state/intellisense";
import { useOperations } from "@/state/operations";
import { Combobox, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus, useFocusWithin, useMagicKeys, useTextSelection, whenever } from "@vueuse/core";
import { computed, ref, toRef, watch, watchEffect, type Component, type ComputedRef, type Ref } from "vue";

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
const reference = computed(() => useFragment(StatementHeaderType, statement.value?.reference));

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

const meta = useStatementMetadata(toRef(props, "file"), toRef(props, "statement"));

const isFocused = computed(() => editorState.focusedElementId == statement.value?.id);
const isFamilyFocused = computed(() => isFocused.value || false); // nocheckin incomplete
const isEditing = computed(() => isFocused.value && editorState.editingElement);
const readonly = computed(() => editorState.readonly || statement.value.compiled);

type MetaAction = {
  icon: Component;
  label: string;
  action: () => void;
};

const metaActions: ComputedRef<MetaAction[]> = computed(() => {
  const metaActions = [];
  if (meta.isRunnable) {
    metaActions.push({
      icon: PlayIcon,
      label: "Run",
      action: () => ({}),
    });
  }
  return metaActions;
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
  if (declarationFocused.value) {
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
    !meta.isArgument &&
    (meta.isImport || statement.value.parent?.id != null)
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
    declarationComboboxOptionsRef.value?.focus?.(); // (this doesn')
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
const availableSymbols: Ref<StatementHeader[]> = computed(() => {
  if (statement.value.type == StatementType.Reference || statement.value.type == StatementType.Definition) {
    return [];
  } else if (statement.value.type == StatementType.Import) {
    return [];
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
function getImportSourcePath(statement: StatementHeader): string | undefined {
  return "???"; // nocheckin
}

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
      if (MODIFIER_BY_SHORTNAME[input] != null) {
        await setModifier(MODIFIER_BY_SHORTNAME[input]);
        declarationContent.value = "";
      }
    }
    if (statement.value.symbolType == null && SYMBOL_TYPE_BY_SHORTNAME[input] != null) {
      // if it's an import, keep it an import
      if (statement.value.type == StatementType.Import) {
        await morphTo(StatementType.Import, SYMBOL_TYPE_BY_SHORTNAME[input]);
      } else {
        await morphTo(StatementType.Reference, SYMBOL_TYPE_BY_SHORTNAME[input]);
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
  if (statement.value.type == StatementType.Definition || meta.isParameter) {
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
    declarationContent.value = meta.requirementPath ?? "";
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
  } else if (statement.value.type == StatementType.Definition || meta.isParameter) {
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
    // delete self, move focus up
    actions.apply("statement.moveFocusUp");
    operations.statement.delete(statement.value.id);
  }
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

async function morphSetReference(to: StatementHeader) {
  let targetType = statement.value.type == StatementType.Definition ? StatementType.Reference : statement.value.type;
  // TODO @Robustness: should morphs be atomic (rename + setReference + morph type)
  if (statement.value.type != targetType || to.symbolType != statement.value.symbolType) {
    await operations.statement.morph(
      statement.value.id,
      { type: statement.value.type, symbolType: statement.value.symbolType ?? undefined },
      { type: targetType, symbolType: to.symbolType ?? undefined }
    );
  }
  if (to.name != statement.value.name) {
    await operations.statement.rename(statement.value.id, statement.value.name ?? null, to.name ?? null);
    declarationContent.value = to.name ?? "";
  }
  if (to.id != reference.value?.id) {
    await operations.statement.setReference(statement.value.id, reference.value?.id, to.id);
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
</script>
<template>
  <div
    ref="containerRef"
    class="group relative border-x-0 border-gray-200 transition-colors"
    :class="{
      // 'border-gray-200 ': !isFocused,
      'border-l-orange-500': isFamilyFocused,
      'hover:border-l-orange-300': !isFocused,
      // 'rounded-t-sm border-t border-gray-200': isFirstInGroup, // group top
      'pb-1': depth > 0, // inside group
      // 'rounded-b-sm border-b border-gray-200': isLastInGroup, // group bottom
      'pb-2.5': isLastInGroup && !isFirstInGroup, // group bottom with other top
      'pb-1.5': isLastInGroup && isFirstInGroup, // group top and bottom
      'font-mono': !meta.isComment, // not sure if everything should be mono, but it's more consistent..
      italic: meta.isCommented,
    }"
    :style="{ paddingLeft: depthOffsetX + 'px' }"
    @click="onClickContainer"
  >
    <!-- Debug info -->
    <span
      v-if="editorState.debug"
      class="absolute -top-1 -right-1 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm lowercase"
    >
      <template v-if="isFocused">f({{ declarationFocused ? "d" : "" }}{{ contentFocused ? "c" : "" }}) </template>
      <template v-if="isEditing">e</template>
      <template v-if="isFirstInGroup">[</template>
      <template v-if="isLastInGroup">]</template>
      <template v-if="meta.isCommented">#</template>
      {{ statement.modifier }}
      {{ statement.type }}
      <template v-if="statement.symbolType">{{ statement.symbolType }}:</template>
      <template v-if="statement.name != null">{{ statement.name }}</template>
      i:{{ statement.index }} d:{{ depth }}
    </span>
    <!-- Commented overlay -->
    <div v-if="meta.isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-50" />
    <!-- Monaco-like line numbers on the left margin -->
    <span
      class="absolute top-[7px] w-6 select-none text-right font-mono text-sm"
      :style="{ left: -30 + 'px' }"
      :class="{
        'text-orange-200': !isFocused && !meta.isComment,
        'text-gray-200': !isFocused && meta.isComment,
        'text-orange-400': isFamilyFocused && !meta.isComment,
        'text-gray-300': isFamilyFocused && meta.isComment,
        'font-bold text-orange-600': isFocused && !meta.isComment,
        'font-bold text-gray-400': isFocused && meta.isComment,
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
      <!-- TODO @Cleanup: factor out statement declaration component (the mess is above) -->
      <div
        v-if="!meta.isComment"
        class="decoration-none text-no-wrap relative flex flex-row items-baseline justify-start py-0.5 text-sm text-black"
      >
        <!-- Statement prefixxes (types & modifiers) -->
        <span class="mr-1 text-orange-600" v-if="meta.isImport">import</span>
        <span class="mr-1 text-orange-600" v-if="statement.modifier">{{ modifierShortname }}</span>
        <span class="mr-1 text-orange-600" v-if="statement.symbolType">{{ symbolTypeShortname }}</span>
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
                      {{ SYMBOL_TYPE_SHORTNAME[symbol.symbolType] }}
                    </template>
                    {{ symbol.name }}
                  </span>
                  <span class="truncate text-gray-500">
                    {{ getImportSourcePath(symbol) || symbol.file.pathWithoutExtension }}
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
        <span v-if="meta.isDefinition" class="-ml-0.5 font-bold text-orange-600">:</span>
        <span v-if="meta.isAlias" class="mx-1 text-orange-600">as</span>
        <!-- Editable alias -->
        <EditableSpan
          v-if="meta.isAlias"
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
        <span v-if="meta.isImport" class="mx-1 text-orange-600">from</span>
        <span v-if="meta.isImport && reference != null && meta.importPath != null">{{ meta.importPath }}</span>
        <span
          v-if="meta.isImport && (reference == null || meta.importPath == null)"
          class="text-gray-400 group-focus:animate-pulse"
          >...</span
        >
      </div>
      <!-- Meta & controls (top right) -->
      <span
        class="inline-flex flex-row items-center font-sans"
        :class="{
          'opacity-0 group-hover:opacity-100': !meta.isDefinition && !isFocused,
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
      <span class="text-red-500" v-else> cannot render {{ statement.symbolType }} </span>
    </div>
    <!-- Comment content -->
    <!-- TODO @Cleanup: comment content should probably be just another component -->
    <!-- TODO @Cleanup: use proper comment styling instead of opacity -->
    <div v-else-if="meta.isComment" class="mx-3 my-1 py-[0.5px]">
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
