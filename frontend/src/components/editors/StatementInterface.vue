<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import CodeDefinitionCell from "@/components/cells/CodeDefinitionCell.vue";
import CommentCell from "@/components/cells/CommentCell.vue";
import DataDefinitionCell from "@/components/cells/DataDefinitionCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import ProtoCell from "@/components/cells/ProtoCell.vue";
import TaskDefinitionCell from "@/components/cells/TaskDefinitionCell.vue";
import TypeDefinitionCell from "@/components/cells/TypeDefinitionCell.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { getClientColor, useCurrentClients } from "@/state/client";
import { useBenchState, useEditorContext, type StatementAction, type StatementHeader } from "@/state/bench";
import { useMagicActions, useNavigationContext } from "@/state/file";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { STATEMENT_CONTEXT, type StatementContext } from "@/state/statement";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import {
  ArrowsPointingOutIcon,
  PencilIcon,
  PlusIcon,
  Square2StackIcon,
  TrashIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { onClickOutside, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, provide, ref, toRef, watch, type Component, type Ref } from "vue";
import { useCurrentModule } from "@/state/module";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  depth: number;
  ancestors: FragmentType<typeof StatementContentType>[];
  readonly: boolean;
  standalone: boolean;
}>();
const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const ancestors = computed(() => props.ancestors.map((s) => useFragment(StatementContentType, s)));

const bench = useBenchState();
const appearance = useAppearance();
const nav = useNavigationContext(!props.standalone);
const editor = useEditorContext();
const module = useCurrentModule();
const actions = useActions();
const magic = useMagicActions(statement as Ref<StatementHeader | null>);

const isActive = computed(() => props.standalone || nav?.value.editor.activeStatementId == statement.value?.id);
const isFocused = computed(() => isActive.value && (props.standalone || nav?.value.editor.focused));
const isEditing = computed(() => isFocused.value && (props.standalone || nav?.value.editor.editing));
const isSelected = computed(() => nav?.value.editor.isSelected(statement.value));
const isComment = computed(() => statement.value?.type == StatementType.Comment);
const isCommented = computed(() => statement.value?.commented || ancestors.value.find((s) => s.commented));
const isCommentish = computed(
  () => isComment.value || isCommented.value || statement.value.type == StatementType.Blank
);
const lineNumber = computed(() => (nav?.value?.statementPositions[statement.value.id] ?? -2) + 1);

// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
const ancestorHighlightDepth = computed(() =>
  ancestors.value.findIndex(
    (s) => nav?.value.editor.activeStatementId == s.id || nav?.value.editor.selectedElementIds.includes(s.id)
  )
);
const isAncestorHighlight = computed(() => !nav?.value.editor.editing && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.depth * 20);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * 20 : contentOffsetX.value
);

