import { StatementType } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useIntelliSense } from "@/state/intellisense";
import { useOperations } from "@/state/operations";
import { computed } from "vue";

export function useStatementActions() {
  const editor = useEditorState();
  const operations = useOperations();
  const sense = useIntelliSense();

  // actions for currently focused statement
  // maybe these actions should be provided by FileInterface? (which has the relevant state)
  const statement = computed(() => sense.registry.statementsById[editor.focusedElementId as string]);
  const index = computed(() => statement.value?.index ?? -1);
  const siblings = computed(
    () =>
      sense.registry.statementsByParentId[statement.value?.parent?.id ?? ""] ||
      sense.rootStatements(statement.value?.file.id ?? "")
  );
  const children = computed(() => sense.registry.statementsByParentId[statement.value.id] ?? []);
  const parent = computed(() => sense.registry.statementsById[statement.value?.parent?.id ?? ""]);
  const parentIndex = computed(() => parent.value?.index);
  const grandparent = computed(() => sense.registry.statementsById[statement.value?.parent?.id ?? ""]?.parent);
  const currentLocation = computed(() => ({
    fileId: statement.value.file.id,
    parentId: statement.value.parent?.id,
    index: index.value,
  }));

  // move statement
  const moveCurrentIn = provideGlobalAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    // we can only indent if there is a sibling above
    enabled: computed(() => !editor.editingElement && !!statement.value && index.value > 0),
    apply: async () => {
      // insert at end of previous sibling children (leave index undefined)
      const previousSibling = siblings.value[index.value - 1];
      await operations.statement.move(statement.value.id, currentLocation.value, {
        fileId: statement.value.file.id,
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
    apply: async () => {
      // insert after parent (leave index undefined)
      await operations.statement.move(statement.value.id, currentLocation.value, {
        fileId: statement.value.file.id,
        parentId: grandparent.value?.id,
        index: (parentIndex.value ?? 0) + 1,
      });
    },
  });

  const moveCurrentUp = provideGlobalAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(() => !!statement.value && (statement.value.parent != null || index.value > 0)),
    apply: async () => {
      // if index is > 0, move up within siblings
      if (index.value > 0) {
        await operations.statement.move(statement.value.id, currentLocation.value, {
          fileId: statement.value.file.id,
          parentId: statement.value.parent?.id,
          index: index.value - 1,
        });
      }
    },
  });

  const moveCurrentDown = provideGlobalAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => !!statement.value),
    apply: async () => {
      // if index is not last of its siblings, move down within parent
      if (index.value < siblings.value.length - 1) {
        await operations.statement.move(statement.value.id, currentLocation.value, {
          fileId: statement.value.file.id,
          parentId: statement.value.parent?.id,
          index: index.value + 1,
        });
      }
    },
  });

  const statementAbove = computed(() => {
    // if index is > 0, move up within siblings
    if (index.value > 0) {
      return siblings.value[index.value - 1];
    } else if (parent.value) {
      // if index is 0, move to parent
      return parent.value;
    }
    return null;
  });
  const statementBelow = computed(() => {
    // if index is not last of its siblings, move down within parent
    if (index.value < siblings.value.length - 1) {
      return siblings.value[index.value + 1];
    } else if (parent.value) {
      // if index is last, move to next sibling of parent
      const parentSiblings = parent.value.parent
        ? sense.registry.statementsByParentId[parent.value.parent.id]
        : sense.rootStatements(statement.value.file.id);
      if (parentSiblings != null && parentIndex.value != null && parentIndex.value < parentSiblings.length - 1) {
        return parentSiblings[parentIndex.value + 1];
      }
    }
    return null;
  });

  // move focus (if not editing element)
  const moveFocusUp = provideGlobalAction({
    id: "statement.moveFocusUp",
    label: "Move focus up",
    shortcuts: ["up"],
    enabled: computed(() => !editor.editingElement && statementAbove.value != null),
    apply: () => {
      if (statementAbove.value != null) {
        editor.focusElement(statementAbove.value);
      }
    },
  });
  const moveFocusDown = provideGlobalAction({
    id: "statement.moveFocusDown",
    label: "Move focus down",
    shortcuts: ["down"],
    enabled: computed(() => !editor.editingElement && statementBelow.value != null),
    apply: () => {
      if (statementBelow.value != null) {
        editor.focusElement(statementBelow.value);
      }
    },
  });
  // move focus in/out (if not editing element)
  const moveFocusIn = provideGlobalAction({
    id: "statement.moveFocusIn",
    label: "Move focus in",
    shortcuts: ["right"],
    enabled: computed(() => !editor.editingElement && !!statement.value && children.value.length > 0),
    apply: () => {
      editor.focusElement(children.value[0]);
    },
  });
  const moveFocusOut = provideGlobalAction({
    id: "statement.moveFocusOut",
    label: "Move focus out",
    shortcuts: ["left"],
    enabled: computed(() => !editor.editingElement && !!statement.value && !!statement.value.parent),
    apply: () => {
      editor.focusElement(parent.value);
    },
  });

  // start / stop editing current statement
  const editCurrent = provideGlobalAction({
    id: "statement.editCurrent",
    label: "Edit current statement",
    shortcuts: ["enter"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    apply: () => {
      editor.editElement(statement.value);
    },
  });
  const stopEditingCurrent = provideGlobalAction({
    id: "statement.stopEditingCurrent",
    label: "Stop editing current statement",
    shortcuts: ["escape"],
    enabled: computed(() => !!statement.value && editor.editingElement),
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
    apply: async () => {
      const current = statement.value;
      const newStatement = await operations.statement.create(
        current.file.id,
        current.parent?.id ?? null,
        current.index ?? 0,
        StatementType.Blank
      );
      editor.editElement(newStatement);
    },
  });
  const insertAfterCurrent = provideGlobalAction({
    id: "statement.insertBelowCurrent",
    label: "Insert statement below current",
    shortcuts: ["i", "b", "shift+enter", "plus"],
    enabled: computed(() => !!statement.value && !editor.editingElement),
    apply: async () => {
      const current = statement.value;
      const newStatement = await operations.statement.create(
        current.file.id,
        current.parent?.id ?? null,
        (current.index ?? 0) + 1,
        StatementType.Blank
      );
      editor.editElement(newStatement);
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
    toggleCommentCurrent: toggleCommentedCurrent,
  };
}
