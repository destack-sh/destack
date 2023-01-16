import { StatementType } from "@/gql/graphql";
import { provideGlobalAction, provideSingletonAction } from "@/state/actions";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { computed, type Ref } from "vue";

export function useStatementActions(
  enabled: Ref<boolean>,
  file: Ref<FileHeader>,
  orderedStatements: Ref<StatementHeader[]>
) {
  const editor = useEditorState();
  const operations = useOperations();

  function getLocation(statement: StatementHeader) {
    return {
      fileId: file.value.id,
      parentId: statement.parent?.id,
      index: index.value,
    };
  }

  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => {
    if (!enabled.value) return {};
    const result: Record<string, StatementHeader> = {};
    for (const statement of orderedStatements.value) {
      result[statement.id] = statement;
    }
    return result;
  });

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
  const statement = computed(() => statementsById.value[editor.focusedElementId as string]);
  const index = computed(() => statement.value?.index ?? -1);
  const siblings = computed(() => statementsByParentId.value[statement.value?.parent?.id ?? ""]);
  const children = computed(() => statementsByParentId.value[statement.value?.id ?? ""]);
  const position = computed(() => orderedStatements.value.findIndex((s) => s.id === statement.value?.id));
  const location = computed(() => getLocation(statement.value));

  const statementAbove = computed(() => orderedStatements.value[position.value - 1]);
  const statementBelow = computed(() => orderedStatements.value[position.value + 1]);

  // move statement
  const moveCurrentIn = provideSingletonAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    // we can only indent if there is a sibling above
    enabled: computed(() => !editor.editingElement && !!statement.value && index.value > 0),
    registered: enabled,
    apply: async () => {
      // insert at end of previous sibling children (leave index undefined)
      const previousSibling = siblings.value[index.value - 1];
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value.id,
        parentId: previousSibling.id,
        index: undefined,
      });
    },
  });

  const moveCurrentOut = provideGlobalAction({
    id: "statement.moveCurrentOut",
    label: "Move statement out",
    shortcuts: ["shift+tab"],
    // we can only outdent if there is a parent
    enabled: computed(() => !editor.editingElement && !!statement.value && !!statement.value.parent),
    registered: enabled,
    apply: async () => {
      const parent = statementsById.value[statement.value.parent?.id];
      const grandparent = statementsById.value[parent.parent?.id];
      // insert after parent (leave index undefined)
      await operations.statement.move(statement.value.id, location.value, {
        fileId: file.value.id,
        parentId: grandparent.id,
        index: (grandparent?.index ?? 0) + 1,
      });
    },
  });

  const moveCurrentUp = provideGlobalAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => !!statement.value && statementAbove.value != null),
    registered: enabled,
    apply: async () => {
      await operations.statement.move(statement.value.id, location.value, getLocation(statementAbove.value));
    },
  });

  const moveCurrentDown = provideGlobalAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => !!statement.value && statementBelow.value != null),
    registered: enabled,
    apply: async () => {
      if (statementBelow.value) {
        await operations.statement.move(statement.value.id, location.value, getLocation(statementBelow.value));
      }
    },
  });

  // move focus
  const moveFocusUp = provideGlobalAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    registered: enabled,
    apply: () => {
      if (statementAbove.value != null) {
        editor.focusElement(statementAbove.value, true);
      } else if (statement.value == null && orderedStatements.value.length > 0) {
        // nothing focused, focus last statement
        editor.focusElement(orderedStatements.value[orderedStatements.value.length - 1], true);
      }
    },
  });
  const moveFocusDown = provideGlobalAction({
    id: "statement.moveFocusDown",
    label: "Move focus down",
    shortcuts: ["down"],
    registered: enabled,
    apply: () => {
      console.log("move focus down");
      if (statementBelow.value != null) {
        editor.focusElement(statementBelow.value, true);
      } else if (statement.value == null && orderedStatements.value.length > 0) {
        // nothing focused, focus first statement
        editor.focusElement(orderedStatements.value[0], true);
      }
    },
  });
  // move focus in/out
  const moveFocusIn = provideGlobalAction({
    id: "statement.moveFocusIn",
    label: "Move focus in",
    shortcuts: ["right"],
    enabled: computed(() => !editor.editingElement && !!statement.value && children.value?.length > 0),
    registered: enabled,
    apply: () => {
      editor.focusElement(children.value[0], true);
    },
  });
  const moveFocusOut = provideGlobalAction({
    id: "statement.moveFocusOut",
    label: "Move focus out",
    shortcuts: ["left"],
    enabled: computed(() => !editor.editingElement && !!statement.value && !!statement.value.parent),
    registered: enabled,
    apply: () => {
      const parent = statementsById.value[statement.value.parent?.id];
      editor.focusElement(parent, true);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideGlobalAction({
    id: "statement.editCurrent",
    label: "Edit current statement",
    shortcuts: ["enter"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: () => {
      editor.editElement(statement.value);
    },
  });
  const stopEditingCurrent = provideGlobalAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing current statement",
    shortcuts: ["escape"],
    enabled: computed(() => !!statement.value && editor.editingElement),
    registered: enabled,
    apply: () => {
      editor.stopEditingElement();
    },
  });

  // delete statement
  const deleteCurrent = provideGlobalAction({
    id: "statement.deleteCurrent",
    label: "Delete current statement",
    shortcuts: ["d", "backspace", "delete"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const below = statementBelow.value;
      await operations.statement.delete(statement.value.id);
      if (below) {
        editor.focusElement(below);
      }
    },
  });

  // insert statement (as a sibling)
  const insertBeforeCurrent = provideGlobalAction({
    id: "statement.insertAboveCurrent",
    label: "Insert statement above current",
    shortcuts: ["a"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const newStatement = await operations.statement.create(
        file.value.id,
        statement.value.parent?.id ?? null,
        statement.value.index ?? 0,
        StatementType.Blank
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertAfterCurrent = provideGlobalAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below current",
    shortcuts: ["i", "b", "shift+enter", "plus"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const newStatement = await operations.statement.create(
        file.value.id,
        statement.value.parent?.id ?? null,
        (statement.value.index ?? 0) + 1,
        StatementType.Blank
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });
  const insertChildCurrent = provideGlobalAction({
    id: "statement.insertChildCurrent",
    label: "Insert statement as child of current",
    shortcuts: ["shift+i", "shift+b", "shift+plus"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    registered: enabled,
    apply: async () => {
      const newStatement = await operations.statement.create(
        file.value.id,
        statement.value.id,
        statementsByParentId.value[statement.value.id]?.length ?? 0,
        StatementType.Blank
      );
      editor.editElement(newStatement as StatementHeader);
    },
  });

  // toggle comment statement
  const toggleCommentedCurrent = provideGlobalAction({
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
    insertBeforeCurrent,
    insertAfterCurrent,
    insertChildCurrent,
    toggleCommentedCurrent,
  };
}
