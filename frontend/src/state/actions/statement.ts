import { StatementType } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newStatementId } from "@/state/operations/statement";
import { useSymbolNavigation, useSymbolOps } from "@/state/runtime";
import { generateKeyBetween, generateNKeysBetween, INTEGER_ZERO } from "@/utils/fractional";
import { createSharedComposable } from "@vueuse/shared";
import { computed, nextTick, onBeforeUnmount, ref, watchEffect, type Ref } from "vue";

export type FileState = {
  editorId: string;
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

const activeFileState = ref<FileState | null>(null);
export function provideStatementActions(file: Ref<FileState | null>) {
  watchEffect(() => {
    if (file.value?.focused) {
      activeFileState.value = file.value;
    }
  });
  onBeforeUnmount(() => {
    if (activeFileState.value?.editorId === file.value?.editorId) {
      activeFileState.value = null;
    }
  });
}

export const hostStatementActions = createSharedComposable(_provideStatementActions);

function _provideStatementActions() {
  _doProvideStatementActions(activeFileState);
}

function _doProvideStatementActions(file: Ref<FileState | null>) {
  const editor = useEditorState();
  const operations = useOperations();

  const statements = computed(() => file.value?.statements ?? []);
  const statementPositions = computed(() => {
    const result: Record<string, number> = {};
    for (let i = 0; i < statements.value.length; i++) {
      result[statements.value[i].id] = i;
    }
    return result;
  });
  const depths = computed(() => file.value?.depths ?? []);

  function getLocation(statement: StatementHeader) {
    return {
      fileId: file.value?.file.id,
      parentId: statement.parent?.id,
      orderKey: statement?.orderKey ?? INTEGER_ZERO,
    };
  }

  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => {
    const result: Record<string, StatementHeader> = {};
    for (const statement of statements.value) {
      result[statement.id] = statement;
    }
    return result;
  });

  // statementsByParentId must be ordered like orderedStatements
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(() => {
    const result: Record<string, StatementHeader[]> = {};
    for (const statement of statements.value) {
      const parentId = statement.parent?.id ?? "";
      if (!result[parentId]) result[parentId] = [];
      result[parentId].push(statement);
    }
    return result;
  });

  function getSiblings(statement?: StatementHeader) {
    return statementsByParentId.value[statement?.parent?.id ?? ""];
  }

  function getPreviousSibling(statement: StatementHeader) {
    const siblings = getSiblings(statement);
    return siblings[siblings.findIndex((s) => s.id === statement.id) - 1];
  }

  function getNextSibling(statement: StatementHeader) {
    const siblings = getSiblings(statement);
    return siblings[siblings.findIndex((s) => s.id === statement.id) + 1];
  }

  function getSelectedRoots(): StatementHeader[] {
    // Gets the in-selection roots of selected statements (ordered by position)
    // (it can happen that we first select a child, then expand to parent, we only want parent)
    let selectedRoots = editor.selectedElementIds.slice();
    // trim statements whose parents are also selected
    for (const id of editor.selectedElementIds) {
      const children = statementsByParentId.value[id] ?? [];
      selectedRoots = selectedRoots.filter((r) => !children.find((s) => s.id == r));
    }
    return selectedRoots
      .sort((a, b) => statementPositions.value[a] - statementPositions.value[b])
      .map((s) => statementsById.value[s]);
  }

  function getDescendants(statement: StatementHeader): StatementHeader[] {
    const result: StatementHeader[] = [];
    for (const child of statementsByParentId.value[statement.id] ?? []) {
      result.push(child);
      result.push(...getDescendants(child));
    }
    return result.sort((a, b) => statementPositions.value[a.id] - statementPositions.value[b.id]);
  }

  function getAboveCurGroup(statement: StatementHeader) {
    // previous statement before this with depth <= this depth
    for (let i = statementPositions.value[statement.id] - 1; i >= 0; i--) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return undefined;
  }

  function getBelowCurGroup(statement: StatementHeader) {
    // next statement after this with depth <= this depth
    for (let i = statementPositions.value[statement.id] + 1; i < statements.value.length; i++) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return undefined;
  }

  function getSelectionBottom(): StatementHeader | undefined {
    if (editor.hasSelection) {
      const selectedRoots = getSelectedRoots();
      return selectedRoots[selectedRoots.length - 1];
    } else if (statement.value != null) {
      return statement.value;
    } else {
      return statements.value[statements.value.length - 1];
    }
  }

  // actions for currently focused statement
  const statement = computed(() => statementsById.value[editor.focusedElementId as string]);
  const orderKey = computed(() => statement.value?.orderKey ?? INTEGER_ZERO);
  const previousSibling = computed(() => getPreviousSibling(statement.value));
  const children = computed(() => statementsByParentId.value[statement.value?.id ?? ""]);
  const position = computed(() => statementPositions.value[statement.value?.id]);
  const location = computed(() => getLocation(statement.value));

  const above = computed(() => statements.value[position.value - 1]);
  const below = computed(() => statements.value[position.value + 1]);
  const aboveCurGroup = computed(() => getAboveCurGroup(statement.value));
  const belowCurGroup = computed(() => getBelowCurGroup(statement.value));
  const navigatingFile = computed(() => !editor.editingElement && editor.focusedViewId == null);

  // move focus
  const moveFocusUp = provideGlobalAction({
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
  const moveFocusDown = provideGlobalAction({
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
        if (editor.hasSelection) {
          const selectedRoots = getSelectedRoots();
          editor.focusElement(selectedRoots[0]);
        } else {
          editor.focusElement(statements.value[0], true);
        }
      } else if (statement.value != null) {
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
    enabled: computed(() => navigatingFile.value && statement.value != null && children.value?.length > 0),
    apply: () => {
      editor.clearSelection();
      editor.focusElement(children.value[0], true);
    },
  });
  const moveFocusOut = provideGlobalAction({
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
  const editCurrent = provideGlobalAction({
    id: "statement.editCurrent",
    label: "Edit statement",
    shortcuts: ["enter"],
    enabled: computed(() => statement.value != null && navigatingFile.value),
    apply: () => {
      editor.editElement(statement.value);
    },
  });
  const stopEditing = provideGlobalAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing statement",
    shortcuts: ["escape"],
    enabled: computed(() => statement.value != null && editor.editingElement && !editor.hasSelection),
    apply: () => {
      editor.stopEditingElement(statement.value);
    },
  });

  // indent statement
  const indent = provideGlobalAction({
    id: "statement.indentCurrent",
    label: "Indent statement",
    shortcuts: ["tab"],
    enabled: computed(() => statement.value != null && above.value != null && !editor.hasSelection),
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
  const unindent = provideGlobalAction({
    id: "statement.unindentCurrent",
    label: "Unindent statement",
    shortcuts: ["shift+tab"],
    enabled: computed(() => statement.value != null && statement.value.parent != null && !editor.hasSelection),
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
  const indentSelection = provideGlobalAction({
    id: "statement.indentSelection",
    label: "Indent selection",
    shortcuts: ["tab"],
    enabled: computed(() => editor.hasSelection),
    apply: async () => {
      // indent selected roots in line with the topmost selected statement
      const selectedRoots = getSelectedRoots();
      const previousSibling = getPreviousSibling(selectedRoots[0]);
      if (previousSibling == null) {
        return;
      }
      const previousSiblingChildren = statementsByParentId.value[previousSibling.id] ?? [];
      const previousSiblingChildrenLast = previousSiblingChildren.slice(-1)[0];
      // insert all selected roots in order after previous sibling's children
      const ids = selectedRoots.map((r) => r.id);
      const oldLocations = selectedRoots.map((s) => getLocation(s));
      const insertOrderKeys = generateNKeysBetween(
        previousSiblingChildrenLast?.orderKey ?? null,
        null,
        selectedRoots.length
      );
      await operations.statement.batchMove(
        ids,
        oldLocations,
        insertOrderKeys.map((k) => ({ fileId: file.value?.file.id, parentId: previousSibling.id, orderKey: k }))
      );
    },
  });
  const unindentSelection = provideGlobalAction({
    id: "statement.unindentSelection",
    label: "Unindent selection",
    shortcuts: ["shift+tab"],
    enabled: computed(() => editor.hasSelection),
    apply: async () => {
      // unindent selected roots in line with the topmost selected statement
      const selectedRoots = getSelectedRoots();
      const parent = statementsById.value[selectedRoots[0].parent?.id];
      const grandparent = statementsById.value[parent.parent?.id];
      const parentSiblings = statementsByParentId.value[grandparent?.id ?? ""];
      const parentNextSibling = parentSiblings.find((s) => s.orderKey > parent.orderKey);
      // insert all selected roots in order after parent
      const ids = selectedRoots.map((r) => r.id);
      const oldLocations = selectedRoots.map((s) => getLocation(s));
      const insertOrderKeys = generateNKeysBetween(
        parent.orderKey,
        parentNextSibling?.orderKey ?? null,
        selectedRoots.length
      );
      await operations.statement.batchMove(
        ids,
        oldLocations,
        insertOrderKeys.map((k) => ({ fileId: file.value?.file.id, parentId: grandparent?.id, orderKey: k }))
      );
    },
  });

  // move statement up/down
  const moveCurrentUp = provideGlobalAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => statement.value != null && above.value != null && !editor.hasSelection),
    apply: async () => {
      // insert between above and above prev sibling (if any)
      const aboveSiblings = statementsByParentId.value[above.value.parent?.id ?? ""];
      const abovePrevSibling = aboveSiblings
        .slice()
        .reverse()
        .find((s) => s.orderKey < above.value.orderKey);
      const orderKey = generateKeyBetween(abovePrevSibling?.orderKey ?? null, above.value.orderKey);
      const targetLocation = {
        fileId: file.value?.file.id,
        parentId: above.value?.parent?.id,
        orderKey,
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });
  const moveCurrentDown = provideGlobalAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => statement.value != null && belowCurGroup.value != null && !editor.hasSelection),
    apply: async () => {
      if (belowCurGroup.value == null) return;
      // insert between the next group below and its next sibling (if any)
      const belowSiblings = statementsByParentId.value[belowCurGroup.value.parent?.id ?? ""];
      const belowNextSibling = belowSiblings.find(
        (s) => s.orderKey > (belowCurGroup.value as StatementHeader).orderKey
      );
      const orderKey = generateKeyBetween(belowCurGroup.value.orderKey ?? null, belowNextSibling?.orderKey ?? null);
      const targetLocation = {
        fileId: file.value?.file.id,
        parentId: belowCurGroup.value?.parent?.id,
        orderKey,
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });
  const moveSelectionUp = provideGlobalAction({
    id: "statement.moveSelectionUp",
    label: "Move selection up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => editor.hasSelection),
    apply: async () => {
      // move selected roots in line with the topmost selected statement
      // (insert between above and above prev sibling (if any))
      const selectedRoots = getSelectedRoots();
      const above = statements.value[statementPositions.value[selectedRoots[0].id] - 1];
      if (above == null) return;
      const aboveSiblings = statementsByParentId.value[above.parent?.id ?? ""];
      const abovePrevSibling = aboveSiblings
        .slice()
        .reverse()
        .find((s) => s.orderKey < above.orderKey);
      const ids = selectedRoots.map((r) => r.id);
      const oldLocations = selectedRoots.map((s) => getLocation(s));
      const orderKeys = generateNKeysBetween(abovePrevSibling?.orderKey ?? null, above.orderKey, selectedRoots.length);
      const targetLocations = orderKeys.map((k) => ({
        fileId: file.value?.file.id,
        parentId: above.parent?.id,
        orderKey: k,
      }));
      await operations.statement.batchMove(ids, oldLocations, targetLocations);
    },
  });
  const moveSelectionDown = provideGlobalAction({
    id: "statement.moveSelectionDown",
    label: "Move selection down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => editor.hasSelection),
    apply: async () => {
      // move selected roots in line with the bottommost selected statement
      // (insert between the next group below and its next sibling (if any))
      const selectedRoots = getSelectedRoots();
      const belowCurGroup = getBelowCurGroup(selectedRoots[selectedRoots.length - 1]);
      if (belowCurGroup == null) return;
      const belowSiblings = statementsByParentId.value[belowCurGroup.parent?.id ?? ""];
      const belowNextSibling = belowSiblings.find((s) => s.orderKey > belowCurGroup.orderKey);
      const ids = selectedRoots.map((r) => r.id);
      const oldLocations = selectedRoots.map((s) => getLocation(s));
      const orderKeys = generateNKeysBetween(
        belowCurGroup.orderKey,
        belowNextSibling?.orderKey ?? null,
        selectedRoots.length
      );
      const targetLocations = orderKeys.map((k) => ({
        fileId: file.value?.file.id,
        parentId: belowCurGroup.parent?.id,
        orderKey: k,
      }));
      await operations.statement.batchMove(ids, oldLocations, targetLocations);
    },
  });

  // manage selection
  const expandSelectionUp = provideGlobalAction({
    id: "statement.expandSelectionUp",
    label: "Expand selection up",
    shortcuts: ["shift+up"],
    enabled: computed(() => statement.value != null && above.value != null),
    apply: () => {
      // remove self from selection if previous selected (last is current) is above
      // (that means we're expanding down)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.previousSelectedElementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition < position.value) {
        editor.removeFromSelection(statement.value);
      } else {
        editor.addToSelection(statement.value);
        editor.addToSelection(aboveCurGroup.value as StatementHeader);
      }
      // then move focus up to previous sibling or parent
      // (if above != null, then aboveCurGroup must also be non null)
      editor.focusElement(aboveCurGroup.value as StatementHeader, false);
    },
  });
  const expandSelectionDown = provideGlobalAction({
    id: "statement.expandSelectionDown",
    label: "Expand selection down",
    shortcuts: ["shift+down"],
    enabled: computed(() => statement.value != null && belowCurGroup.value != null),
    apply: () => {
      // remove self from selection if previous selected (last is current) is below
      // (that means we're expanding up)
      const prevSelectedPosition = statements.value.findIndex((s) => s.id == editor.previousSelectedElementId);
      if (prevSelectedPosition > -1 && prevSelectedPosition > position.value) {
        editor.removeFromSelection(statement.value);
      } else {
        editor.addToSelection(statement.value);
        editor.addToSelection(belowCurGroup.value as StatementHeader);
      }
      // then move focus down
      editor.focusElement(belowCurGroup.value as StatementHeader, false);
    },
  });
  const cancelSelection = provideGlobalAction({
    id: "statement.cancelSelection",
    label: "Cancel selection",
    shortcuts: ["escape"],
    enabled: computed(() => editor.hasSelection),
    apply: () => {
      editor.clearSelection();
    },
  });
  const selectAll = provideGlobalAction({
    id: "statement.selectAll",
    label: "Select all",
    shortcuts: ["ctrl+a", "meta+a"],
    enabled: computed(() => navigatingFile.value),
    apply: () => {
      statements.value.forEach(editor.addToSelection);
    },
  });

  // delete statement
  const delete_ = provideGlobalAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => statement.value != null && navigatingFile.value && !editor.hasSelection),
    apply: async () => {
      const current = statement.value.id;
      if (above.value) {
        editor.focusElement(above.value);
      }
      await operations.statement.delete(current);
    },
  });
  const deleteSelection = provideGlobalAction({
    id: "statement.deleteSelection",
    label: "Delete selected statements",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => editor.hasSelection && navigatingFile.value),
    apply: async () => {
      // after delete focus next statement above
      editor.blurElement();
      await operations.statement.batchDelete(editor.selectedElementIds);
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
  const jumpToReference = provideGlobalAction({
    id: "statement.jumpToReference",
    label: "Jump to reference",
    shortcuts: ["ctrl+b", "meta+b"],
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
  const insertStart = provideGlobalAction({
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
  const insertEnd = provideGlobalAction({
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
  const insertAbove = provideGlobalAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above",
    shortcuts: ["a"],
    enabled: computed(() => (statement.value != null || editor.hasSelection) && navigatingFile.value),
    apply: () => {
      const selectedRoots = editor.hasSelection ? getSelectedRoots() : [statement.value];
      const top = selectedRoots[0];
      const previousSibling = getPreviousSibling(top);
      operations.statement.create(
        newStatementId(),
        file.value?.file.id,
        top.parent?.id ?? null,
        generateKeyBetween(previousSibling?.orderKey ?? null, orderKey.value)
      );
      // don't switch focus if inserting _before_ current
    },
  });
  const insertBelow = provideGlobalAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below",
    shortcuts: ["b", "shift+enter", "plus"],
    enabled: computed(() => (statement.value != null || editor.hasSelection) && navigatingFile.value),
    apply: () => {
      const bottom = getSelectionBottom();
      const nextSibling = bottom != null ? getNextSibling(bottom) : undefined;
      const newStatement = _insertOptimistic(
        bottom?.parent?.id ?? null,
        generateKeyBetween(bottom?.orderKey ?? null, nextSibling?.orderKey ?? null)
      );
      // wait for next tick to ensure there is something to focus
      // this feels a bit hacky, but focus management will likely be overhauled anyway
      nextTick(() => editor.editElement(newStatement as StatementHeader));
    },
  });

  // toggle comment statement
  const toggleCommented = provideGlobalAction({
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

  // cut/copy/paste/duplicate
  // TODO @Cleanup: use custom mime type for copied statements
  //  Getting DOMException when trying, likely because the new clipboard API doesn't allow this yet.
  const CLIPBOARD_CONTENT_TYPE = "text/plain";
  type CopiedStatement = {
    id: string;
    parentId: string | null;
    parentInCopy?: boolean;
    orderKey: string;
  };
  const copy = provideGlobalAction({
    id: "statement.copy",
    label: "Copy statements",
    shortcuts: ["ctrl+c", "meta+c"],
    enabled: computed(() => (statement.value != null || editor.hasSelection) && navigatingFile.value),
    apply: async () => {
      const selectedRoots = editor.hasSelection ? getSelectedRoots() : [statement.value];
      const copiedStatements = []; // include selected roots and all descendants
      for (const root of selectedRoots) {
        // getDescendants is ordered already
        copiedStatements.push(root);
        copiedStatements.push(...getDescendants(root));
      }
      const copiedStatementsIds = new Set<string>();
      copiedStatements.forEach((s) => copiedStatementsIds.add(s.id));
      // write to clipboard as text/_bench-v0
      const sourceStatements = copiedStatements.map(
        (s) =>
          ({
            id: s.id,
            parentId: s.parent?.id,
            parentInCopy: s.parent == null ? undefined : copiedStatementsIds.has(s.parent?.id ?? ""),
            orderKey: s.orderKey,
          } as CopiedStatement)
      );
      const clipboardItem = [
        new ClipboardItem({
          [CLIPBOARD_CONTENT_TYPE]: new Blob([JSON.stringify(sourceStatements)], { type: CLIPBOARD_CONTENT_TYPE }),
        }),
      ];
      try {
        await navigator.clipboard.write(clipboardItem);
        console.log("copied " + copiedStatements.length + " statements");
      } catch (err) {
        console.error("failed to copy statements", err);
      }
    },
  });
  const cut = provideGlobalAction({
    id: "statement.cut",
    label: "Cut statements",
    shortcuts: ["ctrl+x", "meta+x"],
    enabled: computed(() => (statement.value != null || editor.hasSelection) && navigatingFile.value),
    apply: async () => {
      copy.value.apply();
      const selectedRoots = editor.hasSelection ? getSelectedRoots() : [statement.value];
      editor.blurElement();
      await operations.statement.batchDelete(selectedRoots.map((s) => s.id));
    },
  });
  const paste = provideGlobalAction({
    id: "statement.paste",
    label: "Paste statements",
    shortcuts: ["ctrl+v", "meta+v"],
    enabled: computed(() => navigatingFile.value),
    apply: async () => {
      try {
        const cliboardItems = await navigator.clipboard.read();
        const clipboardDataStr = await (await cliboardItems[0].getType(CLIPBOARD_CONTENT_TYPE)).text();
        const sourceStatements = JSON.parse(clipboardDataStr) as CopiedStatement[];

        // insert at bottom of current selection or file (like in insertBelow, below bottom and its next sibling)
        const bottom = getSelectionBottom();
        const nextSibling = bottom != null ? getNextSibling(bottom) : undefined;
        // project source ids and locations to target at insert point (with new ids)
        const sourceIds = sourceStatements.map((s) => s.id);
        const targetIds: Record<string, string> = {};
        sourceStatements.forEach((s) => (targetIds[s.id] = newStatementId()));
        const targetParentIds = sourceStatements.map((s) =>
          s.parentInCopy && s.parentId != null ? targetIds[s.parentId] : bottom?.parent?.id
        );
        // order keys for root are between bottom and next sibling, all other orders are reset
        const sourceStatementsByParentId: Record<string, string[]> = {};
        sourceStatements.forEach((s) => {
          // group children by parents
          const parentId = s.parentInCopy && s.parentId != null ? s.parentId : "";
          if (sourceStatementsByParentId[parentId] == null) {
            sourceStatementsByParentId[parentId] = [];
          }
          sourceStatementsByParentId[parentId].push(s.id);
        });
        const orderKeysByParentId: Record<string, string[]> = {}; // assign order keys by parent
        Object.entries(sourceStatementsByParentId).forEach(([parentId, childIds]) => {
          if (parentId == "") {
            orderKeysByParentId[""] = generateNKeysBetween(
              bottom?.orderKey ?? null,
              nextSibling?.orderKey ?? null,
              childIds.length
            );
          } else {
            orderKeysByParentId[parentId] = generateNKeysBetween(null, null, childIds.length);
          }
        });
        const targetOrderKeys: string[] = sourceStatements.map((s) => {
          const parentId = s.parentInCopy && s.parentId != null ? s.parentId : "";
          const orderKeys = orderKeysByParentId[parentId];
          const childIndex = sourceStatementsByParentId[parentId].indexOf(s.id);
          return orderKeys[childIndex];
        });

        await operations.statement.batchPaste(
          sourceIds,
          sourceIds.map((id) => targetIds[id]),
          file.value?.file.id,
          targetParentIds,
          targetOrderKeys
        );
        console.log("pasted " + sourceStatements.length + " statements");
        // select the pasted stuff
        if (editor.focusedElementId != null && sourceIds.includes(editor.focusedElementId)) {
          editor.focusElement({ id: targetIds[editor.focusedElementId], __typename: "Statement" });
        } else {
          editor.blurElement();
        }
        editor.clearSelection();
        Object.values(targetIds).forEach((targetId) => editor.addToSelection({ id: targetId }));
      } catch (err) {
        console.error("failed to parse clipboard data", err);
        return;
      }
    },
  });
  const duplicate = provideGlobalAction({
    id: "statement.duplicate",
    label: "Duplicate statements",
    shortcuts: ["ctrl+d", "meta+d"],
    enabled: computed(() => (statement.value != null || editor.hasSelection) && navigatingFile.value),
    apply: async () => {
      await copy.value.apply();
      paste.value.apply();
    },
  });

  // symbol ops
  const symbolOps = useSymbolOps();
  const run = provideGlobalAction({
    id: "statement.run",
    label: "Run current statement",
    shortcuts: ["ctrl+r", "meta+r"],
    enabled: computed(() => statement.value != null && navigatingFile.value && !editor.hasSelection),
    apply: async () => {
      await symbolOps.openRun(statement.value);
    },
  });
  const build = provideGlobalAction({
    id: "statement.build",
    label: "Build current statement",
    shortcuts: ["ctrl+b", "meta+b"],
    enabled: computed(() => statement.value != null && navigatingFile.value && !editor.hasSelection),
    apply: async () => {
      await symbolOps.build(statement.value);
    },
  });
  const evaluate = provideGlobalAction({
    id: "statement.evaluate",
    label: "Evaluate current statement",
    shortcuts: ["ctrl+e", "meta+e"],
    enabled: computed(() => statement.value != null && navigatingFile.value && !editor.hasSelection),
    apply: async () => {
      await symbolOps.evaluate(statement.value);
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
    run,
    build,
    evaluate,
  };
}
