import { StatementType } from "@/gql/graphql";
import { provideSharedAction } from "@/state/actions";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newStatementId } from "@/state/operations/statement";
import { useSymbolNavigation } from "@/state/runtime";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { createSharedComposable } from "@vueuse/shared";
import { computed, nextTick, onBeforeUnmount, ref, watchEffect, type Ref } from "vue";

export type FileState = {
  focused: boolean;
  file: FileHeader;
  statements: StatementHeader[]; // ordered
  depths: number[];
  navigateUp: () => void;
  navigateDown: () => void;
};

// There can only be one active file to provide file shortcuts,
// so we have a global reference here that is automatically set to the focused file.
// We can't just use singleton actions here because multiple files may have
// 'focused' set during moves or transition.
const activeFileState: Ref<FileState | null> = ref(null);
export function provideStatementActions(file: Ref<FileState | null>) {
  watchEffect(() => {
    if (file.value?.focused) {
      activeFileState.value = file.value;
    }
  });

  onBeforeUnmount(() => {
    if (activeFileState.value?.file.id == file.value?.file.id) {
      activeFileState.value = null;
    }
  });

  doProvideStatementActions(activeFileState);
}

const doProvideStatementActions = createSharedComposable(_doProvideStatementActions);

function _doProvideStatementActions(file: Ref<FileState | null>) {
  const editor = useEditorState();
  const operations = useOperations();

  const enabled = computed(() => file.value != null);
  const statements = computed(() => file.value?.statements ?? []);
  const depths = computed(() => file.value?.depths ?? []);

  function getLocation(statement: StatementHeader) {
    return {
      fileId: file.value?.file.id,
      parentId: statement.parent?.id,
      orderKey: statement?.orderKey ?? INTEGER_ZERO,
    };
  }

  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => {
    if (!file.value) return {};
    const result: Record<string, StatementHeader> = {};
    for (const statement of statements.value) {
      result[statement.id] = statement;
    }
    return result;
  });

  // statementsByParentId must be ordered like orderedStatements
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(() => {
    if (!file.value) return {};
    const result: Record<string, StatementHeader[]> = {};
    for (const statement of statements.value) {
      const parentId = statement.parent?.id ?? "";
      if (!result[parentId]) result[parentId] = [];
      result[parentId].push(statement);
    }
    return result;
  });

  // actions for currently focused statement
  const statement = computed(() => statementsById.value[editor.focusedElementId as string]);
  const orderKey = computed(() => statement.value?.orderKey ?? INTEGER_ZERO);
  const siblings = computed(() => statementsByParentId.value[statement.value?.parent?.id ?? ""]);
  const previousSibling = computed(
    () => siblings.value[siblings.value.findIndex((s) => s.id === statement.value.id) - 1]
  );
  const nextSibling = computed(() => siblings.value[siblings.value.findIndex((s) => s.id === statement.value.id) + 1]);
  const children = computed(() => statementsByParentId.value[statement.value?.id ?? ""]);
  const position = computed(() => statements.value.findIndex((s) => s.id === statement.value?.id));
  const location = computed(() => getLocation(statement.value));

  const above = computed(() => statements.value[position.value - 1]);
  const below = computed(() => statements.value[position.value + 1]);
  const aboveCurGroup = computed(() => {
    // previous statement before this with depth <= this depth
    for (let i = position.value - 1; i >= 0; i--) {
      if (depths.value[i] <= depths.value[position.value]) {
        return statements.value[i];
      }
    }
    return undefined;
  });
  const belowCurGroup = computed(() => {
    // next statement after this with depth <= this depth
    for (let i = position.value + 1; i < statements.value.length; i++) {
      if (depths.value[i] <= depths.value[position.value]) {
        return statements.value[i];
      }
    }
    return undefined;
  });
  const navigatingFile = computed(() => !editor.editingElement && editor.focusedViewId == null);

  // indent statement
  const moveCurrentIn = provideSharedAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    enabled: computed(() => statement.value != null && above.value != null),
    // we can only indent if there is a sibling above
    apply: async () => {
      // move to end of previous sibling's children
      if (previousSibling.value == null) {
        return;
      }
      const previousSiblingChildren = statementsByParentId.value[previousSibling.value.id] ?? [];
      const previousSiblingChildrenLast = previousSiblingChildren.slice(-1)[0];
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value?.file.id,
        parentId: previousSibling.value?.id,
        orderKey: generateKeyBetween(previousSiblingChildrenLast?.orderKey ?? null, null),
      });
    },
  });
  const moveCurrentOut = provideSharedAction({
    id: "statement.moveCurrentOut",
    label: "Move statement out",
    shortcuts: ["shift+tab"],
    enabled: computed(() => statement.value != null && statement.value.parent != null),
    apply: async () => {
      // move to after parent in grandparent's children
      const parent = statementsById.value[statement.value.parent?.id];
      const grandparent = statementsById.value[parent.parent?.id];
      const parentSiblings = statementsByParentId.value[grandparent?.id ?? ""];
      const parentNextSibling = parentSiblings.find((s) => s.orderKey > parent.orderKey);
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value?.file.id,
        parentId: grandparent?.id,
        orderKey: generateKeyBetween(parent.orderKey, parentNextSibling?.orderKey ?? null),
      });
    },
  });

  // move statement up/down
  const moveCurrentUp = provideSharedAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => statement.value != null && above.value != null),
    apply: async () => {
      // insert between above and above prev sibling (if any)
      const aboveSiblings = statementsByParentId.value[above.value.parent?.id ?? ""];
      const abovePrevSibling = aboveSiblings
        .slice()
        .reverse()
        .find((s) => s.orderKey < above.value.orderKey);
      const targetLocation = {
        fileId: file.value?.file.id,
        parentId: above.value?.parent?.id,
        orderKey: generateKeyBetween(abovePrevSibling?.orderKey ?? null, above.value.orderKey),
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });
  const moveCurrentDown = provideSharedAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => statement.value != null && belowCurGroup.value != null),
    apply: async () => {
      if (belowCurGroup.value == null) return;
      // insert between the next group below and its next sibling (if any)
      const belowSiblings = statementsByParentId.value[belowCurGroup.value.parent?.id ?? ""];
      const belowNextSibling = belowSiblings.find(
        (s) => s.orderKey > (belowCurGroup.value as StatementHeader).orderKey
      );
      const targetLocation = {
        fileId: file.value?.file.id,
        parentId: belowCurGroup.value?.parent?.id,
        orderKey: generateKeyBetween(belowCurGroup.value.orderKey ?? null, belowNextSibling?.orderKey ?? null),
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });

  // move focus
  const moveFocusUp = provideSharedAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      editor.clearSelection();
      if (above.value != null) {
        editor.focusElement(above.value, true);
      } else if (statement.value == null && statements.value.length > 0) {
        // nothing focused, focus last statement
        // note: I have disabled auto-focus last since it leads to some annoying behaviour,
        // particularly when you think something is focused but it's not this leads to jumping
        // editor.focusElement(statements.value[statements.value.length - 1], true);
      } else if (statement.value != null) {
        // navigate up from statements
        file.value?.navigateUp();
      }
    },
  });
  const moveFocusDown = provideSharedAction({
    id: "statement.moveFocusDown",
    label: "Move focus down",
    enabled: computed(() => navigatingFile.value),
    shortcuts: ["down"],
    apply: () => {
      editor.clearSelection();
      if (below.value != null) {
        editor.focusElement(below.value, true);
      } else if (statement.value == null && statements.value.length > 0) {
        // nothing focused, focus first statement
        editor.focusElement(statements.value[0], true);
      } else if (statement.value != null) {
        // navigate down from statements
        file.value?.navigateDown();
      }
    },
  });
  // move focus in/out
  const moveFocusIn = provideSharedAction({
    id: "statement.moveFocusIn",
    label: "Move focus in",
    shortcuts: ["right"],
    enabled: computed(() => navigatingFile.value && statement.value != null && children.value?.length > 0),
    apply: () => {
      editor.clearSelection();
      editor.focusElement(children.value[0], true);
    },
  });
  const moveFocusOut = provideSharedAction({
    id: "statement.moveFocusOut",
    label: "Move focus out",
    shortcuts: ["left"],
    enabled: computed(() => navigatingFile.value && statement.value != null && statement.value.parent != null),
    apply: () => {
      editor.clearSelection();
      const parent = statementsById.value[statement.value.parent?.id];
      editor.focusElement(parent, true);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideSharedAction({
    id: "statement.editCurrent",
    label: "Edit current statement",
    shortcuts: ["enter"],
    enabled: computed(() => statement.value != null && navigatingFile.value),
    apply: () => {
      editor.editElement(statement.value);
    },
  });
  const stopEditingCurrent = provideSharedAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing current statement",
    shortcuts: ["escape"],
    enabled: computed(() => statement.value != null && editor.editingElement && !editor.hasSelection),
    apply: () => {
      editor.stopEditingElement(statement.value);
    },
  });

  // manage selection
  const expandSelectionUp = provideSharedAction({
    id: "statement.expandSelectionUp",
    label: "Expand selection up",
    shortcuts: ["shift+up"],
    enabled: computed(() => statement.value != null && above.value != null && editor.hasSelection),
    apply: () => {
      // remove self from selection if previous selected (last is current) is above
      // (that means we're expanding down)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.previousSelectedElementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition < position.value) {
        editor.removeFromSelection(statement.value);
      }
      // then move focus up to previous sibling or parent
      // (if above != null, then aboveCurGroup must also be non null)
      editor.focusElement(aboveCurGroup.value as StatementHeader, false);
    },
  });
  const expandSelectionDown = provideSharedAction({
    id: "statement.expandSelectionDown",
    label: "Expand selection down",
    shortcuts: ["shift+down"],
    enabled: computed(() => statement.value != null && belowCurGroup.value != null && editor.hasSelection),
    apply: () => {
      // remove self from selection if previous selected (last is current) is below
      // (that means we're expanding up)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.previousSelectedElementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition > position.value) {
        editor.removeFromSelection(statement.value);
      }
      // then move focus down
      editor.focusElement(belowCurGroup.value as StatementHeader, false);
    },
  });
  const cancelSelection = provideSharedAction({
    id: "statement.cancelSelection",
    label: "Cancel selection",
    shortcuts: ["escape"],
    enabled: computed(() => editor.hasSelection),
    apply: () => {
      editor.clearSelection();
    },
  });
  const selectAll = provideSharedAction({
    id: "statement.selectAll",
    label: "Select all",
    shortcuts: ["ctrl+a"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      statements.value.forEach(editor.addToSelection);
    },
  });

  // delete statement
  const deleteCurrent = provideSharedAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => statement.value != null && navigatingFile.value),
    apply: async () => {
      // after delete focus next statement above
      if (editor.hasSelection) {
        // batch delete
        editor.blurElement();
        await operations.statement.batchDelete(editor.selectedElementIds);
      } else {
        // single delete
        const current = statement.value.id;
        if (above.value) {
          editor.focusElement(above.value);
        }
        await operations.statement.delete(current);
      }
    },
  });
  // TODO @Cleanup @Incomplete: provide statement surrounding context to all statements
  //  deleteAboveCurrent action very specific because we don't have
  //  the relevant above/below context in the statement and have no way of
  //  telling other statements what to focus on specifically (start, end, content, etc.)
  //  We should introduce an intermediate statement local context that statement interfaces
  //  can use as well, which could also reduce move focus up/down latency.
  //  :MissingStatementContext
  const deleteAboveCurrent = provideSharedAction({
    id: "statement.deleteCurrentLeft",
    label: "Delete current statement and move to end of above statement",
    shortcuts: [],
    enabled: computed(() => statement.value != null && above.value != null),
    apply: async () => {
      const current = statement.value.id;
      if (above.value) {
        editor.focusElement(above.value, true);
      }
      await operations.statement.delete(current);
    },
  });

  // jump to reference
  const { focusSymbol } = useSymbolNavigation();
  const jumpToReference = provideSharedAction({
    id: "statement.jumpToReference",
    label: "Jump to reference",
    shortcuts: ["ctrl+b"],
    enabled: computed(() => statement.value != null && statement.value.reference != null),
    apply: () => {
      if (statement.value.reference != null) {
        focusSymbol(statement.value.reference);
      }
    },
  });

  // optimistic insert that doesn't wait for the server response
  function _insertOptimistic(parentId: string | null, orderKey: string): { __typename: string; id: string } {
    const newStatement = { __typename: "Statement", id: newStatementId() };
    operations.statement.create(newStatement.id, file.value?.file.id, parentId, orderKey);
    return newStatement;
  }

  // insert statement (as a sibling)
  const insertStart = provideSharedAction({
    id: "statement.insertStart",
    label: "Insert statement at start of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[""];
      const firstRootKey = roots?.[0]?.orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimistic(null, generateKeyBetween(null, firstRootKey));
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertEnd = provideSharedAction({
    id: "statement.insertEnd",
    label: "Insert statement at end of file",
    shortcuts: [],
    enabled: computed(() => file.value != null && navigatingFile.value),
    apply: () => {
      const roots = statementsByParentId.value[""];
      const lastRootKey = roots?.slice(-1)[0].orderKey ?? INTEGER_ZERO;
      const newStatement = _insertOptimistic(null, generateKeyBetween(lastRootKey, null));
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertAboveCurrent = provideSharedAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above current",
    shortcuts: ["a"],
    enabled: computed(() => statement.value != null && navigatingFile.value),
    apply: () => {
      operations.statement.create(
        newStatementId(),
        file.value?.file.id,
        statement.value.parent?.id ?? null,
        generateKeyBetween(previousSibling.value?.orderKey ?? null, orderKey.value)
      );
      // don't switch focus if inserting _before_ current
    },
  });
  const insertBelowCurrent = provideSharedAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below current",
    shortcuts: ["i", "b", "shift+enter", "plus"],
    enabled: computed(() => statement.value != null && navigatingFile.value),
    apply: () => {
      const newStatement = _insertOptimistic(
        statement.value.parent?.id ?? null,
        generateKeyBetween(orderKey.value, nextSibling.value?.orderKey ?? null)
      );
      // wait for next tick to ensure there is something to focus
      // this feels a bit hacky, but focus management will likely be overhauled anyway
      nextTick(() => editor.editElement(newStatement as StatementHeader));
    },
  });

  // toggle comment statement
  const toggleCommentedCurrent = provideSharedAction({
    id: "statement.toggleCommentCurrent",
    label: "Comment current statement",
    shortcuts: ["t", "shift+t"],
    enabled: computed(
      () =>
        statement.value != null &&
        navigatingFile.value &&
        statement.value.type != StatementType.Blank &&
        statement.value.type != StatementType.Comment
    ),
    apply: async () => {
      await operations.statement.comment(statement.value.id, !statement.value.commented);
    },
  });

  return {
    moveCurrentIn,
    moveCurrentOut,
    moveCurrentUp,
    moveCurrentDown,
    moveFocusUp,
    moveFocusDown,
    moveFocusIn,
    moveFocusOut,
    expandSelectionUp,
    expandSelectionDown,
    cancelSelection,
    selectAll,
    editCurrent,
    stopEditingCurrent,
    deleteCurrent,
    deleteAboveCurrent,
    jumpToReference,
    insertStart,
    insertEnd,
    insertAboveCurrent,
    insertBelowCurrent,
    toggleCommentedCurrent,
  };
}
