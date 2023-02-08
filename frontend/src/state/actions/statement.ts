import { StatementType } from "@/gql/graphql";
import { provideSingletonAction } from "@/state/actions";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newStatementId } from "@/state/operations/statement";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { computed, type Ref } from "vue";

export function provideStatementActions(
  enabled: Ref<boolean>,
  file: Ref<FileHeader | undefined>,
  orderedStatements: Ref<StatementHeader[]>,
  depths: Ref<number[]>
) {
  const editor = useEditorState();
  const operations = useOperations();

  function getLocation(statement: StatementHeader) {
    return {
      fileId: file.value?.id,
      parentId: statement.parent?.id,
      orderKey: statement?.orderKey ?? INTEGER_ZERO,
    };
  }

  const symbolsById: Ref<Record<string, StatementHeader>> = computed(() => {
    if (!enabled.value) return {};
    const result: Record<string, StatementHeader> = {};
    for (const statement of orderedStatements.value) {
      result[statement.id] = statement;
    }
    return result;
  });

  // statementsByParentId must be ordered like orderedStatements
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(() => {
    if (!enabled.value) return {};
    const result: Record<string, StatementHeader[]> = {};
    for (const statement of orderedStatements.value) {
      const parentId = statement.parent?.id ?? "";
      if (!result[parentId]) result[parentId] = [];
      result[parentId].push(statement);
    }
    return result;
  });

  // actions for currently focused statement
  const statement = computed(() => symbolsById.value[editor.focusedElementId as string]);
  const orderKey = computed(() => statement.value?.orderKey ?? INTEGER_ZERO);
  const siblings = computed(() => statementsByParentId.value[statement.value?.parent?.id ?? ""]);
  const children = computed(() => statementsByParentId.value[statement.value?.id ?? ""]);
  const position = computed(() => orderedStatements.value.findIndex((s) => s.id === statement.value?.id));
  const location = computed(() => getLocation(statement.value));

  const above = computed(() => orderedStatements.value[position.value - 1]);
  const below = computed(() => orderedStatements.value[position.value + 1]);
  const belowCurGroup = computed(() => {
    // next statement after this with depth <= this depth
    for (let i = position.value + 1; i < orderedStatements.value.length; i++) {
      if (depths.value[i] <= depths.value[position.value]) {
        return orderedStatements.value[i];
      }
    }
    return undefined;
  });

  // move statement
  const moveCurrentIn = provideSingletonAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    enabled: computed(() => !!statement.value && above.value != null),
    // we can only indent if there is a sibling above
    registered: enabled,
    apply: async () => {
      // move to end of previous sibling's children
      const previousSibling = siblings.value[siblings.value.findIndex((s) => s.id === statement.value.id) - 1];
      if (previousSibling == null) {
        // cannot move
        return;
      }
      const previousSiblingChildren = statementsByParentId.value[previousSibling.id] ?? [];
      const previousSiblingChildrenLast = previousSiblingChildren.slice(-1)[0];
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value?.id,
        parentId: previousSibling?.id,
        orderKey: generateKeyBetween(previousSiblingChildrenLast?.orderKey ?? null, null),
      });
    },
  });

  const moveCurrentOut = provideSingletonAction({
    id: "statement.moveCurrentOut",
    label: "Move statement out",
    shortcuts: ["shift+tab"],
    enabled: computed(() => !!statement.value && !!statement.value.parent),
    registered: enabled,
    apply: async () => {
      // move to after parent in grandparent's children
      const parent = symbolsById.value[statement.value.parent?.id];
      const grandparent = symbolsById.value[parent.parent?.id];
      const parentSiblings = statementsByParentId.value[grandparent?.id ?? ""];
      const parentNextSibling = parentSiblings.find((s) => s.orderKey > parent.orderKey);
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value?.id,
        parentId: grandparent?.id,
        orderKey: generateKeyBetween(parent.orderKey, parentNextSibling?.orderKey ?? null),
      });
    },
  });

  const moveCurrentUp = provideSingletonAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => !!statement.value && above.value != null),
    registered: enabled,
    apply: async () => {
      // insert between above and above prev sibling (if any)
      const aboveSiblings = statementsByParentId.value[above.value.parent?.id ?? ""];
      const abovePrevSibling = aboveSiblings
        .slice()
        .reverse()
        .find((s) => s.orderKey < above.value.orderKey);
      const targetLocation = {
        fileId: file.value?.id,
        parentId: above.value?.parent?.id,
        orderKey: generateKeyBetween(abovePrevSibling?.orderKey ?? null, above.value.orderKey),
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });

  const moveCurrentDown = provideSingletonAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => !!statement.value && belowCurGroup.value != null),
    registered: enabled,
    apply: async () => {
      if (belowCurGroup.value == null) return;
      // insert between the next group below and its next sibling (if any)
      const belowSiblings = statementsByParentId.value[belowCurGroup.value.parent?.id ?? ""];
      const belowNextSibling = belowSiblings.find((s) => s.orderKey > belowCurGroup.value.orderKey);
      const targetLocation = {
        fileId: file.value?.id,
        parentId: below.value?.parent?.id,
        orderKey: generateKeyBetween(belowCurGroup.value.orderKey ?? null, belowNextSibling?.orderKey ?? null),
      };
      await operations.statement.move(statement.value.id, location.value, targetLocation);
    },
  });

  // move focus
  const moveFocusUp = provideSingletonAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    enabled: computed(() => !editor.editingElement),
    registered: enabled,
    apply: () => {
      if (above.value != null) {
        editor.focusElement(above.value, true);
      } else if (statement.value == null && orderedStatements.value.length > 0) {
        // nothing focused, focus last statement
        editor.focusElement(orderedStatements.value[orderedStatements.value.length - 1], true);
      }
    },
  });
  const moveFocusDown = provideSingletonAction({
    id: "statement.moveFocusDown",
    label: "Move focus down",
    enabled: computed(() => !editor.editingElement),
    shortcuts: ["down"],
    registered: enabled,
    apply: () => {
      if (below.value != null) {
        editor.focusElement(below.value, true);
      } else if (statement.value == null && orderedStatements.value.length > 0) {
        // nothing focused, focus first statement
        editor.focusElement(orderedStatements.value[0], true);
      }
    },
  });
  // move focus in/out
  const moveFocusIn = provideSingletonAction({
    id: "statement.moveFocusIn",
    label: "Move focus in",
    shortcuts: ["right"],
    enabled: computed(() => !editor.editingElement && !!statement.value && children.value?.length > 0),
    registered: enabled,
    apply: () => {
      editor.focusElement(children.value[0], true);
    },
  });
  const moveFocusOut = provideSingletonAction({
    id: "statement.moveFocusOut",
    label: "Move focus out",
    shortcuts: ["left"],
    enabled: computed(() => !editor.editingElement && !!statement.value && !!statement.value.parent),
    registered: enabled,
    apply: () => {
      const parent = symbolsById.value[statement.value.parent?.id];
      editor.focusElement(parent, true);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideSingletonAction({
    id: "statement.editCurrent",
    label: "Edit current statement",
    shortcuts: ["enter"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: () => {
      editor.editElement(statement.value);
    },
  });
  const stopEditingCurrent = provideSingletonAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing current statement",
    shortcuts: ["escape"],
    enabled: computed(() => !!statement.value && editor.editingElement),
    registered: enabled,
    apply: () => {
      editor.stopEditingElement(statement.value);
    },
  });

  // delete statement
  const deleteCurrent = provideSingletonAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["d", "backspace", "delete"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const current = statement.value.id;
      if (above.value) {
        editor.focusElement(above.value);
      }
      await operations.statement.delete(current);
    },
  });
  // TODO @Cleanup: provide statement surrounding context to all statements
  //  This very specific deleteAboveCurrent action is testatment to the
  //  slightly clumsiness of only having the relevant above/below context in
  //  statement actions because it's provided by the FileInterface. We should
  //  introduce an intermediate statement local context that statement interfaces
  //  can use as well. :MissingStatementContext
  const deleteAboveCurrent = provideSingletonAction({
    id: "statement.deleteAboveCurrent",
    label: "Delete statement above current statement",
    shortcuts: ["shift+backspace", "shift+delete"],
    enabled: computed(() => !!statement.value && above.value != null),
    registered: enabled,
    apply: async () => {
      if (above.value) {
        await operations.statement.delete(above.value.id);
      }
    },
  });

  // insert statement (as a sibling)
  const insertStart = provideSingletonAction({
    id: "statement.insertStart",
    label: "Insert statement at start of file",
    shortcuts: [],
    enabled: computed(() => !!file.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const roots = statementsByParentId.value[""];
      const firstRootKey = roots?.[0]?.orderKey ?? INTEGER_ZERO;
      const newStatement = await operations.statement.create(
        newStatementId(),
        file.value?.id,
        null,
        generateKeyBetween(null, firstRootKey)
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertEnd = provideSingletonAction({
    id: "statement.insertEnd",
    label: "Insert statement at end of file",
    shortcuts: [],
    enabled: computed(() => !!file.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const roots = statementsByParentId.value[""];
      const lastRootKey = roots?.slice(-1)[0].orderKey ?? INTEGER_ZERO;
      const newStatement = await operations.statement.create(
        newStatementId(),
        file.value?.id,
        null,
        generateKeyBetween(lastRootKey, null)
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertAboveCurrent = provideSingletonAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above current",
    shortcuts: ["a"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      await operations.statement.create(
        newStatementId(),
        file.value?.id,
        statement.value.parent?.id ?? null,
        generateKeyBetween(above.value?.orderKey ?? null, orderKey.value)
      );
      // don't switch focus if inserting _before_ current
    },
  });
  const insertBelowCurrent = provideSingletonAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below current",
    shortcuts: ["i", "b", "shift+enter", "plus"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const newStatement = await operations.statement.create(
        newStatementId(),
        file.value?.id,
        statement.value.parent?.id ?? null,
        generateKeyBetween(orderKey.value, below.value?.orderKey ?? null)
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });

  // toggle comment statement
  const toggleCommentedCurrent = provideSingletonAction({
    id: "statement.toggleCommentCurrent",
    label: "Comment current statement",
    shortcuts: ["t", "shift+t"],
    enabled: computed(
      () =>
        !!statement.value &&
        !editor.editingElement &&
        statement.value.type != StatementType.Blank &&
        statement.value.type != StatementType.Comment
    ),
    registered: enabled,
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
    editCurrent,
    stopEditingCurrent,
    deleteCurrent,
    deleteAboveCurrent,
    insertStart,
    insertEnd,
    insertAboveCurrent,
    insertBelowCurrent,
    toggleCommentedCurrent,
  };
}
