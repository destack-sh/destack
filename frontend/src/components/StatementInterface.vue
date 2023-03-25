<script lang="ts" setup>
import CodeDefinitionCell from "@/components/cells/CodeDefinitionCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import DataDefinitionCell from "@/components/cells/DataDefinitionCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import ProtoCell from "@/components/cells/ProtoCell.vue";
import TaskDefinitionCell from "@/components/cells/TaskDefinitionCell.vue";
import TypeDefinitionCell from "@/components/cells/TypeDefinitionCell.vue";
import { STATEMENT_CONTEXT, type StatementContext } from "@/components/statement";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { isSymbolStale, localErrorsOf, symbolOf } from "@/state/runtime";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import { computed, nextTick, provide, ref, watch, type Component, type Ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  ancestors: string[];
  readonly: boolean;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  lineNumberBase: number;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));

const editor = useEditorState();

const isFocused = computed(() => editor.focusedElementId == statement.value?.id);
const isEditing = computed(() => isFocused.value && editor.editingElement);
const isSelected = computed(() => editor.isSelected(statement.value));
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented);
const isCommentish = computed(
  () => isComment.value || isCommented.value || statement.value.type == StatementType.Blank
);

// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
const ancestorHighlightDepth = computed(() =>
  props.ancestors.findIndex((s) => editor.focusedElementId == s || editor.selectedElementIds.includes(s))
);
const isAncestorHighlight = computed(() => !editor.editingElement && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.depth * 20);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * 20 : contentOffsetX.value
);

// manage cells
const context: Ref<StatementContext> = computed(() => ({
  readonly: editor.readonly || props.readonly,
  focused: isFocused.value,
  editing: isEditing.value,
  depth: props.depth,
  xOffset: contentOffsetX.value,
  lineNumberBase: props.lineNumberBase,
  statement: props.statement,
  reference: symbolOf(statement.value.reference?.id) ?? null,
  file: props.file,
}));
provide(STATEMENT_CONTEXT, context);
const actions = useActions();

type Cell = {
  component: Component;
  props?: any;
};

const rootCell: Ref<Cell> = computed(() => {
  if (statement.value.type == StatementType.Comment) {
    return {
      component: CommentCell,
    };
  } else if (statement.value.type == StatementType.Definition) {
    if (statement.value.symbolType == SymbolType.Type) {
      return {
        component: TypeDefinitionCell,
      };
    } else if (statement.value.symbolType == SymbolType.Task) {
      return {
        component: TaskDefinitionCell,
        props: { isTyped: true },
      };
    } else if (statement.value.symbolType == SymbolType.Expectation) {
      return {
        component: TaskDefinitionCell,
        props: { isTyped: false },
      };
    } else if (statement.value.symbolType == SymbolType.Code) {
      return {
        component: CodeDefinitionCell,
      };
    } else if (statement.value.symbolType == SymbolType.Data) {
      return {
        component: DataDefinitionCell,
      };
    }

    // default to just declaration cell
    return {
      component: DeclarationCell,
    };
  } else if (statement.value.type == StatementType.Reference) {
    return {
      component: DeclarationCell,
    };
  }

  // default to empty cell
  return {
    component: ProtoCell,
    props: { showDots: true },
  };
});

const containerRef = ref<HTMLElement | null>(null);
const rootCellRef = ref<InstanceType<typeof ProtoCell>>();
const { focused: inContainerFocused } = useFocusWithin(containerRef);
const { focused: containerFocused } = useFocus(containerRef);
const { focused: inRootCellFocused } = useFocusWithin(rootCellRef);

// focus containerRef if focused in editor but not in container and not editing
watch(
  () => [isFocused.value, isEditing.value, containerFocused.value],
  () => {
    if (isFocused.value && !isEditing.value && !inRootCellFocused.value && !containerFocused.value) {
      containerFocused.value = true;
    }
  }
);

