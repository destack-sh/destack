<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import CodeDefinitionCell from "@/components/cells/CodeDefinitionCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import DataDefinitionCell from "@/components/cells/DataDefinitionCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import ProtoCell from "@/components/cells/ProtoCell.vue";
import TaskDefinitionCell from "@/components/cells/TaskDefinitionCell.vue";
import TypeDefinitionCell from "@/components/cells/TypeDefinitionCell.vue";
import { useMagicActions, useNavigationContext } from "@/components/file";
import { STATEMENT_CONTEXT, type StatementAction, type StatementContext } from "@/components/statement";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType, SymbolType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { getClientColor, useCurrentClients } from "@/state/client";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { isSymbolStale, localErrorsOf, symbolOf } from "@/state/runtime";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import { PlusIcon, Square2StackIcon, TrashIcon, XCircleIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, provide, ref, watch, type Component, type Ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  ancestors: FragmentType<typeof StatementContentType>[];
  readonly: boolean;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const ancestors = computed(() => props.ancestors.map((s) => useFragment(StatementContentType, s)));

const editor = useEditorState();
const nav = useNavigationContext();

const isFocused = computed(() => editor.focusedElementId == statement.value?.id);
const isEditing = computed(() => isFocused.value && editor.editingElement);
const isSelected = computed(() => editor.isSelected(statement.value));
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented || ancestors.value.find((s) => s.commented));
const isCommentish = computed(
  () => isComment.value || isCommented.value || statement.value.type == StatementType.Blank
);
const lineNumber = computed(() => nav.value?.statementPositions[statement.value.id] + 1 ?? 0);
const lineNumberDigits = computed(() => lineNumber.value.toString().length);

// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
const ancestorHighlightDepth = computed(() =>
  ancestors.value.findIndex((s) => editor.focusedElementId == s.id || editor.selectedElementIds.includes(s.id))
);
const isAncestorHighlight = computed(() => !editor.editingElement && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.depth * 20);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * 20 : contentOffsetX.value
);

// provide context
const destroyed = ref(false); // (useful for delete tracking if component had no time to update)
onBeforeUnmount(() => {
  destroyed.value = true;
});
const context: Ref<StatementContext> = computed(() => ({
  readonly: editor.readonly || props.readonly,
  focused: isFocused.value,
  editing: isEditing.value,
  depth: props.depth,
  xOffset: contentOffsetX.value,
  lineNumberBase: lineNumber.value,
  statement: props.statement,
  reference: symbolOf(statement.value.reference?.id) ?? null,
  file: props.file,
  destroyed: destroyed.value,
}));
provide(STATEMENT_CONTEXT, context);
const actions = useActions();
const magic = useMagicActions(statement as Ref<StatementHeader | null>);

// manage cells
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
const innerWrapperRef = ref<HTMLElement | null>(null);
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

const altKeyState = useKeyModifier("Alt");
const shiftKeyState = useKeyModifier("Shift");
// cancel focus if clicked outside (unless alt/shift is pressed)
onClickOutside(containerRef, () => {
  if (isFocused.value && !altKeyState.value && !shiftKeyState.value) {
    // We don't blur the root cell here because the focus is already elsewhere.
    editor.blurElement(statement.value as StatementHeader);
  }
});

