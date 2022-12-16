import { provideGlobalAction } from "@/utils/actions";
import { useEditorState } from "@/utils/editor";
import { useIntelliSense } from "@/utils/intellisense";
import { useOperations } from "@/utils/operations";
import { computed } from "vue";

export function useStatementActions() {
  const editor = useEditorState();
  const operations = useOperations();
  const sense = useIntelliSense();

  // actions for currently focused statement
  // maybe these actions should be provided by FileInterface? (which has the relevant state)
  const statement = computed(() => sense.statementsById[editor.focusedElementId as string]);
  const index = computed(() => statement.value?.index ?? -1);
  const siblings = computed(
    () =>
      sense.statementsByParentId[statement.value?.parent?.id ?? ""] ||
      sense.rootStatements(statement.value?.file.id ?? "")
  );
  const parentIndex = computed(() => sense.statementsById[statement.value?.parent?.id ?? ""]?.index);
  const grandparent = computed(() => sense.statementsById[statement.value?.parent?.id ?? ""]?.parent);
  const currentLocation = computed(() => ({
    fileId: statement.value.file.id,
    parentId: statement.value.parent?.id,
    index: index.value,
  }));

  const moveIn = provideGlobalAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    // we can only indent if there is a sibling above
    enabled: computed(() => !!statement.value && index.value > 0),
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

  const moveOut = provideGlobalAction({
    id: "statement.moveCurrentOut",
    label: "Move statement out",
    shortcuts: ["shift+tab"],
    // we can only outdent if there is a parent
    enabled: computed(() => !!statement.value && !!statement.value.parent),
    apply: async () => {
      // insert after parent (leave index undefined)
      await operations.statement.move(statement.value.id, currentLocation.value, {
        fileId: statement.value.file.id,
        parentId: grandparent.value?.id,
        index: (parentIndex.value ?? 0) + 1,
      });
    },
  });

  const moveUp = provideGlobalAction({
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

  const moveDown = provideGlobalAction({
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

  return { moveIn, moveOut, moveUp, moveDown };
}