// refocus if root cell changed and we're editing
watch(
  () => rootCell.value.component,
  (oldComponent, newComponent) => {
    if (isEditing.value && oldComponent !== newComponent) {
      nextTick(() => rootCellRef.value?.focus());
    }
  },
  { deep: false }
);

// focus root cell if editing in editor but not in container
whenever(
  isEditing,
  () => {
    if (isEditing.value && (!inContainerFocused.value || containerFocused.value)) {
      rootCellRef.value?.focus();
      nextTick(() => rootCellRef.value?.focus()); // required to focus if just loaded
    }
  },
  { immediate: true }
);

// blur root cell if focused in container (but no longer editing or focused)
watch(
  () => [isEditing.value, inRootCellFocused.value],
  () => {
    if (!isEditing.value && inRootCellFocused.value) {
      rootCellRef.value?.blur();
    }
  }
);

// cancel focus if clicked outside
onClickOutside(containerRef, () => {
  if (isFocused.value) {
    // We don't blur the root cell here because the focus is already elsewhere.
    editor.blurElement(statement.value as StatementHeader);
  }
});

// if anything inside the container becomes focused (except the container), enable editing mode
// (unless alt is pressed) :AltKeyEditing
const altKeyState = useKeyModifier("Alt");
whenever(inRootCellFocused, () => {
  if (!isFocused.value) {
    focusInEditor();
  }
  if (altKeyState.value || containerFocused.value) {
    return;
  }
  if (!isEditing.value && !editor.readonly) {
    editor.editElement(statement.value as StatementHeader);
  }
});

function focusInEditor() {
  editor.focusFile(file.value as any);
  editor.focusElement(statement.value as StatementHeader);
}

function onClickContainer(e: MouseEvent) {
  // ignore if alt was pressed :AltKeyEditing
  if (e.altKey) {
    return;
  }
  if (!isFocused.value) {
    focusInEditor();
  }
  if (!editor.readonly) {
    editor.editElement(statement.value as StatementHeader);
    containerFocused.value = false;
  } else {
    containerFocused.value = true;
  }
  if (!inContainerFocused.value) {
    rootCellRef.value?.focus();
  }
}

function insertStatementBelow(e: MouseEvent) {
  onClickContainer(e);
  actions.apply("statement.insertBelowCurrent");
  e.preventDefault();
  e.stopPropagation();
}

