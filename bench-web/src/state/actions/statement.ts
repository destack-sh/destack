import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type StatementHeader } from "@/state/bench";
import { activeFileState, navigationContexts, type NavigationContext } from "@/state/file";
import { newNodeIdentity } from "@/state/module";
import { useOperations } from "@/state/operations";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { createSharedComposable } from "@vueuse/shared";
import { computed, nextTick, type Ref } from "vue";

export function useStatementActions() {
  return hostStatementActions();
}

export const hostStatementActions = createSharedComposable(_provideStatementActions);

function _provideStatementActions() {
  const activeFileContext = computed(() => navigationContexts.value[activeFileState.value?.file.id ?? ""]);
  return _doProvideStatementActions(activeFileContext);
}

function _doProvideStatementActions(file: Ref<NavigationContext | null>) {
  const bench = useBenchState();
  const ops = useOperations();

  const statements = computed(() => file.value?.statements ?? []);
  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => file.value?.statementsById ?? {});
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(
    () => file.value?.statementsByParentId ?? {}
  );
  const cur = computed(() => file.value?.current);
  const panel = computed(() => file.value?.panel);
  const navigatingFile = computed(() => !file.value?.editing && !panel.value?.editing && bench.focusedViewId == null);

  // move focus
  const moveFocusUp = provideGlobalAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      panel.value?.clearSelection();
      if (cur.value?.above != null) {
        const above = cur.value.above;
        panel.value?.focusElement(above, true);
        if (panel.value?.editing) {
          file.value?.statementsComponents[above.id]?.focus("last");
        }
      } else if (cur.value?.statement == null && statements.value.length > 0) {
        // nothing focused, focus last statement
        const last = statements.value[statements.value.length - 1];
        panel.value?.focusElement(last, true);
      } else if (cur.value?.statement != null) {
        // navigate up from statements
        file.value?.navigateUp();
      }
    },
  });
  const moveFocusDown = provideGlobalAction({
    id: "statement.moveFocusDown",
    label: "Move focus down",
    enabled: computed(() => navigatingFile.value),
    shortcuts: ["down"],
    apply: () => {
      panel.value?.clearSelection();
      if (cur.value?.below != null) {
        panel.value?.focusElement(cur.value?.below, true);
      } else if (cur.value?.statement == null && statements.value.length > 0) {
        // nothing focused, focus first statement
        if (panel.value?.hasSelection) {
          const selectedRoots = file.value?.getSelectedRoots();
          panel.value?.focusElement(selectedRoots?.[0] as StatementHeader);
        } else {
          panel.value?.focusElement(statements.value[0], true);
        }
      } else if (cur.value?.statement != null) {
        // navigate down from statements
        file.value?.navigateDown();
      }
    },
  });
  // move focus in/out
  const moveFocusIn = provideGlobalAction({
    id: "statement.moveFocusIn",
    label: "Move focus in",
    shortcuts: ["right"],
    enabled: computed(() => navigatingFile.value && cur.value?.statement != null && cur.value?.children.length > 0),
    apply: () => {
      panel.value?.clearSelection();
      panel.value?.focusElement(cur.value?.children[0] as StatementHeader, true);
    },
  });
  const moveFocusOut = provideGlobalAction({
    id: "statement.moveFocusOut",
    label: "Move focus out",
    shortcuts: ["left"],
    enabled: computed(
      () => navigatingFile.value && cur.value?.statement != null && cur.value?.statement.parent != null
    ),
    apply: () => {
      panel.value?.clearSelection();
      const parent = statementsById.value[cur.value?.statement?.parent?.id];
      panel.value?.focusElement(parent, true);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideGlobalAction({
    id: "statement.editCurrent",
    label: "Edit statement",
    shortcuts: ["enter"],
    enabled: computed(() => cur.value?.statement != null && navigatingFile.value),
    apply: () => {
      panel.value?.editElement(cur.value?.statement as StatementHeader);
    },
  });
  const stopEditing = provideGlobalAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing statement",
    shortcuts: ["escape"],
    enabled: computed(() => cur.value?.statement != null && panel.value?.editing && !panel.value?.hasSelection),
    apply: () => {
      panel.value?.stopEditingElement(cur.value?.statement as StatementHeader);
    },
  });
  const cancelFocus = provideGlobalAction({
    id: "statement.cancelFocus",
    label: "Cancel focus",
    shortcuts: ["escape"],
    enabled: computed(() => !panel.value?.editing && panel.value?.activeStatementCk != null),
    apply: () => {
      panel.value?.blurElement();
    },
  });

  // indent statement
  const indent = provideGlobalAction({
    id: "statement.indentCurrent",
    label: "Indent statement",
    shortcuts: ["tab"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null && !panel.value?.hasSelection),
    // we can only indent if there is a sibling above
    apply: async () => {
      await file.value?.indent(cur.value?.statement as StatementHeader);
    },
  });
  const unindent = provideGlobalAction({
    id: "statement.unindentCurrent",
    label: "Unindent statement",
    shortcuts: ["shift+tab"],
    enabled: computed(
      () => cur.value?.statement != null && cur.value?.statement.parent != null && !panel.value?.hasSelection
    ),
    apply: async () => {
      await file.value?.unindent(cur.value?.statement as StatementHeader);
    },
  });
  const indentSelection = provideGlobalAction({
    id: "statement.indentSelection",
    label: "Indent selection",
    shortcuts: ["tab"],
    enabled: computed(() => panel.value?.hasSelection),
    apply: async () => {
      // indent selected roots in line with the topmost selected statement
      const selectedRoots = file.value?.getSelectedRoots();
      await file.value?.indentBatch(selectedRoots ?? []);
    },
  });
  const unindentSelection = provideGlobalAction({
    id: "statement.unindentSelection",
    label: "Unindent selection",
    shortcuts: ["shift+tab"],
    enabled: computed(() => panel.value?.hasSelection),
    apply: async () => {
      // unindent selected roots in line with the topmost selected statement
      const selectedRoots = file.value?.getSelectedRoots();
      await file.value?.unindentBatch(selectedRoots ?? []);
    },
  });

  // move statement up/down
  const moveCurrentUp = provideGlobalAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(
      () => cur.value?.statement != null && file.value?.current.above != null && !panel.value?.hasSelection
    ),
    apply: async () => {
      await file.value?.moveUp(cur.value?.statement as StatementHeader);
    },
  });
  const moveCurrentDown = provideGlobalAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(
      () => cur.value?.statement != null && file.value?.current.belowGroup != null && !panel.value?.hasSelection
    ),
    apply: async () => {
      await file.value?.moveDown(cur.value?.statement as StatementHeader);
    },
  });
  const moveSelectionUp = provideGlobalAction({
    id: "statement.moveSelectionUp",
    label: "Move selection up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => panel.value?.hasSelection),
    apply: async () => {
      await file.value?.moveBatchUp(file.value?.getSelectedRoots() ?? []);
    },
  });
  const moveSelectionDown = provideGlobalAction({
    id: "statement.moveSelectionDown",
    label: "Move selection down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => panel.value?.hasSelection),
    apply: async () => {
      await file.value?.moveBatchDown(file.value?.getSelectedRoots() ?? []);
    },
  });

  // manage selection
  const expandSelectionUp = provideGlobalAction({
    id: "statement.expandSelectionUp",
    label: "Expand selection up",
    shortcuts: ["shift+up"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null),
    apply: () => {
      // remove self from selection if previous selected (last is current) is above
      // (that means we're expanding down)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == panel.value?.previousSelectedStatementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition < (cur.value?.position as number)) {
        panel.value?.removeFromSelection(cur.value?.statement as StatementHeader);
      } else {
        panel.value?.addToSelection(cur.value?.statement as StatementHeader);
        panel.value?.addToSelection(cur.value?.aboveGroup as StatementHeader);
      }
      // then move focus up to previous sibling or parent
      // (if above != null, then aboveGroup must also be non null)
      panel.value?.focusElement(cur.value?.aboveGroup as StatementHeader, false);
    },
  });
  const expandSelectionDown = provideGlobalAction({
    id: "statement.expandSelectionDown",
    label: "Expand selection down",
    shortcuts: ["shift+down"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.belowGroup != null),
    apply: () => {
      // remove self from selection if previous selected (last is current) is below
      // (that means we're expanding up)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == panel.value?.previousSelectedStatementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition > (cur.value?.position as number)) {
        panel.value?.removeFromSelection(cur.value?.statement as StatementHeader);
      } else {
        panel.value?.addToSelection(cur.value?.statement as StatementHeader);
        panel.value?.addToSelection(cur.value?.belowGroup as StatementHeader);
      }
      // then move focus down
      panel.value?.focusElement(cur.value?.belowGroup as StatementHeader, false);
    },
  });
  const cancelSelection = provideGlobalAction({
    id: "statement.cancelSelection",
    label: "Cancel selection",
    shortcuts: ["escape"],
    enabled: computed(() => panel.value?.hasSelection),
    apply: () => {
      panel.value?.clearSelection();
    },
  });
  const selectAll = provideGlobalAction({
    id: "statement.selectAll",
    label: "Select all",
    shortcuts: ["ctrl+a", "meta+a"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      if (panel.value == null) return;
      statements.value.forEach((s) => panel.value?.addToSelection(s));
    },
  });

  // delete statement
  const delete_ = provideGlobalAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => cur.value?.statement != null && navigatingFile.value && !panel.value?.hasSelection),
    apply: async () => {
      const current = cur.value?.statement?.id;
      if (current == null) return;
      if (cur.value?.above) {
        panel.value?.focusElement(cur.value?.above);
      }
      await ops.statement.softDelete(null, current);
    },
  });
  const deleteSelection = provideGlobalAction({
    id: "statement.deleteSelection",
    label: "Delete selected statements",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => panel.value?.hasSelection && navigatingFile.value),
    apply: async () => {
      // after delete focus next statement above
      if (panel.value == null) return;
      panel.value?.blurElement();
      const selectedRoots = file.value?.getSelectedRoots();
      await ops.statement.batchSoftDelete(selectedRoots?.map((s) => s?.id) ?? []);
    },
  });

  const deleteCurrentLeft = provideGlobalAction({
    id: "statement.deleteCurrentLeft",
    label: "Delete current statement and move to end of above statement",
    shortcuts: [],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null),
    apply: async () => {
      const current = cur.value?.statement?.id;
      if (cur.value?.above) {
        panel.value?.focusElement(cur.value?.above, true);
        file.value?.statementsComponents[cur.value?.above.id]?.focus("last");
      }
      await ops.statement.softDelete(null, current);
    },
  });

  const deleteLeft = provideGlobalAction({
    id: "statement.deleteLeft",
    label: "Delete to the left of current statement (the one above)",
    shortcuts: [],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null),
    apply: async () => {
      if (cur.value?.above) {
        await ops.statement.softDelete(null, cur.value?.above.id);
      }
    },
  });

  // optimistic insert that doesn't wait for the server response
  function _insertOptimisticBlank(parentId: string | null, orderKey: string): { __typename: string; id: string } {
    const newStatement = { __typename: "Statement", ...newNodeIdentity(bench.projectVersionId as string, "Statement") };
    ops.statement.create(null, newStatement.id, newStatement.ck, file.value?.file.id, parentId, orderKey);
    return newStatement;
  }

  // insert statement (as a sibling)
  const insertStart = provideGlobalAction({
    id: "statement.insertStart",
    label: "Insert statement at start of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[file.value?.file.id];
      const firstRootKey = roots?.[0]?.orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimisticBlank(null, generateKeyBetween(null, firstRootKey));
      panel.value?.editElement(newStatement as StatementHeader);
    },
  });
  const insertEnd = provideGlobalAction({
    id: "statement.insertEnd",
    label: "Insert statement at end of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[file.value?.file.id];
      const lastRootKey = roots?.slice(-1)[0].orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimisticBlank(null, generateKeyBetween(lastRootKey, null));
      panel.value?.editElement(newStatement as StatementHeader);
    },
  });
  const insertAbove = provideGlobalAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above",
    shortcuts: ["a"],
    enabled: computed(() => (cur.value?.statement != null || panel.value?.hasSelection) && navigatingFile.value),
    apply: () => {
      const selectedRoots = panel.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      const top = selectedRoots?.[0];
      if (top != null) {
        const previousSibling = file.value?.getPreviousSibling(top);
        const identity = newNodeIdentity(bench.projectVersionId as string, "Statement");
        ops.statement.create(
          null,
          identity.id,
          identity.ck,
          file.value?.file.id,
          top.parent?.id ?? null,
          generateKeyBetween(previousSibling?.orderKey ?? null, cur.value?.orderKey as string)
        );
        // don't switch focus if inserting _before_ current
      }
    },
  });
  const insertBelow = provideGlobalAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below",
    shortcuts: ["b", "shift+enter", "plus"],
    enabled: computed(() => (cur.value?.statement != null || panel.value?.hasSelection) && navigatingFile.value),
    apply: () => {
      const bottom = file.value?.getSelectionBottom();
      const nextSibling = bottom != null ? file.value?.getNextSibling(bottom) : undefined;
      const newStatement = _insertOptimisticBlank(
        bottom?.parent?.id ?? null,
        generateKeyBetween(bottom?.orderKey ?? null, nextSibling?.orderKey ?? null)
      );
      // wait for next tick to ensure there is something to focus (feels a bit hacky)
      nextTick(() => panel.value?.editElement(newStatement as StatementHeader));
    },
  });

  // cut/copy/paste/duplicate
  // TODO :Cleanup: use custom mime type for copied statements
  //  Getting DOMException when trying, likely because the new clipboard API doesn't allow this yet.

  const copy = provideGlobalAction({
    id: "statement.copy",
    label: "Copy statements",
    shortcuts: ["ctrl+c", "meta+c"],
    enabled: computed(() => (cur.value?.statement != null || panel.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      const selectedRoots = panel.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      file.value?.copy(selectedRoots as StatementHeader[]);
    },
  });
  const cut = provideGlobalAction({
    id: "statement.cut",
    label: "Cut statements",
    shortcuts: ["ctrl+x", "meta+x"],
    enabled: computed(() => (cur.value?.statement != null || panel.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      copy.value.apply();
      const selectedRoots = panel.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      panel.value?.blurElement();
      await ops.statement.batchSoftDelete(selectedRoots?.map((s) => s?.id) ?? []);
    },
  });
  const paste = provideGlobalAction({
    id: "statement.paste",
    label: "Paste statements",
    shortcuts: ["ctrl+v", "meta+v"],
    enabled: computed(() => navigatingFile.value),
    apply: async () => {
      await file.value?.paste(undefined, file.value?.getSelectionBottom() as StatementHeader);
    },
  });
  const duplicate = provideGlobalAction({
    id: "statement.duplicate",
    label: "Duplicate statements",
    shortcuts: ["ctrl+d", "meta+d"],
    enabled: computed(() => (cur.value?.statement != null || panel.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      await copy.value.apply();
      paste.value.apply();
    },
  });

  // instance specific actions
  // TODO :Architecture: statement component instance specific actions should be inlined from statment actions
  const run = provideGlobalAction({
    id: "statement.run",
    label: "Run statement",
    shortcuts: ["ctrl+enter", "meta+enter"],
    enabled: computed(() => cur.value?.statement != null),
    apply: async () => {
      cur.value?.component?.run();
    },
  });

  const showActions = provideGlobalAction({
    id: "statement.showActions",
    label: "Show statement actions",
    shortcuts: ["alt+enter", "meta+shift+enter"],
    enabled: computed(() => cur.value?.statement != null),
    apply: async () => {
      cur.value?.component?.showActionsPopover();
    },
  });

  return {
    indent,
    unindent,
    indentSelection,
    unindentSelection,
    moveCurrentUp,
    moveCurrentDown,
    moveSelectionUp,
    moveSelectionDown,
    moveFocusUp,
    moveFocusDown,
    moveFocusIn,
    moveFocusOut,
    expandSelectionUp,
    expandSelectionDown,
    cancelSelection,
    selectAll,
    editCurrent,
    stopEditing,
    delete: delete_,
    deleteSelection,
    deleteCurrentLeft,
    deleteLeft,
    insertStart,
    insertEnd,
    insertAbove,
    insertBelow,
    copy,
    cut,
    paste,
    duplicate,
    run,
  };
}