// provide context
const destroyed = ref(false); // (useful for delete tracking if component had no time to update)
onBeforeUnmount(() => {
  destroyed.value = true;
});
provide(STATEMENT_CONTEXT, {
  readonly: computed(() => bench.readonly || props.readonly),
  active: isActive,
  focused: isFocused,
  editing: isEditing,
  standalone: toRef(props, "standalone"),
  depth: toRef(props, "depth"),
  xOffset: contentOffsetX,
  lineNumberBase: lineNumber,
  statement,
  reference: computed(() => module.statementOf(statement.value.reference?.id) ?? null),
  file,
  destroyed,
} as StatementContext);
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
  } else if (statement.value.type == StatementType.Type) {
    return {
      component: TypeDefinitionCell,
    };
  } else if (statement.value.type == StatementType.Task) {
    return {
      component: TaskDefinitionCell,
      props: { isTyped: true },
    };
  } else if (statement.value.type == StatementType.Expectation) {
    return {
      component: TaskDefinitionCell,
      props: { isTyped: false },
    };
  } else if (statement.value.type == StatementType.Code) {
    return {
      component: CodeDefinitionCell,
    };
  } else if (statement.value.type == StatementType.Dataset || statement.value.type == StatementType.Value) {
    return {
      component: DataDefinitionCell,
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
const { focused: inRootCellFocused } = useFocusWithin(innerWrapperRef);

// scroll into view when becoming active
whenever(isActive, () => {
  if (isEditing.value) return; //  (but not editing, which would focus an actual HTML element)
  // scroll into view if not visible
  const editorRect = editor.container.value?.getBoundingClientRect();
  const containerRect = containerRef.value?.getBoundingClientRect();
  if (editorRect != null && containerRect != null) {
    if (
      containerRect.top < editorRect.top + appearance.editorHeaderHeight ||
      containerRect.bottom > editorRect.bottom
    ) {
      // TODO @UX: improve scroll behavior (feels a bit janky sometimes)
      containerRef.value?.scrollIntoView(false);
    }
  }
});

// focus root cell if editing in editor but not in container
whenever(
  isEditing,
  () => {
    if (isEditing.value && !inContainerFocused.value) {
      rootCellRef.value?.focus();
      nextTick(() => rootCellRef.value?.focus()); // required to focus if just loaded
    }
  },
  { immediate: true }
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

// blur root cell if focused in container but no longer editing or focused
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
// cancel focus if clicked outside this statement in our editor (unless alt/shift is pressed)
onClickOutside(containerRef, (e) => {
  if (
    isFocused.value &&
    !altKeyState.value &&
    !shiftKeyState.value &&
    editor.container.value?.parentNode?.contains(e.target as Node)
  ) {
    // we don't blur the root cell here because the focus is already elsewhere
    nav?.value.editor.blurElement(statement.value as StatementHeader);
  }
});

// if anything inside the container becomes focused (except the container), enable editing mode
// (unless alt is pressed) :AltKeyEditing
whenever(inRootCellFocused, () => {
  if (!isFocused.value) {
    focusInEditor();
  }
  if (altKeyState.value) {
    return;
  }
  if (!isEditing.value && !bench.readonly) {
    nav?.value.editor.editElement(statement.value as StatementHeader);
  }
});

function focusInEditor() {
  if (props.standalone) {
    bench.focusStatement(statement.value as any);
  } else {
    bench.focusFile(file.value as any);
    nav?.value.editor.focusElement(statement.value as StatementHeader);
  }
}

function onClickContainer(e: MouseEvent) {
  // ignore if alt was pressed :AltKeyEditing
  if (e.altKey) {
    return;
  }
  // create selection to here if shift was pressed
  if (e.shiftKey && nav != null) {
    const index = nav.value.statementPositions[statement.value.id];
    const lastIndex = nav.value.statementPositions[nav.value.editor.activeStatementId ?? ""];
    console.log("select all statements between", index, lastIndex);
    focusInEditor();
    if (lastIndex != null) {
      // select all statements between
      for (let i = Math.min(index, lastIndex); i <= Math.max(index, lastIndex); i++) {
        const statement = nav.value.statements[i];
        if (statement != null) {
          nav.value.editor.addToSelection(statement);
        }
      }
    }
    return;
  }
  if (!isFocused.value) {
    focusInEditor();
  }
  if (!bench.readonly) {
    nav?.value.editor.editElement(statement.value as StatementHeader);
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
  ["Statement", "NativeFile"],
  onDrop,
  computed(() => !innerDrag.value && !props.readonly)
);

// TODO @Broken @UX: drag & drop doesn't work while holding shift, which means we can't move selections
//  (also would need to include other elements (incl. descendants) in drag image)
function onDragStart(e: DragEvent) {
  if (innerWrapperRef.value == null) return;
  setDragData(e, { type: "Statement", id: statement.value.id });
  e.dataTransfer?.setDragImage(innerWrapperRef.value, 0, 0);
}

async function onDrop(thing: File[] | { type: string; id: string } | null) {
  if (thing == null || nav == null) return;
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
const canOpenInStandaloneEditor = computed(() => !props.standalone);
const defaultActions: Ref<StatementAction[]> = computed(() => {
  const actions = [];
  if (canOpenInStandaloneEditor.value) {
    actions.push({
      label: "Open in Editor",
      icon: ArrowsPointingOutIcon,
      disabled: props.standalone,
      action: () => {
        nav?.value.editor.bench.openStatement(statement.value as StatementHeader, { focus: true });
      },
    });
  }
  actions.push({
    label: "Rename",
    icon: PencilIcon,
    action: () => {
      nav?.value.editor.editElement(statement.value as StatementHeader);
      nextTick(() => rootCellRef.value?.focus());
    },
  });
  if (!props.standalone) {
    // doesn't work in standalone editor because it needs file context right now
    actions.push({
      label: "Duplicate",
      icon: Square2StackIcon,
      action: () => magic.duplicate(),
    });
  }
  actions.push({
    label: "Delete",
    icon: TrashIcon,
    action: () => magic.delete(),
  });
  return actions;
});

// runtime
const localErrors = module.localErrorsOf(statement);
const hasLocalErrors = computed(() => (localErrors.value?.length ?? 0) > 0);

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    return rootCellRef.value?.focus(position);
  },
  blur: () => rootCellRef.value?.blur(),
  root: rootCellRef,
});
</script>
<template>
  <!-- Statement wrapper -->
  <div
    class="group/statement relative w-full"
    :style="editor.editor.value.contentMarginXAsPaddingX"
    @click="onClickContainer"
  >
    <!-- Statement main -->
    <div
      ref="containerRef"
      @dragstart.stop="onDragStart"
      class="relative min-h-[30px] w-full rounded-sm outline-none transition duration-150 focus:outline-none"
      :class="{
        'focus:bg-orange-100': !isCommentish,
        'focus:bg-gray-100': isCommentish,
        'bg-orange-100': !isCommentish && ((isFocused && !isEditing) || isSelected || isAncestorHighlight || dragOver),
        'bg-gray-100': isCommentish && ((isFocused && !isEditing) || isSelected || isAncestorHighlight || dragOver),
        'text-gray-700': isCommented,
        ...appearance.baseClass,
      }"
      :style="{
        marginLeft: highlightOffsetX + 'px',
        paddingLeft: contentOffsetX - highlightOffsetX + 'px',
        width: `calc(100% - ${highlightOffsetX}px)`,
      }"
    >
      <!-- Left gutter -->
      <!-- Small positioning hack to get content right-aligned on absolute left offset -->
      <div class="absolute -left-1 top-0.5 z-[5]">
        <div class="relative">
          <div class="absolute right-0 flex flex-row-reverse items-center gap-0.5">
            <!-- Monaco-like line number and drag handle -->
            <ActionPopover
              v-if="lineNumber >= 0"
              anchor="right"
              :thing="statement"
              :actions="defaultActions"
              v-slot="{ open }"
              @mousedown="containerRef?.setAttribute('draggable', 'true')"
              @mouseup="containerRef?.setAttribute('draggable', 'false')"
              @click.stop
            >
              <span
                class="cursor-grab select-none text-right not-italic transition duration-150"
                :class="{
                  'opacity-0 group-hover/statement:opacity-100': !isActive && !open && !bench.showLineNumbers,
                  'opacity-100': isActive && !bench.showLineNumbers,
                  'text-orange-200 hover:bg-orange-100 group-hover/statement:font-bold group-hover/statement:text-orange-500':
                    !isCommentish,
                  'text-gray-200 hover:bg-gray-100  group-hover/statement:font-bold group-hover/statement:text-gray-500':
                    isCommentish,
                  'text-orange-500': (dragOver || open || isActive) && !isCommentish,
                  'text-gray-500': (dragOver || open || isActive) && isCommentish,
                  ...appearance.baseClass,
                }"
              >
                {{ lineNumber }}
              </span>
            </ActionPopover>
            <!-- Add statement below button -->
            <button
              v-if="!standalone && !bench.readonly && !props.readonly"
              class="rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:opacity-100"
              :class="isActive ? 'opacity-100' : 'opacity-0'"
              @click="insertStatementOnClick"
            >
              <PlusIcon class="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>
      <!-- Commented overlay (TODO @UX: commented overlay is ugly) -->
      <div v-if="isCommented" class="absolute inset-0 z-[8] bg-gray-100 opacity-25" />
      <!-- Statement drag & drop indicator (top/bottom) :DragStyle -->
      <div
        v-if="!readonly"
        class="absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="dragOver && dragInTopHalf ? 'opacity-100' : 'opacity-0'"
      />
      <div
        v-if="!readonly"
        class="absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="dragOver && dragInBottomHalf ? 'opacity-100' : 'opacity-0'"
      />
      <!-- Main cell -->
      <!-- :StatementPadding -->
      <div
        ref="innerWrapperRef"
        class="relative px-2 py-1"
        :class="{
          'text-sm': bench.textSmall,
          'text-md': !bench.textSmall,
        }"
      >
        <!-- Most cells handle these events themselves, this is for raw DeclarationCells -->
        <component
          v-if="rootCell.component == DeclarationCell"
          :is="rootCell.component"
          ref="rootCellRef"
          @navigate-up="magic.moveFocusUp"
          @navigate-down="magic.moveFocusDown"
        />
        <component v-else ref="rootCellRef" :is="rootCell.component" v-bind="rootCell.props" />
      </div>
      <!-- Gutter indicators on the right margin -->
      <div
        v-if="statement.type == StatementType.Symbol || statement.type == StatementType.Reference"
        class="absolute left-full top-[6px] flex origin-top-right select-none flex-row gap-2 px-1 not-italic"
        :class="{
          'text-md': !bench.textSmall,
          'text-sm': bench.textSmall,
        }"
      >
        <!-- Issues -->
        <div class="group/issues">
          <!-- Errors -->
          <button
            v-if="hasLocalErrors"
            class="flex rounded-sm font-bold text-red-700 underline-offset-4 hover:bg-red-100 hover:text-red-900"
            @click="actions.apply('bench.view.openIssues')"
          >
            <XCircleIcon class="h-5 w-5" />
          </button>
          <!-- Warnings (don't exist yet) -->
          <!-- Preview on hover -->
          <div
            v-if="hasLocalErrors"
            class="invisible absolute right-0 z-10 flex w-fit max-w-3xl flex-col gap-1 whitespace-normal rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-1 shadow-sm group-hover/issues:visible"
          >
            <span v-for="error in localErrors" :key="error.id" class="text-red-700">
              {{ error.message }}
            </span>
          </div>
        </div>
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="bench.debug" class="absolute -right-1 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <template v-if="isAncestorHighlight">h{{ ancestorHighlightDepth }}</template>
      <template v-if="isActive">A</template>
      <template v-if="isFocused">F</template>
      <template v-if="isEditing">e</template>
      <template v-if="isSelected">S</template>
      <template v-if="inContainerFocused">*</template>
      <template v-if="inRootCellFocused">r*</template>
      <template v-if="isCommented">#</template>
      <span class="lowercase">
        {{ statement.modifier }}
        {{ statement.type }}
        <template v-if="statement.type">{{ statement.type }}:</template>
      </span>
      <template v-if="statement.name != null">{{ statement.name }}</template>
      r:{{ statement.revision }} i:{{ statement.orderKey }} d:{{ depth }}
    </div>
  </div>
</template>