// runtime
const localErrors = localErrorsOf(statement);
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);
const isStale = isSymbolStale(statement);
</script>
<template>
  <!-- Statement wrapper -->
  <div class="group/statement relative w-full px-[50px]" @click="onClickContainer">
    <!-- Add statement below button -->
    <button
      v-if="!context.readonly"
      class="invisible absolute top-0.5 rounded-sm p-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:visible"
      :style="{
        left: highlightOffsetX + (highlightOffsetX != 0 ? 28 : 6) + 'px',
      }"
      @click="insertStatementBelow"
    >
      <PlusIcon class="h-4 w-4" />
    </button>
    <!-- Monaco-like line numbers on the left margin -->
    <span
      v-if="editor.showLineNumbers"
      class="absolute top-[3px] w-6 select-none text-right not-italic transition duration-75"
      :style="{ transform: 'translateX(' + -30 + 'px)' }"
      :class="{
        'text-sm': editor.textSmall,
        'text-md': !editor.textSmall,
        'font-mono': editor.fontMono,
        'text-orange-200 group-focus-within/statement:font-bold group-focus-within/statement:text-orange-500 group-hover/statement:font-bold group-hover/statement:text-orange-500 group-focus/statement:text-orange-500':
          !isCommentish,
        'text-gray-200 group-focus-within/statement:font-bold group-focus-within/statement:text-gray-500 group-hover/statement:font-bold group-hover/statement:text-gray-500 group-focus/statement:text-gray-500':
          isCommentish,
      }"
    >
      {{ lineNumberBase + 1 }}
    </span>
    <!-- Gutter indicators on the right margin -->
    <span
      class="absolute right-0 top-[6px] select-none text-left text-sm font-bold not-italic"
      :style="{ right: -12 + 'px' }"
    >
      <span class="text-red-700" v-if="hasLocalErrors">
        {{ localErrors?.length }}
      </span>
    </span>
    <!-- Statement main -->
    <div
      tabindex="-1"
      ref="containerRef"
      class="relative min-h-[30px] w-full outline-none transition duration-75 focus:outline-none"
      :class="{
        'focus:bg-orange-100': !isCommentish,
        'focus:bg-gray-100': isCommentish,
        'bg-orange-100': !isCommentish && (isSelected || isAncestorHighlight),
        'bg-gray-100': isCommentish && (isSelected || isAncestorHighlight),
        'font-mono': editor.fontMono && !isComment,
        'text-gray-700': isCommented,
      }"
      :style="{
        marginLeft: highlightOffsetX + 'px',
        paddingLeft: contentOffsetX - highlightOffsetX + 'px',
        width: `calc(100% - ${highlightOffsetX}px)`,
      }"
    >
      <!-- TODO @UX: focus on @mousedown would be more responsive but doesn't focus properly.. -->
      <!-- Commented overlay (TODO @UX: commented overlay is ugly) -->
      <div v-if="isCommented" class="absolute inset-0 z-20 bg-gray-100 opacity-25" />
      <!-- Statement focus indicator (left side if not editing) -->
      <!-- (the z-[5] puts it in front of the statement focus border) -->
      <div
        class="absolute -left-0.5 top-0 z-[5] h-full w-1.5 transition-colors duration-75"
        :class="{
          'group-focus-within/statement:bg-orange-200 group-hover/statement:bg-orange-300': !isCommentish,
          'group-focus-within/statement:bg-gray-200 group-hover/statement:bg-gray-300': isCommentish,
        }"
      />
      <!-- Statement focus indicator (all around if editing) -->
      <template v-if="isEditing">
        <div
          class="duration-50 absolute top-0 left-0 h-0.5 w-full transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute bottom-0 left-0 h-0.5 w-full transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute top-0 left-0 h-full w-0.5 transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
        <div
          class="duration-50 absolute right-0 top-0 h-full w-0.5 transition-colors"
          :class="isCommentish ? 'bg-gray-200' : 'bg-orange-200'"
        />
      </template>
      <!-- Main cell -->
      <div
        class="relative py-1 px-2"
        :class="{
          'text-sm': editor.textSmall,
          'text-md': !editor.textSmall,
        }"
      >
        <!-- Most cells handle these events themselves, this is for raw DeclarationCells -->
        <component
          v-if="rootCell.component == DeclarationCell"
          ref="rootCellRef"
          :is="rootCell.component"
          @navigate-up="actions.apply('statement.moveFocusUp')"
          @navigate-down="actions.apply('statement.moveFocusDown')"
        />
        <component v-else ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="editor.debug" class="absolute top-2 -right-1 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <template v-if="isAncestorHighlight">h{{ ancestorHighlightDepth }}</template>
      <template v-if="isFocused">F</template>
      <template v-if="isSelected">S</template>
      <template v-if="inContainerFocused">*</template>
      <template v-if="containerFocused">.</template>
      <template v-if="isEditing">e</template>
      <template v-if="isFirstInGroup">[</template>
      <template v-if="isLastInGroup">]</template>
      <template v-if="isCommented">#</template>
      <template v-if="isStale">S</template>
      <span class="lowercase">
        {{ statement.modifier }}
        {{ statement.type }}
        <template v-if="statement.symbolType">{{ statement.symbolType }}:</template>
      </span>
      <template v-if="statement.name != null">{{ statement.name }}</template>
      r:{{ statement.revision }} i:{{ statement.orderKey }} d:{{ depth }}
    </div>
  </div>
</template>
