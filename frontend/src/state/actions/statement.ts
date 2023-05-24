import { activeFileState, navigationContexts, type NavigationContext } from "@/components/file";
import { StatementType } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newStatementId } from "@/state/operations/statement";
import { useSymbolNavigation } from "@/state/runtime";
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
  const editor = computed(() => file.value?.editor);
  const navigatingFile = computed(() => !editor.value?.editing && bench.focusedViewId == null);

  // move focus
  const moveFocusUp = provideGlobalAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      editor.value?.clearSelection();
      if (cur.value?.above != null) {
        editor.value?.focusElement(cur.value.above, true);
      } else if (cur.value?.statement == null && statements.value.length > 0) {
        // nothing focused, focus last statement
        // note: I have disabled auto-focus last since it leads to some annoying behaviour,
        // particularly when you think something is focused but it's not this leads to jumping
        // editor.focusElement(statements.value[statements.value.length - 1], true);
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
      editor.value?.clearSelection();
      if (cur.value?.below != null) {
        editor.value?.focusElement(cur.value?.below, true);
      } else if (cur.value?.statement == null && statements.value.length > 0) {
        // nothing focused, focus first statement
        if (editor.value?.hasSelection) {
          const selectedRoots = file.value?.getSelectedRoots();
          editor.value?.focusElement(selectedRoots?.[0] as StatementHeader);
        } else {
          editor.value?.focusElement(statements.value[0], true);
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
      editor.value?.clearSelection();
      editor.value?.focusElement(cur.value?.children[0] as StatementHeader, true);
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
      editor.value?.clearSelection();
      const parent = statementsById.value[cur.value?.statement?.parent?.id];
      editor.value?.focusElement(parent, true);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideGlobalAction({
    id: "statement.editCurrent",
    label: "Edit statement",
    shortcuts: ["enter"],
    enabled: computed(() => cur.value?.statement != null && navigatingFile.value),
    apply: () => {
      editor.value?.editElement(cur.value?.statement as StatementHeader);
    },
  });
  const stopEditing = provideGlobalAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing statement",
    shortcuts: ["escape"],
    enabled: computed(() => cur.value?.statement != null && editor.value?.editing && !editor.value?.hasSelection),
    apply: () => {
      editor.value?.stopEditingElement(cur.value?.statement as StatementHeader);
    },
  });

  // indent statement
  const indent = provideGlobalAction({
    id: "statement.indentCurrent",
    label: "Indent statement",
    shortcuts: ["tab"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null && !editor.value?.hasSelection),
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
      () => cur.value?.statement != null && cur.value?.statement.parent != null && !editor.value?.hasSelection
    ),
    apply: async () => {
      await file.value?.unindent(cur.value?.statement as StatementHeader);
    },
  });
  const indentSelection = provideGlobalAction({
    id: "statement.indentSelection",
    label: "Indent selection",
    shortcuts: ["tab"],
    enabled: computed(() => editor.value?.hasSelection),
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
    enabled: computed(() => editor.value?.hasSelection),
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
      () => cur.value?.statement != null && file.value?.current.above != null && !editor.value?.hasSelection
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
      () => cur.value?.statement != null && file.value?.current.belowCurGroup != null && !editor.value?.hasSelection
    ),
    apply: async () => {
      await file.value?.moveDown(cur.value?.statement as StatementHeader);
    },
  });
  const moveSelectionUp = provideGlobalAction({
    id: "statement.moveSelectionUp",
    label: "Move selection up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => editor.value?.hasSelection),
    apply: async () => {
      await file.value?.moveBatchUp(file.value?.getSelectedRoots() ?? []);
    },
  });
  const moveSelectionDown = provideGlobalAction({
    id: "statement.moveSelectionDown",
    label: "Move selection down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => editor.value?.hasSelection),
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
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.value?.previousSelectedStatementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition < (cur.value?.position as number)) {
        editor.value?.removeFromSelection(cur.value?.statement as StatementHeader);
      } else {
        editor.value?.addToSelection(cur.value?.statement as StatementHeader);
        editor.value?.addToSelection(cur.value?.aboveCurGroup as StatementHeader);
      }
      // then move focus up to previous sibling or parent
      // (if above != null, then aboveCurGroup must also be non null)
      editor.value?.focusElement(cur.value?.aboveCurGroup as StatementHeader, false);
    },
  });
  const expandSelectionDown = provideGlobalAction({
    id: "statement.expandSelectionDown",
    label: "Expand selection down",
    shortcuts: ["shift+down"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.belowCurGroup != null),
    apply: () => {
      // remove self from selection if previous selected (last is current) is below
      // (that means we're expanding up)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.value?.previousSelectedStatementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition > (cur.value?.position as number)) {
        editor.value?.removeFromSelection(cur.value?.statement as StatementHeader);
      } else {
        editor.value?.addToSelection(cur.value?.statement as StatementHeader);
        editor.value?.addToSelection(cur.value?.belowCurGroup as StatementHeader);
      }
      // then move focus down
      editor.value?.focusElement(cur.value?.belowCurGroup as StatementHeader, false);
    },
  });
  const cancelSelection = provideGlobalAction({
    id: "statement.cancelSelection",
    label: "Cancel selection",
    shortcuts: ["escape"],
    enabled: computed(() => editor.value?.hasSelection),
    apply: () => {
      editor.value?.clearSelection();
    },
  });
  const selectAll = provideGlobalAction({
    id: "statement.selectAll",
    label: "Select all",
    shortcuts: ["ctrl+a", "meta+a"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      if (editor.value == null) return;
      statements.value.forEach(editor.value?.addToSelection);
    },
  });

  // delete statement
  const delete_ = provideGlobalAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => cur.value?.statement != null && navigatingFile.value && !editor.value?.hasSelection),
    apply: async () => {
      const current = cur.value?.statement?.id;
      if (current == null) return;
      if (cur.value?.above) {
        editor.value?.focusElement(cur.value?.above);
      }
      await ops.statement.softDelete(null, current);
    },
  });
  const deleteSelection = provideGlobalAction({
    id: "statement.deleteSelection",
    label: "Delete selected statements",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => editor.value?.hasSelection && navigatingFile.value),
    apply: async () => {
      // after delete focus next statement above
      if (editor.value == null) return;
      editor.value?.blurElement();
      await ops.statement.batchSoftDelete(editor.value?.selectedElementIds);
    },
  });

  // TODO @Cleanup @Incomplete: provide statement surrounding context to all statements
  //  deleteAboveCurrent action very specific because we don't have
  //  the relevant above/below context in the statement and have no way of
  //  telling other statements what to focus on specifically (start, end, content, etc.)
  //  We should introduce an intermediate statement local context that statement interfaces
  //  can use as well, which could also reduce move focus up/down latency.
  //  :MissingStatementContext
  const deleteAbove = provideGlobalAction({
    id: "statement.deleteCurrentLeft",
    label: "Delete current statement and move to end of above statement",
    shortcuts: [],
    enabled: computed(() => cur.value?.statement != null && cur.value?.above != null),
    apply: async () => {
      const current = cur.value?.statement?.id;
      if (cur.value?.above) {
        editor.value?.focusElement(cur.value?.above, true);
      }
      await ops.statement.softDelete(null, current);
    },
  });

  // jump to reference
  const { focusSymbol } = useSymbolNavigation();
  const jumpToReference = provideGlobalAction({
    id: "statement.jumpToReference",
    label: "Jump to reference",
    shortcuts: ["ctrl+b", "meta+b"],
    enabled: computed(() => cur.value?.statement != null && cur.value?.statement.reference != null),
    apply: () => {
      if (cur.value?.statement?.reference != null) {
        focusSymbol(cur.value?.statement.reference);
      }
    },
  });

  // optimistic insert that doesn't wait for the server response
  function _insertOptimisticBlank(parentId: string | null, orderKey: string): { __typename: string; id: string } {
    const newStatement = { __typename: "Statement", id: newStatementId() };
    ops.statement.create(null, newStatement.id, file.value?.file.id, parentId, orderKey);
    return newStatement;
  }

  // insert statement (as a sibling)
  const insertStart = provideGlobalAction({
    id: "statement.insertStart",
    label: "Insert statement at start of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[""];
      const firstRootKey = roots?.[0]?.orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimisticBlank(null, generateKeyBetween(null, firstRootKey));
      editor.value?.editElement(newStatement as StatementHeader);
    },
  });
  const insertEnd = provideGlobalAction({
    id: "statement.insertEnd",
    label: "Insert statement at end of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[""];
      const lastRootKey = roots?.slice(-1)[0].orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimisticBlank(null, generateKeyBetween(lastRootKey, null));
      editor.value?.editElement(newStatement as StatementHeader);
    },
  });
  const insertAbove = provideGlobalAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above",
    shortcuts: ["a"],
    enabled: computed(() => (cur.value?.statement != null || editor.value?.hasSelection) && navigatingFile.value),
    apply: () => {
      const selectedRoots = editor.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      const top = selectedRoots?.[0];
      if (top != null) {
        const previousSibling = file.value?.getPreviousSibling(top);
        ops.statement.create(
          null,
          newStatementId(),
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
    enabled: computed(() => (cur.value?.statement != null || editor.value?.hasSelection) && navigatingFile.value),
    apply: () => {
      const bottom = file.value?.getSelectionBottom();
      const nextSibling = bottom != null ? file.value?.getNextSibling(bottom) : undefined;
      const newStatement = _insertOptimisticBlank(
        bottom?.parent?.id ?? null,
        generateKeyBetween(bottom?.orderKey ?? null, nextSibling?.orderKey ?? null)
      );
      // wait for next tick to ensure there is something to focus
      // this feels a bit hacky, but focus management will likely be overhauled anyway
      nextTick(() => editor.value?.editElement(newStatement as StatementHeader));
    },
  });

  // toggle comment statement
  const toggleCommented = provideGlobalAction({
    id: "statement.toggleCommentCurrent",
    label: "Comment current statement",
    shortcuts: ["t", "shift+t"],
    enabled: computed(
      () =>
        cur.value?.statement != null &&
        navigatingFile.value &&
        cur.value?.statement.type != StatementType.Blank &&
        cur.value?.statement.type != StatementType.Comment
    ),
    apply: async () => {
      await ops.statement.comment(null, cur.value?.statement?.id, !cur.value?.statement?.commented);
    },
  });

  // cut/copy/paste/duplicate
  // TODO @Cleanup: use custom mime type for copied statements
  //  Getting DOMException when trying, likely because the new clipboard API doesn't allow this yet.

  const copy = provideGlobalAction({
    id: "statement.copy",
    label: "Copy statements",
    shortcuts: ["ctrl+c", "meta+c"],
    enabled: computed(() => (cur.value?.statement != null || editor.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      const selectedRoots = editor.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      file.value?.copy(selectedRoots as StatementHeader[]);
    },
  });
  const cut = provideGlobalAction({
    id: "statement.cut",
    label: "Cut statements",
    shortcuts: ["ctrl+x", "meta+x"],
    enabled: computed(() => (cur.value?.statement != null || editor.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      copy.value.apply();
      const selectedRoots = editor.value?.hasSelection ? file.value?.getSelectedRoots() : [cur.value?.statement];
      editor.value?.blurElement();
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
    enabled: computed(() => (cur.value?.statement != null || editor.value?.hasSelection) && navigatingFile.value),
    apply: async () => {
      await copy.value.apply();
      paste.value.apply();
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
    deleteAbove,
    jumpToReference,
    insertStart,
    insertEnd,
    insertAbove,
    insertBelow,
    toggleCommented,
    copy,
    cut,
    paste,
    duplicate,
  };
}
