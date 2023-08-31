<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import BlankStatement from "@/components/statements/BlankStatement.vue";
import BlockStatement from "@/components/statements/BlockStatement.vue";
import CodeStatement from "@/components/statements/CodeStatement.vue";
import DatasetStatement from "@/components/statements/DatasetStatement.vue";
import ReferenceStatement from "@/components/statements/ReferenceStatement.vue";
import TaskStatement from "@/components/statements/TaskStatement.vue";
import TextStatement from "@/components/statements/TextStatement.vue";
import TypeStatement from "@/components/statements/TypeStatement.vue";
import ValueStatement from "@/components/statements/ValueStatement.vue";
import { IssueKind, StatementType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import {
  useBenchState,
  usePanelContext,
  type StatementAction,
  type StatementHeader,
  EditFilePanel,
  type FileHeader,
} from "@/state/bench";
import { useMagicActions, useNavigationContext, type NavigationContext } from "@/state/file";
import { useCurrentModule, type Statement } from "@/state/module";
import {
  STATEMENT_CONTEXT,
  STATEMENT_STANDALONE_TYPES,
  STATEMENT_TYPE_LABELS,
  type StatementContext,
} from "@/state/statement";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import {
  ArrowsPointingOutIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  PencilSquareIcon,
  PlusIcon,
  Square2StackIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { ExclamationTriangleIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { onClickOutside, useElementBounding, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, provide, ref, toRef, watch, type Component, type Ref } from "vue";

const props = defineProps<{
  file: FileHeader;
  statement: Statement;
  depth: number;
  ancestors: Statement[];
  readonly: boolean;
  standalone: boolean;
  shown?: boolean;
}>();
const file = toRef(props, "file");
const statement = toRef(props, "statement");
const ancestors = toRef(props, "ancestors");

const bench = useBenchState();
const appearance = useAppearance();
const nav = useNavigationContext(!props.standalone);
const panel = usePanelContext();
const module = useCurrentModule();
const magic = useMagicActions(statement as Ref<StatementHeader | null>);

const location = computed(() => nav?.value?.getLocation(statement.value));
const isActive = computed(() => props.standalone || nav?.value?.panel.activeStatementCk == statement.value?.ck);
const isFocused = computed(() => isActive.value && (props.standalone || nav?.value?.panel.focused));
const isEditing = computed(() => isFocused.value && (props.standalone || nav?.value?.panel.editing));
const isSelected = computed(() => nav?.value?.panel.isSelected(statement.value));
const isInSelection = computed(() => isSelected.value && (nav?.value?.panel.selectedElementIds?.length ?? 0) > 1);
const canContentFold = computed(
  () =>
    statement.value.type != StatementType.Blank &&
    statement.value.type != StatementType.Reference &&
    statement.value.type != StatementType.Block
);
const isContentFolded = computed(
  () => !props.standalone && (panel.panel.value as EditFilePanel).isStatementContentFolded(statement.value)
);

function toggleContentFold(descendants?: boolean) {
  if (props.standalone) return;
  if (descendants) {
    (panel.panel.value as EditFilePanel).setStatementContentsFolded(
      module.getDescendantsOf(statement.value.id),
      !isContentFolded.value
    );
  } else {
    (panel.panel.value as EditFilePanel).toggleStatementContentFolded(statement.value);
  }
}

// highlighting
const INDENT_OFFSET_X = 28;
// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
const ancestorHighlightDepth = computed(() =>
  ancestors.value.findIndex(
    (s) => nav?.value?.panel.activeStatementCk == s.ck || nav?.value?.panel.selectedElementIds.includes(s.id)
  )
);
const isAncestorHighlight = computed(() => !nav?.value?.panel.editing && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.depth * INDENT_OFFSET_X);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * INDENT_OFFSET_X : contentOffsetX.value
);

// manage interfaces
type StatementInterface = {
  component: Component;
  props?: any;
};

// TODO @Cleanup @Architecture: unify statement interfaces/components
const statementInterface: Ref<StatementInterface> = computed(() => {
  if (statement.value.type == StatementType.Text) {
    return {
      component: TextStatement,
    };
  } else if (statement.value.type == StatementType.Type) {
    return {
      component: TypeStatement,
    };
  } else if (statement.value.type == StatementType.Task || statement.value.type == StatementType.Flow) {
    return {
      component: TaskStatement,
      props: { isTyped: true },
    };
  } else if (statement.value.type == StatementType.Expectation || statement.value.type == StatementType.Tag) {
    return {
      component: TaskStatement,
      props: { isTyped: false },
    };
  } else if (statement.value.type == StatementType.Code) {
    return {
      component: CodeStatement,
    };
  } else if (statement.value.type == StatementType.Dataset) {
    return {
      component: DatasetStatement,
    };
  } else if (statement.value.type == StatementType.Value) {
    return {
      component: ValueStatement,
    };
  } else if (statement.value.type == StatementType.Block) {
    return {
      component: BlockStatement,
    };
  } else if (statement.value.type == StatementType.Reference) {
    return {
      component: ReferenceStatement,
    };
  }

  // default to blank statement
  return {
    component: BlankStatement,
    props: { showDots: true },
  };
});

const containerRef = ref<HTMLElement | null>(null);
const containerBounding = useElementBounding(containerRef);
const innerWrapperRef = ref<HTMLElement | null>(null);
const statementRef = ref<InstanceType<typeof BlankStatement>>();
const actionPopoverRef = ref<InstanceType<typeof ActionPopover>>();
const { focused: inContainerFocused } = useFocusWithin(containerRef);
const { focused: inStatementFocused } = useFocusWithin(innerWrapperRef);

// update container bounding whenever location changes (since ResizeObserver doesn't seem to be triggered in that case)
watch(location, () => {
  containerBounding.update();
});

// scroll into view when becoming active
whenever(isActive, () => {
  if (isEditing.value) return; //  (but not editing, which would focus an actual HTML element)
  // scroll into view if not visible
  const editorRect = panel.container.value?.getBoundingClientRect();
  const containerRect = containerRef.value?.getBoundingClientRect();
  if (editorRect != null && containerRect != null) {
    if (containerRect.top < editorRect.top + appearance.panelHeaderHeight || containerRect.bottom > editorRect.bottom) {
      // TODO @UX: improve scroll behavior (feels a bit janky sometimes)
      containerRef.value?.scrollIntoView(false);
    }
  }
});

// focus statement interface if editing in editor but not in container
watch(
  () => [isEditing.value, props.shown],
  () => {
    if (isEditing.value && props.shown && !inContainerFocused.value) {
      statementRef.value?.focus();
    }
  },
  { immediate: true }
);

// refocus if statement interface changed and we're editing
watch(
  () => statementInterface.value.component,
  (oldComponent, newComponent) => {
    if (isEditing.value && oldComponent !== newComponent) {
      nextTick(() => statementRef.value?.focus());
    }
  },
  { deep: false }
);

// blur statement interface if focused in container but no longer editing or focused
watch(
  () => [isEditing.value, inStatementFocused.value],
  () => {
    if (!isEditing.value && inStatementFocused.value) {
      // it can take a frame for focus to take effect, so we wait a frame before blurring
      // (otherwise we may cancel focus before it happens)
      nextTick(() => {
        if (!isEditing.value && inStatementFocused.value) {
          statementRef.value?.blur();
        }
      });
    }
  }
);

const altKey = useKeyModifier("Alt");
const shiftKey = useKeyModifier("Shift");
// cancel focus if clicked outside this statement in our editor (unless alt/shift is pressed)
onClickOutside(containerRef, (e) => {
  if (
    isFocused.value &&
    !altKey.value &&
    !shiftKey.value &&
    panel.container.value?.parentNode?.contains(e.target as Node)
  ) {
    // we don't blur the statement interface here because the focus is already elsewhere
    nav?.value?.panel.blurElement(statement.value);
  }
});

// if anything inside the container becomes focused (except the container), enable editing mode
// (unless alt is pressed) :AltKeyEditing
whenever(inStatementFocused, () => {
  if (!isFocused.value) {
    focusInEditor();
  }
  if (altKey.value) {
    return;
  }
  if (!isEditing.value && !bench.readonly) {
    nav?.value?.panel.editElement(statement.value);
  }
});

function focusInEditor() {
  if (props.standalone) {
    bench.focusStatement(statement.value as any);
  } else {
    bench.focusFile(file.value as any);
    nav?.value?.panel.focusElement(statement.value);
  }
}

function onClickContainer(e: MouseEvent) {
  // create selection to here if shift was pressed
  if (e.shiftKey && nav != null) {
    const index = nav?.value?.statementPositions[statement.value.id];
    const lastIndex = nav?.value?.statementPositions[nav?.value?.panel.activeStatementCk ?? ""];
    console.log("select all statements between", index, lastIndex);
    focusInEditor();
    if (lastIndex != null) {
      // select all statements between
      for (let i = Math.min(index, lastIndex); i <= Math.max(index, lastIndex); i++) {
        const statement = nav?.value?.statements[i];
        if (statement != null) {
          nav?.value?.panel.addToSelection(statement);
        }
      }
    }
    return;
  } else if (nav?.value?.panel.hasSelection) {
    // clear selection if clicking outside
    nav?.value?.panel.clearSelection();
  }
}

function insertStatementOnClick(e: MouseEvent) {
  onClickContainer(e);
  (e.altKey ? magic.insertAbove : magic.insertBelow)(true);
  e.preventDefault();
  e.stopPropagation();
}

// provide context
const destroyed = ref(false); // (useful for delete tracking if component had no time to update)
onBeforeUnmount(() => {
  destroyed.value = true;
});
const context = {
  readonly: computed(() => bench.readonly || props.readonly),
  active: isActive,
  focused: isFocused,
  editing: isEditing,
  standalone: toRef(props, "standalone"),
  depth: toRef(props, "depth"),
  xOffset: contentOffsetX,
  statement,
  reference: computed(() => module.statementOf(statement.value.referenceCk) ?? null),
  file,
  bounding: containerBounding,
  destroyed,
  customActions: ref([]),
} as StatementContext;
provide(STATEMENT_CONTEXT, context);

// drag & drop
const innerDrag = computed(() => (statementRef.value as any)?.innerDrag == true);
const {
  isOverDropZone: dragOver,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(
  containerRef,
  ["Statement", "BrowserFile"],
  (thing) => onDrop(thing),
  computed(() => !innerDrag.value && !props.readonly)
);

function onDragStart(e: DragEvent) {
  if (innerWrapperRef.value == null) return;
  setDragData(e, { type: "Statement", id: statement.value.id });
  // TODO @Broken @UX: drag image looks horrible sometimes
  // (when statements have large hidden content like Code (Monaco infinite lines view) or Dataset (horizontal overscroll area))
  e.dataTransfer?.setDragImage(innerWrapperRef.value, 0, 0);
}

async function onDrop(thing: any[] | { type: string; id: string } | null) {
  if (thing == null || nav == null) return;
  if (Array.isArray(thing)) {
    console.log("drop insert files into new statement", thing);
    await magic.insertFilesAsDataset(dragInTopHalf.value ? "above" : "below", thing);
  } else if (thing?.type == "Statement") {
    if (nav?.value.panel.hasSelection && nav?.value.panel.selectedElementIds.length > 1) {
      // batch move
      const statementsToMove = nav.value.panel.selectedElementIds.map((id) => nav?.value?.statementsById[id]);
      if (
        statementsToMove.some(
          (s) =>
            statement.value.id == s?.id ||
            nav?.value?.isDescendantOf(s, statement.value) ||
            nav?.value?.isDescendantOf(statement.value, s)
        )
      ) {
        return; // can't move within own tree
      }
      console.debug("drop move statements batch", statement.value, nav?.value?.panel.selectedElementIds);
      await nav?.value.moveBatchTo(statementsToMove, dragInTopHalf.value ? "above" : "below", statement.value);
    } else {
      // single move
      const statementToMove = nav?.value?.statementsById[thing.id];
      if (
        thing.id == statement.value.id ||
        statementToMove == null ||
        nav?.value?.isDescendantOf(statementToMove, statement.value) ||
        nav?.value?.isDescendantOf(statement.value, statementToMove)
      ) {
        return; // can't move within own tree
      }
      const dropLocation = dragInTopHalf.value
        ? nav?.value?.getLocationRightAbove(statement.value)
        : nav?.value?.getLocationRightBelow(statement.value);
      console.debug("drop move statement", thing, statement.value.id);
      await nav?.value?.moveTo(statementToMove, dropLocation);
    }
  } else {
    throw new Error("unexpected drop");
  }
}

// actions
const canOpenInStandaloneEditor = computed(
  () => !props.standalone && STATEMENT_STANDALONE_TYPES.includes(statement.value.type)
);
const defaultActions: Ref<StatementAction[]> = computed(() => {
  const actions = [];
  if (canOpenInStandaloneEditor.value) {
    actions.push({
      groupId: "nav",
      label: "Open",
      icon: ArrowsPointingOutIcon,
      disabled: props.standalone,
      action: () => {
        nav?.value?.panel.bench.openEditStatement(statement.value, { focus: true });
      },
    });
    actions.push({
      groupId: "nav",
      label: "Open on other side",
      icon: ArrowsPointingOutIcon,
      disabled: props.standalone,
      action: () => {
        nav?.value?.panel.bench.openEditStatement(statement.value, {
          group: panel.panel.value.group,
          focus: true,
          opposite: true,
        });
      },
    });
    actions.push({
      groupId: "nav",
      label: isContentFolded.value ? "Expand" : "Collapse",
      icon: isContentFolded.value ? ChevronDownIcon : ChevronRightIcon,
      disabled: !canContentFold.value,
      action: () => toggleContentFold(false),
    });
  }
  actions.push({
    groupId: "edit",
    label: "Rename",
    icon: PencilSquareIcon,
    action: () => {
      nav?.value?.panel.editElement(statement.value);
      nextTick(() => statementRef.value?.focus());
    },
  });
  if (!props.standalone) {
    // doesn't work in standalone editor because it needs file context right now
    actions.push({
      groupId: "edit",
      label: "Duplicate",
      icon: Square2StackIcon,
      action: () => magic.duplicate(),
    });
    actions.push({
      groupId: "edit",
      label: "Delete",
      icon: TrashIcon,
      action: () => magic.delete(),
    });
  }
  return actions;
});
const allActions: Ref<StatementAction[]> = computed(() => [
  ...defaultActions.value,
  ...(context.customActions.value?.map((a) => ({ ...a, groupId: "custom" })) ?? []),
]);
const actionGroups = computed(() => [
  { id: "general" },
  { id: "custom", label: STATEMENT_TYPE_LABELS[statement.value.type] ?? "Statement" },
]);

function showActionsPopover() {
  actionPopoverRef.value?.show();
}

// runtime
const issues = module.issuesOfRef(statement);
const hasIssues = computed(() => (issues.value?.length ?? 0) > 0);
const hasErrors = computed(() => issues.value?.find((i) => i.kind == IssueKind.Error));

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    return statementRef.value?.focus(position);
  },
  blur: () => statementRef.value?.blur(),
  root: statementRef,
  bounding: containerBounding,
  loading: computed(() => statementRef.value == null || (statementRef.value?.loading ?? false)),
  showActionsPopover,
  context,
  allActions,
});
</script>
<template>
  <!-- Statement wrapper -->
  <!-- The :group/statement here is used in file editor -->
  <div
    class="group/statement relative w-full max-w-full"
    :style="panel.panel.value.contentMarginXAsPaddingX"
    @click="onClickContainer"
    @contextmenu.prevent="showActionsPopover"
  >
    <!-- Statement main -->
    <div
      ref="containerRef"
      @dragstart.stop="onDragStart"
      class="relative min-h-[30px] w-full rounded-sm outline-none transition duration-150 focus:outline-none"
      :class="{
        'focus:bg-orange-100': true,
        'bg-orange-100':
          (isFocused && !isEditing) || isSelected || isAncestorHighlight || dragOver || actionPopoverRef?.open,
        ...appearance.baseClass,
      }"
      :style="{
        marginLeft: highlightOffsetX + 'px',
        paddingLeft: contentOffsetX - highlightOffsetX + 'px',
        width: `calc(100% - ${highlightOffsetX}px)`,
      }"
    >
      <!-- Left gutter -->
      <div v-if="!standalone" class="absolute top-1">
        <!-- Small positioning hack to get content right-aligned on absolute left offset -->
        <div class="relative">
          <div class="absolute right-0 flex flex-row-reverse items-center gap-0.5">
            <!-- Actions / drag handle -->
            <ActionPopover
              ref="actionPopoverRef"
              anchor="right"
              :thing="isInSelection ? null : statement"
              :actions="isInSelection ? (nav as unknown as NavigationContext).selectionActions : allActions"
              :groups="actionGroups"
              v-slot="{ open }"
              @mouseup="containerRef?.setAttribute('draggable', 'false')"
              @click.stop
            >
              <!-- For some reason I had to put the mousedown back into the inner element for dragging to work -- previously,
            there was *some* reason not to do this (maybe old styling/padding), but it seems to work fine now. -->
              <div
                @mousedown.stop="
                  {
                    (nav as unknown as NavigationContext)?.panel?.addToSelection(statement);
                    containerRef?.setAttribute('draggable', 'true');
                  }
                "
                class="group cursor-grab p-0.5 transition duration-150"
                :class="{
                  'opacity-0 group-hover/statement:opacity-100': !isActive && !open,
                  'opacity-100': isActive,
                  'text-gray-400 hover:bg-orange-100 hover:text-gray-700': true,
                  ...appearance.baseClass,
                }"
              >
                <DragHandleIcon class="h-4 w-4" />
                <!-- Label -->
                <span
                  class="pointer-events-none absolute -left-10 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100"
                >
                  <strong>Click</strong> for actions
                  <br />
                  <strong>Drag</strong> to move
                </span>
              </div>
            </ActionPopover>
            <!-- Add statement below button -->
            <button
              v-if="!standalone && !bench.readonly && !props.readonly"
              class="group rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:opacity-100"
              :class="isActive ? 'opacity-100' : 'opacity-0'"
              @click="insertStatementOnClick"
            >
              <PlusIcon class="h-4 w-4" />
              <!-- Label -->
              <span
                class="pointer-events-none absolute -left-2 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100"
              >
                <strong>Click</strong> to insert below
                <br />
                <strong>Option-click</strong> for above
              </span>
            </button>
            <!-- Fold / unfold content -->
            <button
              v-if="!standalone && canContentFold"
              class="group rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:opacity-100"
              :class="isActive ? 'opacity-100' : 'opacity-0'"
              @click="(e) => toggleContentFold(e.altKey)"
            >
              <ChevronRightIcon
                class="h-4 w-4 transition-transform duration-150"
                :class="[isContentFolded ? '' : 'rotate-90']"
              />
              <!-- Label (yeah these should be refactored) -->
              <span
                class="pointer-events-none absolute -left-1 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100"
              >
                <strong>Click</strong> to {{ isContentFolded ? "expand" : "fold" }}
                <br />
                <strong>Option-click</strong> for all
              </span>
            </button>
          </div>
        </div>
      </div>
      <!-- Statement drag & drop indicator (top/bottom) :DragStyle -->
      <div
        v-if="!readonly && !standalone"
        class="absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="dragOver && dragInTopHalf ? 'opacity-100' : 'opacity-0'"
      />
      <div
        v-if="!readonly && !standalone"
        class="absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-300 transition duration-150"
        :class="dragOver && dragInBottomHalf ? 'opacity-100' : 'opacity-0'"
      />
      <!-- Statement interface -->
      <!-- :StatementPadding -->
      <div
        ref="innerWrapperRef"
        class="relative max-w-full px-2 py-1"
        :class="{
          'text-sm': bench.textSmall,
          'text-md': !bench.textSmall,
        }"
      >
        <component
          ref="statementRef"
          :is="statementInterface.component"
          v-bind="statementInterface.props"
          :folded="isContentFolded"
          @toggle-fold="toggleContentFold"
          @toggleActions="showActionsPopover"
        />
      </div>
      <!-- Issues in right gutter -->
      <div
        v-if="!standalone && hasIssues"
        class="group/issues absolute left-full top-[5px] flex origin-top-right select-none flex-row gap-2 px-1 not-italic"
        :class="{
          'text-md': !bench.textSmall,
          'text-sm': bench.textSmall,
        }"
      >
        <button
          class="flex rounded-sm p-0.5 font-bold text-red-600 underline-offset-4 transition duration-75 hover:bg-orange-100"
          :class="[hasIssues ? 'opacity-100' : 'opacity-0']"
          @click="bench.openActiveView('issues')"
        >
          <XCircleIcon v-if="hasErrors" class="h-5 w-5 text-red-600" />
          <ExclamationTriangleIcon v-else class="h-5 w-5 text-yellow-600" />
        </button>
        <!-- Preview on hover -->
        <div
          class="invisible absolute right-0 top-5 z-10 flex w-fit min-w-[200px] max-w-3xl flex-col gap-1 whitespace-normal rounded-sm border border-orange-900 border-opacity-[12%] bg-white p-1 shadow-sm group-hover/issues:visible"
        >
          <span
            v-for="issue in issues"
            :key="issue.id"
            class="text-xs"
            :class="{
              'text-red-600': issue.kind == IssueKind.Error,
              'text-yellow-600': issue.kind == IssueKind.Warning,
            }"
          >
            {{ issue.message }}
          </span>
        </div>
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="bench.debug" class="absolute right-2 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm">
      <span class="mr-0.5"
        >x:{{ Math.round(containerBounding.x.value) }} y:{{ Math.round(containerBounding.y.value) }}</span
      >
      <template v-if="isAncestorHighlight">h{{ ancestorHighlightDepth }}</template>
      <template v-if="isActive">a</template>
      <template v-if="isFocused">f</template>
      <template v-if="isEditing">e</template>
      <span class="ml-1">{{ isSelected ? "1" : "0" }}/{{ (nav as any)?.panel?.selectedElementIds.length }}</span>
      <template v-if="dragOver">d</template>
      <template v-if="inContainerFocused">*</template>
      <template v-if="inStatementFocused">**</template>
      <span class="mx-1 lowercase">{{ statement.type }}</span>
      <span v-if="statement.name != null">'{{ statement.name }}'</span>
      r:{{ statement.revision }} o:{{ statement.orderKey }} d:{{ depth }}
    </div>
  </div>
</template>