// if anything inside the container becomes focused (except the container), enable editing mode
// (unless alt is pressed) :AltKeyEditing
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
  // create selection to here if shift was pressed
  if (e.shiftKey) {
    const index = nav.value.statementPositions[statement.value.id];
    const lastIndex = nav.value.statementPositions[editor.focusedElementId ?? ""];
    console.log("select all statements between", index, lastIndex);
    focusInEditor();
    if (lastIndex != null) {
      // select all statements between
      for (let i = Math.min(index, lastIndex); i <= Math.max(index, lastIndex); i++) {
        const statement = nav.value.statements[i];
        if (statement != null) {
          editor.addToSelection(statement);
        }
      }
    }
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

function insertStatementOnClick(e: MouseEvent) {
  onClickContainer(e);
  (e.altKey ? magic.insertAbove : magic.insertBelow)(true);
  e.preventDefault();
  e.stopPropagation();
}

// drag & drop
const innerDrag = computed(() => (rootCellRef.value as any)?.innerDrag == true);
const {
  isOverDropZone: dragOver,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(
  containerRef,
  ["Statement", "File"],
  onDrop,
  computed(() => !innerDrag.value && !props.readonly)
);

// TODO @Broken @UX: drag & drop doesn't work while holding shift, which means we can't move selections
//  (also would need to include other elements (incl. descendants) in drag image)
function onDragStart(e: DragEvent) {
  if (innerWrapperRef.value == null) return;
  if (e.dataTransfer == null) throw new Error("no dataTransfer??");
  setDragData(e, { type: "Statement", id: statement.value.id });
  e.dataTransfer.setDragImage(innerWrapperRef.value, 0, 0);
}

async function onDrop(thing: File[] | { type: string; id: string } | null) {
  if (thing == null) return;
  if (Array.isArray(thing)) {
    console.log("drop insert files into new statement", thing);
    await magic.insertFilesAsDataset(dragInTopHalf.value ? "above" : "below", thing);
  } else if (thing?.type == "Statement") {
    const targetStatement = nav.value.statementsById[thing.id];
    if (
      thing.id == statement.value.id ||
      targetStatement == null ||
      nav.value.isDescendantOf(targetStatement, statement.value as StatementHeader) ||
      nav.value.isDescendantOf(statement.value as StatementHeader, targetStatement)
    ) {
      return;
    }
    const dropLocation = dragInTopHalf.value
      ? nav.value.getLocationRightAbove(statement.value as StatementHeader)
      : nav.value.getLocationRightBelow(statement.value as StatementHeader);
    console.log("drop move statement", thing, dropLocation);
    await nav.value.moveTo(targetStatement as StatementHeader, dropLocation);
  } else {
    throw new Error("unexpected drop");
  }
}

// actions
const defaultActions: StatementAction[] = [
  {
    label: "Duplicate",
    icon: Square2StackIcon,
    action: () => magic.duplicate(),
  },
  {
    label: "Delete",
    icon: TrashIcon,
    action: () => magic.delete(),
  },
];

// runtime
const localErrors = localErrorsOf(statement);
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);
const isStale = isSymbolStale(statement);

// connected clients / multiplayer
// TODO @Performance: don't update & render clients per statement (ideally per file?)
const clients = useCurrentClients();
const filteredClients = computed(() =>
  clients.activeClientsWithoutSelf.value.filter((c) => c.statement?.id == statement.value.id)
);
</script>
<template>
  <!-- Statement wrapper -->
  <div class="group/statement relative w-full px-[50px]" @click="onClickContainer">
    <!-- Statement main -->
    <div
      tabindex="-1"
      ref="containerRef"
      @dragstart="onDragStart"
      class="relative min-h-[30px] w-full outline-none transition duration-150 focus:outline-none"
      :class="{
        'focus:bg-orange-100': !isCommentish,
        'focus:bg-gray-100': isCommentish,
        'bg-orange-100': !isCommentish && (isSelected || isAncestorHighlight || dragOver),
        'bg-gray-100': isCommentish && (isSelected || isAncestorHighlight || dragOver),
        'font-mono': editor.fontMono && !isComment,
        'text-gray-700': isCommented,
      }"
      :style="{
        marginLeft: highlightOffsetX + 'px',
        paddingLeft: contentOffsetX - highlightOffsetX + 'px',
        width: `calc(100% - ${highlightOffsetX}px)`,
      }"
    >
      <!-- Left gutter -->
      <!-- Small positioning hack to get content right-aligned on absolute left offset -->
      <div class="absolute -left-1 top-0 z-[5]">
        <div class="relative">
          <div class="absolute right-0 flex flex-row-reverse items-center gap-0.5">
            <!-- Monaco-like line number and drag handle -->
            <!-- TODO @Broken: fix dragging (broke when wrapping span in action popover button) -->
            <ActionPopover :thing="statement" :actions="defaultActions" v-slot="{ open }">
              <span
                class="select-none text-right not-italic transition duration-150"
                :class="{
                  'opacity-0': !isFocused && !open && !editor.showLineNumbers,
                  'group-focus-within/statement:opacity-100 group-hover/statement:opacity-100': !editor.showLineNumbers,
                  'text-sm': editor.textSmall,
                  'text-md': !editor.textSmall,
                  'font-mono': editor.fontMono,
                  'text-orange-200 hover:bg-orange-100 group-focus-within/statement:font-bold group-focus-within/statement:text-orange-500 group-hover/statement:font-bold group-hover/statement:text-orange-500 group-focus/statement:text-orange-500':
                    !isCommentish,
                  'text-gray-200 hover:bg-gray-100 group-focus-within/statement:font-bold group-focus-within/statement:text-gray-500 group-hover/statement:font-bold group-hover/statement:text-gray-500 group-focus/statement:text-gray-500':
                    isCommentish,
                  'text-orange-500': (dragOver || open || isFocused) && !isCommentish,
                  'text-gray-500': (dragOver || open || isFocused) && isCommentish,
                  'cursor-grab': !context.readonly,
                }"
                @mousedown="context.readonly || containerRef?.setAttribute('draggable', 'true')"
                @mouseup="context.readonly || containerRef?.setAttribute('draggable', 'false')"
              >
                {{ lineNumber }}
              </span>
            </ActionPopover>
            <!-- Add statement below button -->
            <button
              v-if="!context.readonly"
              class="rounded-sm p-0.5 text-gray-500 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:opacity-100"
              :class="isFocused ? 'opacity-100' : 'opacity-0'"
              @click="insertStatementOnClick"
            >
              <PlusIcon class="h-4 w-4" />
            </button>
            <!-- Other connected clients -->
            <div
              v-for="client in filteredClients"
              :key="client.id"
              class="rounded-sm px-1 py-0.5 text-gray-700"
              :class="[editor.textSmall ? 'text-xs' : 'text-sm']"
              :style="{
                backgroundColor: getClientColor(client.id),
              }"
            >
              {{ client.user.username.slice(0, 2).toLocaleUpperCase() }}
            </div>
          </div>
        </div>
      </div>
      <!-- Commented overlay (TODO @UX: commented overlay is ugly) -->
      <div v-if="isCommented" class="absolute inset-0 z-[8] bg-gray-100 opacity-25" />
      <!-- Statement drag & drop indicator (top/bottom) -->
      <div
        class="absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="!readonly && dragOver && dragInTopHalf ? 'opacity-100' : 'opacity-0'"
      />
      <div
        class="absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="!readonly && dragOver && dragInBottomHalf ? 'opacity-100' : 'opacity-0'"
      />
      <!-- Main cell -->
      <div
        ref="innerWrapperRef"
        class="relative px-2 py-1"
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
          @navigate-up="magic.moveFocusUp"
          @navigate-down="magic.moveFocusDown"
        />
        <component v-else ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
      </div>
      <!-- Gutter indicators on the right margin -->
      <div
        v-if="statement.type == StatementType.Definition || statement.type == StatementType.Reference"
        class="absolute left-full top-[6px] flex origin-top-right select-none flex-row gap-2 px-1 not-italic"
        :class="{
          'text-md': !editor.textSmall,
          'text-sm': editor.textSmall,
        }"
      >
        <!-- Errors/warnings -->
        <div>
          <!-- Errors -->
          <button
            v-if="hasLocalErrors"
            class="flex rounded-sm font-bold text-red-700 underline-offset-4 hover:bg-red-100 hover:text-red-900"
            @click="actions.apply('editor.view.openIssues')"
          >
            <XCircleIcon class="h-5 w-5" />
          </button>
          <!-- Warnings (don't exist yet) -->
        </div>
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="editor.debug" class="absolute -right-1 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
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
