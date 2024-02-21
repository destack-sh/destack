<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import AnnotatedText from "@/components/interfaces/AnnotatedText.vue";
import {
  BASIC_CONTROL_PARTS,
  STATEMENT_INTERFACES,
  STATEMENT_RUNNABLE_TYPES,
  type StatementElementId,
  type StatementEmitDict,
  type StatementInterface,
  type StatementPart,
  type StatementPartComponent,
  type StatementPartId,
} from "@/components/statements";
import DeclarationControl from "@/components/statements/DeclarationControl.vue";
import MorphStatementInterface from "@/components/statements/MorphStatementInterface.vue";
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
import { useCurrentModule, type Statement, TypeFlag, type InterpStatement } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useCurrentSessions } from "@/state/session";
import { STATEMENT_TYPE_LABELS, useFieldsState } from "@/state/statement";
import { setDragData, useRelativeDropZone } from "@/utils/drop";
import { toValueRef } from "@/utils/functools";
import {
  ArrowPathRoundedSquareIcon,
  ArrowsPointingOutIcon,
  AtSymbolIcon,
  Bars3BottomLeftIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  ChevronUpIcon,
  EllipsisHorizontalIcon,
  EllipsisVerticalIcon,
  PlusIcon,
  SparklesIcon,
  Square2StackIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { ExclamationTriangleIcon, InformationCircleIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { onClickOutside, useElementBounding, useFocusWithin, useKeyModifier, whenever } from "@vueuse/core";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{
  file: FileHeader;
  statement: Statement;
  depth: number;
  renderedDepth: number;
  ancestors: Statement[];
  readonly: boolean;
  shown?: boolean;
  isGroupStart?: boolean;
  isGroupMiddle?: boolean;
  isGroupEnd?: boolean;
}>();
const emit = defineEmits<{
  (e: "launchAssist", text: string, selection?: StatementHeader[]): void;
}>();
const file = toRef(props, "file");
const statement = toRef(props, "statement");
const ancestors = toRef(props, "ancestors");

const bench = useBenchState();
const appearance = useAppearance();
const nav: Ref<NavigationContext | null> = useNavigationContext() ?? ref(null);
const panel = usePanelContext();
const module = useCurrentModule();
const ops = useOperations();
const magic = useMagicActions(statement as Ref<StatementHeader | null>);

const containerRef = ref<HTMLElement | null>(null);
const containerBounding = useElementBounding(containerRef);
const innerWrapperRef = ref<HTMLElement | null>(null);
const { focused: inContainerFocused } = useFocusWithin(containerRef);
const { focused: inStatementFocused } = useFocusWithin(innerWrapperRef);
const isActive = toValueRef(computed(() => nav?.value?.panel.activeStatementCk == statement.value?.ck));
const isFocused = toValueRef(computed(() => isActive.value && (nav?.value?.panel.focused ?? false)));
const isEditing = toValueRef(computed(() => isFocused.value && (nav?.value?.panel.editing ?? false)));
const isSelected = toValueRef(computed(() => nav?.value?.panel.isSelected(statement.value)));
const isInSelection = toValueRef(
  computed(() => isSelected.value && (nav?.value?.panel.selectedElementIds?.length ?? 0) > 1)
);
const isAnySelection = toValueRef(computed(() => nav?.value?.panel.hasSelection));
const canContentFold = computed(
  () => statement.value.type != StatementType.Blank && statement.value.type != StatementType.Text
);
const isContentFolded = computed(() => (panel.panel.value as EditFilePanel).isStatementContentFolded(statement.value));

const pasting = ref(false);
const location = computed(() => nav?.value?.getLocation(statement.value));
const fields = useFieldsState(statement, isContentFolded);

function toggleContentFold(descendants?: boolean) {
  if (descendants) {
    (panel.panel.value as EditFilePanel).setStatementContentsFolded(
      module.getDescendantsOf(statement.value.id),
      !isContentFolded.value
    );
  } else if ((panel.panel.value as EditFilePanel).hasSelection) {
    (panel.panel.value as EditFilePanel).setStatementContentsFolded(
      nav?.value?.panel.selectedElementIds
        ?.map((id) => nav.value?.statementsById[id] as { ck: string })
        .filter((s) => s != null) ?? [],
      !isContentFolded.value
    );
  } else {
    (panel.panel.value as EditFilePanel).toggleStatementContentFolded(statement.value);
  }
}

// manage interfaces
// TODO :Performance: don't instantiate inactive statement parts components
//  We currently need to to contribute their available actions, but ideally the
//  actions and add popovers would be factored out so we don't need their instances.

const iface = computed(() => STATEMENT_INTERFACES[statement.value.type]);
const actionPopoverRef = ref<InstanceType<typeof ActionPopover>>();
const foldedFields = computed(() =>
  iface.value?.foldable?.includes("all") ? fields.allFields.value : fields.selfFields.value
);

const partsForceShown: Ref<StatementPartId[]> = ref([]);
const enabledControlParts = computed(() => {
  if (iface.value == null) return [];
  const i = iface.value as StatementInterface;
  const enabledControlParts = [...BASIC_CONTROL_PARTS.filter((p) => p.enabled(i, props.statement))];
  return enabledControlParts.map((p) => ({
    part: p,
    active:
      p.id == "declaration" ||
      partsForceShown.value.includes(p.id) ||
      p.exists(iface.value as StatementInterface, props.statement),
  }));
});
const activeControlParts = computed(() => enabledControlParts.value.filter((p) => p.active).map((p) => p.part));
const elementParts = computed(() => {
  if (iface.value == null) return [];
  return iface.value?.elements.map((p) => ({
    part: p,
    active:
      !isContentFolded.value &&
      (p.showIfNotExists ||
        partsForceShown.value.includes(p.id) ||
        p.exists(iface.value as StatementInterface, props.statement)),
  }));
});
const activeElementParts = computed(() => elementParts.value.filter((p) => p.active).map((p) => p.part));

const partsRefs: Ref<Record<string, StatementPartComponent>> = ref({});

function handleStatementPartEvents(kind: "control" | "element", partId: StatementPartId): StatementEmitDict {
  return {
    navigateUp: () => navigate("up", partId),
    navigateDown: () => navigate("down", partId),
    navigateLeft: () => navigate("left", partId),
    navigateRight: () => navigate("right", partId),
    enterLeft: () => {
      if (kind == "element" && !(partId == "text" && getPartsInOrder().partsInOrder.length == 1)) {
        magic.insertBelow(true);
      } else {
        magic.insertAbove();
      }
    },
    enter: () => magic.insertBelow(true),
    enterRight: () => magic.insertBelow(true),
    paste: () => nav.value?.paste(),
    run,
    deleteLeft: () => {
      if (
        partId == "text" &&
        canHaveText.value &&
        (statement.value.type != StatementType.Text || statement.value.name != null)
      ) {
        // if part has text and can do without, remove the text
        partsForceShown.value = partsForceShown.value.filter((p) => p != "text");
        nextTick(() => focus("declaration"));
      } else {
        // otherwise delete proper
        const above = nav?.value?.getAbove(statement.value);
        ops.statement.softDelete(null, statement.value.id);
        nextTick(() => nav?.value?.statementsComponents[above?.id ?? ""]?.focus("last"));
      }
    },
    deleteSelf: () => {
      ops.statement.softDelete(null, statement.value.id);
    },
    focus: (partId: StatementPartId) => focus(partId),
    hide: () => {
      partsForceShown.value = partsForceShown.value.filter((p) => p != partId);
    },
    escape: () => (panel.panel.value as EditFilePanel).stopEditingElement(statement.value),
    openActions: showActionsPopover,
    launchAssist: (text: string) => emit("launchAssist", text, [statement.value]),
    illegal: (char: string) => {
      if (char == "#" && partId == "declaration") {
        actions.value.find((a) => a.label == "Add tag")?.action(statement.value);
      }
    },
  };
}

function getPartsInOrder(options?: { includeInactive?: boolean; excludeNotFocusable?: boolean }) {
  const partsInOrder: StatementPart[] = [
    ...(options?.includeInactive ? enabledControlParts.value.map((p) => p.part) : activeControlParts.value),
    ...(options?.includeInactive ? elementParts.value.map((p) => p.part) : activeElementParts.value),
  ].filter((p) => !options?.excludeNotFocusable || !p.notFocusable);
  // controls in one 'row', elements have their own rows
  const partsRows: StatementPart[][] = [
    [...activeControlParts.value, ...(iface.value?.extraControls ?? [])].filter(
      (p) => !options?.excludeNotFocusable || !p.notFocusable
    ),
    ...(activeElementParts.value ?? [])
      .map((e) => [e])
      .filter((p) => !options?.excludeNotFocusable || !p[0].notFocusable),
  ].filter((row) => row.length > 0);
  return { partsInOrder, partsRows };
}

function navigate(direction: "left" | "up" | "right" | "down", partId: string) {
  const { partsRows } = getPartsInOrder({ excludeNotFocusable: true });
  const y = partsRows.findIndex((row) => row.find((p) => p.id == partId));
  if (y == null || y < 0) {
    console.warn("statement part not found", partId, partsRows);
    return;
  }
  const x = partsRows[y].findIndex((p) => p.id == partId);
  if (x == null || x < 0) return;

  // navigate inside statement parts if possible, otherwise navigate in file
  if (direction == "left" && x == 0) direction = "up";
  if (direction == "right" && x >= partsRows[y].length - 1) direction = "down";
  if (direction == "up") {
    if (y > 0) {
      const nextPart = partsRows[y - 1][Math.min(x, partsRows[y - 1].length - 1)];
      partsRefs.value[nextPart.id]?.focus("last");
    } else {
      const above = nav?.value?.getAbove(statement.value);
      if (above != null) nav?.value?.statementsComponents[above.id]?.focus("last");
      else nav?.value?.navigateUp();
    }
  } else if (direction == "down") {
    if (y < partsRows.length - 1) {
      const nextPart = partsRows[y + 1][Math.min(x, partsRows[y + 1].length - 1)];
      partsRefs.value[nextPart.id]?.focus("first");
    } else {
      const below = nav?.value?.getBelow(statement.value);
      if (below != null) nav?.value?.statementsComponents[below.id]?.focus("first");
      else if (props.statement.type != StatementType.Blank) {
        // insert new statement below
        magic.insertBelow(true);
      }
    }
  } else if (direction == "left") {
    const nextPart = partsRows[y][x - 1];
    partsRefs.value[nextPart.id]?.focus("last");
  } else if (direction == "right") {
    const nextPart = partsRows[y][x + 1];
    partsRefs.value[nextPart.id]?.focus("first");
  }
}

function focus(focus: "first" | "last" | StatementPartId = "first") {
  const { partsInOrder } = getPartsInOrder({ excludeNotFocusable: true });
  if (partsInOrder.length == 0) {
    console.warn("statement has no parts to focus", props.statement, focus, iface.value, partsInOrder);
  } else if (focus == "first") {
    partsRefs.value[partsInOrder[0].id]?.focus("first");
  } else if (focus == "last") {
    partsRefs.value[partsInOrder[partsInOrder.length - 1].id]?.focus("last");
  } else {
    partsRefs.value[focus]?.focus("first");
    if (partsRefs.value[focus] == null) console.warn("part to focus not found", props.statement, focus, partsInOrder);
  }
  scrollIntoViewIfNeeded();
}

function focusIfUnfocused(f: "first" | "last" | StatementPartId = "first") {
  if (!inContainerFocused.value) {
    focus(f);
  }
}

function blur() {
  Object.values(partsRefs.value).forEach((e) => e?.blur?.());
}

// update container bounding whenever location changes (since ResizeObserver doesn't seem to be triggered in that case)
watch(location, () => {
  containerBounding.update();
});

// scroll into view when becoming active
whenever(isActive, () => {
  if (isEditing.value) return; //  (but not editing, which would focus an actual HTML element)
  scrollIntoViewIfNeeded();
});

function scrollIntoViewIfNeeded() {
  const editorRect = panel.container.value?.getBoundingClientRect();
  const containerRect = containerRef.value?.getBoundingClientRect();
  if (editorRect != null && containerRect != null) {
    if (containerRect.top < editorRect.top + appearance.panelHeaderHeight || containerRect.bottom > editorRect.bottom) {
      // TODO @UX: improve scroll behavior (feels a bit janky sometimes)
      containerRef.value?.scrollIntoView(false);
    }
  }
}

// refocus if statement interface changed and we're editing
watch(
  () => iface.value,
  (a, b) => {
    if (isEditing.value && a?.type !== b?.type) {
      nextTick(focus);
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
          blur();
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
    nav?.value?.panel.editElement(statement.value, false);
  }
});

function focusInEditor() {
  bench.focusFile(file.value as any);
  nav?.value?.panel.focusElement(statement.value);
}

function onClickContainer(e: MouseEvent) {
  // create selection to here if shift was pressed
  if (e.shiftKey && nav != null) {
    const index = nav?.value?.statementPositions[statement.value.id];
    const lastIndex = nav?.value?.statementPositions[nav?.value?.panel.activeStatementCk ?? ""];
    console.log("select all statements between", index, lastIndex);
    focusInEditor();
    if (index != null && lastIndex != null) {
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

// drag & drop
const capturingDrag = computed(() => Object.values(partsRefs.value).find((e) => e?.capturingDrag === true));
const {
  isOverDropZone: dragOver,
  inTopHalf: dragInTopHalf,
  inBottomHalf: dragInBottomHalf,
} = useRelativeDropZone(
  containerRef,
  ["Statement", "BrowserFile"],
  (thing) => onDrop(thing),
  computed(() => !capturingDrag.value && !props.readonly)
);

function onDragStart(e: DragEvent) {
  if (innerWrapperRef.value == null) return;
  setDragData(e, { type: "Statement", id: statement.value.id });
  // TODO :Broken @UX: drag image looks horrible sometimes
  // (when statements have large hidden content like Code (Monaco infinite lines view) or Database (horizontal overscroll area))
  e.dataTransfer?.setDragImage(innerWrapperRef.value, 0, 0);
}

async function onDrop(thing: any[] | { type: string; id: string } | null) {
  if (thing == null || nav == null || nav.value == null) return;
  if (Array.isArray(thing)) {
    console.log("drop insert files into new statement", thing);
    // not supported right now until we get transactions
  } else if (thing?.type == "Statement") {
    if (nav?.value?.panel.hasSelection && nav?.value.panel.selectedElementIds.length > 1) {
      // batch move
      const statementsToMove = nav.value.panel.selectedElementIds.map(
        (id) => nav?.value?.statementsById[id] as StatementHeader
      );
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
        ? nav.value.getLocationRightAbove(statement.value)
        : nav.value.getLocationRightBelow(statement.value);
      console.debug("drop move statement", thing, statement.value.id);
      await nav?.value?.moveTo(statementToMove, dropLocation);
    }
  } else {
    throw new Error("unexpected drop");
  }
}

// highlighting
const INDENT_OFFSET_X = 28;
// ancestor is considered highlighted if it's focused or selected (need to expand highlight to their depth)
// (this is a bit ugly but will also disappear with :NestedStatementRendering)
const ancestorHighlightDepth = computed(() => {
  const highlightedAncestorIdx = ancestors.value.findIndex(
    (s) => nav?.value?.panel.activeStatementCk == s.ck || nav?.value?.panel.selectedElementIds.includes(s.id)
  );
  if (highlightedAncestorIdx > -1) {
    // subtract any group ancestors
    return (
      highlightedAncestorIdx -
      ancestors.value.slice(0, highlightedAncestorIdx).filter((s) => s.type == StatementType.Group).length
    );
  }
  return -1;
});
const isAncestorHighlight = computed(() => !nav?.value?.panel.editing && ancestorHighlightDepth.value > -1);
const contentOffsetX = computed(() => props.renderedDepth * INDENT_OFFSET_X);
const highlightOffsetX = computed(() =>
  isAncestorHighlight.value ? ancestorHighlightDepth.value * INDENT_OFFSET_X : contentOffsetX.value
);
const isHighlighted = toValueRef(
  computed(
    () =>
      (isFocused.value && !isEditing.value && !isAnySelection.value) ||
      isSelected.value ||
      isAncestorHighlight.value ||
      dragOver.value ||
      actionPopoverRef.value?.open
  )
);

// actions
const canHaveText = computed(() => iface.value?.elements.some((e) => e.id == "text"));
const defaultActions: Ref<StatementAction[]> = computed(() => {
  const actions = [];
  if (canContentFold.value) {
    actions.push({
      groupId: "nav",
      label: isContentFolded.value ? "Expand" : "Collapse",
      icon: isContentFolded.value ? ChevronDownIcon : ChevronRightIcon,
      disabled: !canContentFold.value,
      hideInline: true,
      hideInMenu: true,
      action: () => toggleContentFold(false),
    });
  }
  if (canHaveText.value && (props.statement.text ?? "").length == 0 && !partsForceShown.value.includes("text")) {
    actions.push({
      groupId: "edit",
      label: "Add text",
      icon: Bars3BottomLeftIcon,
      hideInline: false,
      disabled: props.readonly,
      action: () => {
        if (isContentFolded.value) toggleContentFold(false);
        partsForceShown.value.push("text");
        nextTick(() => focus("text"));
      },
    });
  }
  if (props.statement.type == StatementType.Text && props.statement.name == null) {
    actions.push({
      groupId: "edit",
      label: "Add name",
      icon: AtSymbolIcon,
      hideInline: false,
      disabled: props.readonly,
      action: () => {
        ops.statement.rename(null, props.statement.id, null, "");
        nextTick(() => focus("declaration"));
      },
    });
  }
  actions.push({
    groupId: "edit-core",
    label: "Turn into",
    icon: ArrowPathRoundedSquareIcon,
    hideInline: true,
    disabled: props.readonly,
    action: () => {
      /* noop */
    },
    component: () => ({ component: MorphStatementInterface, props: { statement: props.statement } }),
  });
  actions.push({
    groupId: "edit-core",
    label: "Insert above",
    icon: ChevronDoubleUpIcon,
    hideInline: true,
    disabled: props.readonly,
    action: () => magic.insertAbove(),
  });
  actions.push({
    groupId: "edit-core",
    label: "Insert below",
    icon: ChevronDoubleDownIcon,
    hideInline: true,
    disabled: props.readonly,
    action: () => magic.insertBelow(),
  });
  actions.push({
    groupId: "edit-core",
    label: "Copy",
    icon: Square2StackIcon,
    disabled: props.readonly,
    action: () => magic.copy(),
  });
  actions.push({
    groupId: "edit-core",
    label: "Duplicate",
    icon: Square2StackIcon,
    disabled: props.readonly,
    action: () => magic.duplicate(),
  });
  actions.push({
    groupId: "edit-core",
    label: "Assist",
    icon: SparklesIcon,
    action: () => {
      emit("launchAssist", "", [props.statement]);
    },
  });
  actions.push({
    groupId: "edit-core",
    label: "Delete",
    hideInline: true,
    disabled: props.readonly,
    icon: TrashIcon,
    action: () => magic.delete(),
  });
  return actions;
});
const actions: Ref<StatementAction[]> = computed(() => {
  const actions: StatementAction[] = [...defaultActions.value];
  // collect actions and ensure that we're unfolded & the source element is always shown (even if currently hidden)
  const { partsInOrder } = getPartsInOrder({ includeInactive: true });
  for (const part of partsInOrder.reverse()) {
    const partComponent = partsRefs.value[part.id];
    if (partComponent?.actions == null) continue;

    // wrap the underlying action
    partComponent.actions.forEach((a: StatementAction) => {
      actions.push({
        ...a,
        action: (thing: StatementHeader) => {
          if (!partsForceShown.value.includes(part.id)) {
            partsForceShown.value.push(part.id);
          }
          if (isContentFolded.value) {
            toggleContentFold(false);
          }
          a.action(thing);
        },
      });
    });
  }
  return actions;
});
function orderActions(actions: StatementAction[], order: string[]) {
  return actions.slice().sort((a, b) => order.indexOf(a.groupId ?? "other") - order.indexOf(b.groupId ?? "other"));
}
const actionsInlineOrder = computed(() => orderActions(actions.value, ["edit", "nav", "other", "edit-core", "misc"]));
const actionsPopoverOrder = computed(() => {
  if (isInSelection.value) {
    // add assist to selection actions
    return [
      {
        groupId: "edit-core",
        label: "Assist",
        icon: SparklesIcon,
        action: () => {
          emit("launchAssist", "", nav?.value?.getSelectedRoots() ?? [statement.value]);
        },
      },
      ...(nav?.value?.selectionActions ?? []),
    ];
  } else {
    return orderActions(actions.value, ["run", "edit", "edit-core", "nav", "other", "misc"]);
  }
});

function showActionsPopover() {
  actionPopoverRef.value?.show();
}

// sessions

const sessions = useCurrentSessions();
function run() {
  if (!STATEMENT_RUNNABLE_TYPES.includes(statement.value.type)) {
    console.warn(`can't run ${statement.value} of type ${statement.value.type}`);
    return;
  }
  if (!bench.canUse) {
    console.warn(`can't run ${statement.value} with access level ${bench.accessLevel}`);
    return;
  }
  // force sync
  Object.values(partsRefs.value).forEach((p) => p?.syncNow?.());

  // actually run / launch
  if (statement.value.fields.some((f) => f.deletedAt == null && !(f.flags & TypeFlag.IS_OUTPUT))) {
    bench.openLaunchRun(statement.value, { group: panel.panel.value.group, focus: true, opposite: true });
  } else {
    sessions.run(props.statement);
    // show run element
    if (!partsForceShown.value.includes("run")) {
      partsForceShown.value.push("run");
    }
  }
}

// interp
const issues = module.issuesOfRef(statement);
const hasIssues = computed(() => (issues.value?.length ?? 0) > 0);
const hasErrors = computed(() => issues.value?.find((i) => i.kind == IssueKind.Error));
const hasWarnings = computed(() => issues.value?.find((i) => i.kind == IssueKind.Warning));

defineExpose({
  focus,
  focusIfUnfocused,
  blur,
  bounding: containerBounding,
  loading: computed(() => elementParts.value.some((p) => p.active && partsRefs.value[p.part.id]?.loading === true)),
  actions,
  showActionsPopover,
  run,
  pasting,
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
    <!-- TODO :Performance!: fix statement redraw on every statement focus change -->
    <!-- {{ console.log("redraw statement", props.statement.type, props.statement.ck) }} -->
    <!-- TODO @UX: group border wrapper (ugly, stupid, too many wrappers... see :NestedStatementRendering) -->
    <!--  (can't put this in outer wrapper because of padding, same with main inner wrapper right below) -->
    <div
      :class="{
        'border-x': isGroupMiddle || isGroupStart || isGroupEnd,
        'border-orange-900/[12%]': !isHighlighted,
        'border-orange-300': isHighlighted,
        'rounded-t-sm border-t': isGroupStart,
        'rounded-b-sm border-b': isGroupEnd,
      }"
    >
      <!-- Statement main -->
      <div
        ref="containerRef"
        @dragstart.stop="onDragStart"
        class="relative min-h-[30px] w-full rounded-sm outline-none transition duration-150 focus:outline-none"
        :class="{
          'focus:bg-orange-100': true,
          'bg-orange-100': isHighlighted || pasting,
          'animate-pulse': pasting,
          ...appearance.baseClass,
        }"
        :style="{
          marginLeft: highlightOffsetX + 'px',
          paddingLeft: contentOffsetX - highlightOffsetX + 'px',
          width: `calc(100% - ${highlightOffsetX}px)`,
        }"
      >
        <!-- Left gutter -->
        <div class="absolute top-1">
          <!-- Small positioning hack to get content right-aligned on absolute left offset -->
          <div class="relative">
            <div class="absolute right-0 flex flex-row-reverse items-center gap-0.5">
              <!-- Actions / drag handle -->
              <ActionPopover
                ref="actionPopoverRef"
                anchor="right"
                :thing="isInSelection ? null : statement"
                :actions="actionsPopoverOrder"
                v-slot="{ open }"
                :allow-freeform="bench.canEdit"
                @freeform="emit('launchAssist', $event, nav?.getSelectedRoots() ?? [statement])"
                @click.stop
                @close="
                  /* TODO @UX: this doesn't always work, also see other ActionPopover */
                  $nextTick(() => focusIfUnfocused('first'))
                "
                @mouseup="containerRef?.setAttribute('draggable', 'false')"
              >
                <!-- For some reason I had to put the mousedown back into the inner element for dragging to work -- previously,
            there was *some* reason not to do this (maybe old styling/padding), but it seems to work fine now. -->
                <div
                  @mousedown.stop="
                    {
                      nav?.panel?.addToSelection(statement);
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
                  <EllipsisVerticalIcon class="h-4 w-4" />
                  <!-- Label -->
                  <span
                    class="pointer-events-none absolute left-0 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100 group-hover:delay-in-500"
                  >
                    <strong>Click</strong> for actions
                    <br />
                    <strong>Drag</strong> to move
                  </span>
                </div>
              </ActionPopover>
            </div>
          </div>
        </div>
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
        <!-- Statement interface -->
        <!-- :StatementPadding -->
        <div
          ref="innerWrapperRef"
          class="relative flex max-w-full flex-col gap-y-0.5 px-2 py-1"
          :class="{
            'text-sm': bench.textSmall,
            'text-md': !bench.textSmall,
          }"
        >
          <!-- Header -->
          <div
            v-if="iface?.needsDeclaration || activeControlParts.length > 0 || statement.name != null"
            class="flex w-full flex-row justify-between"
          >
            <!-- Declaration or title (if text with heading) -->
            <div class="flex flex-row flex-wrap gap-y-1">
              <DeclarationControl
                v-if="iface?.needsDeclaration || (statement.type == StatementType.Text && statement.name != null)"
                :ref="(ref: any) => (partsRefs['declaration'] = ref)"
                :statement="statement"
                :readonly="readonly"
                v-on="handleStatementPartEvents('control', 'declaration')"
                class="mr-1.5"
              />
              <!-- Controls inline -->
              <component
                v-for="{ part: control, active } in enabledControlParts.filter((c) => c.part.id != 'declaration')"
                :ref="(ref: any) => (partsRefs[control.id] = ref)"
                :key="control.id"
                :is="control.component"
                :statement="statement"
                :focused="isFocused"
                :selected="isSelected"
                :editing="isEditing"
                :readonly="readonly"
                v-on="handleStatementPartEvents('control', control.id)"
                :class="[active ? 'mr-1.5' : '']"
              />
            </div>
            <!-- Actions -->
            <div
              class="relative flex flex-shrink-0 gap-x-0.5 self-start transition-opacity duration-150"
              :class="[isFocused ? 'opacity-100' : 'opacity-0 group-hover/statement:opacity-100']"
            >
              <!-- Extra controls -->
              <component
                v-for="{ id, component } in iface?.extraControls ?? []"
                :ref="(ref: any) => (partsRefs[id] = ref)"
                :key="id"
                :is="component"
                :statement="statement"
                :focused="isFocused"
                :selected="isSelected"
                :editing="isEditing"
                :readonly="readonly"
              />
              <!-- Extra space between extra controls and actions -->
              <span />
              <!-- Actions -->
              <button
                v-for="action in actionsInlineOrder.filter((action) => !action.hideInline && !action.disabled)"
                :key="action.label"
                class="group relative p-0.5 text-gray-400 hover:text-gray-700"
                :class="action.active ? 'animate-spin cursor-not-allowed' : 'hover:bg-orange-100'"
                @click.prevent.stop="action.action(statement)"
                :disabled="action.disabled || action.active"
              >
                <component :is="action.active ? BusySpinnerIcon : action.icon" class="h-4 w-4" />
                <!-- Label popover -->
                <span
                  v-if="!action.active"
                  class="pointer-events-none absolute -left-3 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition delay-in-500 duration-150 group-hover:opacity-100"
                >
                  {{ action.label }}
                </span>
              </button>
              <!-- All actions popover (same as on other side for convenience) -->
              <ActionPopover
                anchor="right"
                :thing="isInSelection ? null : statement"
                :actions="actionsPopoverOrder"
                :allow-freeform="bench.canEdit"
                @freeform="emit('launchAssist', $event, nav?.getSelectedRoots() ?? [statement])"
                @click.stop
                @close="$nextTick(() => focus('first'))"
              >
                <div class="group p-0.5 text-gray-400 hover:text-gray-700">
                  <EllipsisVerticalIcon class="h-4 w-4" />
                  <span
                    class="pointer-events-none absolute -right-2 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition delay-in-500 duration-150 group-hover:opacity-100"
                  >
                    More actions
                  </span>
                </div>
              </ActionPopover>
            </div>
          </div>
          <!-- Body -->
          <!-- Folded -->
          <template v-if="isContentFolded && iface?.foldable != null">
            <button
              class="flex min-h-[20px] max-w-full flex-row gap-1.5 truncate rounded-sm hover:bg-gray-100"
              @click="toggleContentFold()"
            >
              <span v-for="field in foldedFields" class="text-gray-400" :key="field.id">
                {{ field.name }}
              </span>
              <template v-if="canHaveText && statement.text != null && statement.text.length > 0">
                <span class="text-gray-400" v-if="foldedFields.length > 0">•</span>
                <AnnotatedText :model-value="statement.text" minimal-mentions readonly class="truncate text-gray-400" />
              </template>
              <EllipsisHorizontalIcon class="h-4 w-4 self-center text-gray-400" />
            </button>
          </template>
          <!-- Actual body -->
          <!-- Missing statement interface -->
          <div v-if="iface == null" class="w-full font-bold text-red-600">
            {{ statement.type }}
          </div>
          <!-- Body elements -->
          <component
            v-for="{ part: element, active } in elementParts"
            v-show="active"
            :ref="(ref: any) => (partsRefs[element.id] = ref)"
            :key="element.id"
            :is="element.component"
            :statement="statement"
            :focused="isFocused"
            :selected="isSelected"
            :editing="isEditing"
            :readonly="readonly"
            :visible="active"
            :bounding="containerBounding"
            :xoffset="contentOffsetX"
            :class="[
              // push run element up so it's directly below code, push database/value down else it looks cramped
              element.id == 'run' ? '-mt-0.5' : '',
              element.id == 'database' || element.id == 'value' ? 'mt-0.5' : '',
            ]"
            v-on="handleStatementPartEvents('element', element.id)"
          />
          <!-- Fold / unfold elements -->
          <button
            v-if="canContentFold"
            class="group absolute -left-5 top-7 rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-hover/statement:opacity-100"
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
        <!-- Issues in right gutter -->
        <div
          v-if="hasIssues"
          class="group/issues absolute left-full top-[3px] flex origin-top-right select-none flex-row gap-2 px-1 not-italic"
          :class="{
            'text-md': !bench.textSmall,
            'text-sm': bench.textSmall,
          }"
        >
          <button
            class="flex rounded-sm p-1 font-bold underline-offset-4 transition duration-75 hover:bg-orange-100"
            @click="bench.openActiveView('issues')"
          >
            <XCircleIcon v-if="hasErrors" class="h-4 w-4 text-red-600" />
            <ExclamationTriangleIcon v-else-if="hasWarnings" class="h-4 w-4 text-yellow-600" />
            <InformationCircleIcon v-else class="h-4 w-4 text-cyan-600" />
          </button>
          <!-- Preview on hover -->
          <div
            class="invisible absolute right-0 top-5 z-10 flex w-fit min-w-[200px] max-w-3xl flex-col gap-1 whitespace-normal rounded-sm border border-orange-900/[12%] bg-white p-1 shadow-sm group-hover/issues:visible"
          >
            <span
              v-for="issue in issues"
              :key="issue.id"
              class="text-xs"
              :class="{
                'text-red-600': issue.kind == IssueKind.Error,
                'text-yellow-600': issue.kind == IssueKind.Warning,
                'text-cyan-600': issue.kind == IssueKind.Notice,
              }"
            >
              {{ issue.message }}
            </span>
          </div>
        </div>
      </div>
    </div>
    <!-- Debug info -->
    <div v-if="bench.debug" class="absolute right-2 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-xs">
      <span class="mr-0.5"
        >x:{{ Math.round(containerBounding.x.value) }} y:{{ Math.round(containerBounding.y.value) }}</span
      >
      <span>{{ statement?.ck.slice(0, 5) }}/{{ statement?.id.slice(-6, -1) }}</span>
      <span class="mx-0.5">{{ statement.type.toLocaleLowerCase() }}</span>
      <template v-if="!statement.versioned">global</template>
      <span class="mx-0.5">{{ activeControlParts.length }}c {{ activeElementParts.length }}e</span>
      <template v-if="isAncestorHighlight">h{{ ancestorHighlightDepth }}</template>
      <template v-if="isActive">a</template>
      <template v-if="isFocused">f</template>
      <template v-if="isEditing">e</template>
      <span class="ml-1" v-if="nav?.panel?.hasSelection"
        >{{ isSelected ? "1" : "0" }}/{{ nav?.panel?.selectedElementIds.length }}</span
      >
      <template v-if="dragOver">d</template>
      <template v-if="inContainerFocused">*</template>
      <template v-if="inStatementFocused">**</template>
      r:{{ statement.revision }} o:{{ statement.orderKey }} d:{{ depth }}
    </div>
  </div>
</template>
