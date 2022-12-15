import { provideGlobalAction } from "@/utils/actions";
import { useEditorState } from "@/utils/editor";
import { useIntelliSense } from "@/utils/intellisense";
import { useOperations } from "@/utils/operations";
import { computed } from "vue";

export function useStatementActions() {
  const editor = useEditorState();
  const operations = useOperations();
  const sense = useIntelliSense();

  const focusedStatement = computed(() => sense.statementsById[editor.focusedElementId as string]);
  const focusedStatementIndex = computed(() => focusedStatement.value?.index ?? -1);
  const siblings = computed(() => sense.statementsByParentId[focusedStatement.value?.parent?.id ?? ""]);

  const moveIn = provideGlobalAction({
    id: "statement.moveCurrentIn",
    label: "Move statement in",
    shortcuts: ["tab"],
    enabled: computed(() => !!focusedStatement.value),
    apply: async () => {
      console.log("move statement in");
    },
  });

  const moveOut = provideGlobalAction({
    id: "statement.moveCurrentOut",
    label: "Move statement out",
    shortcuts: ["shift+tab"],
    enabled: computed(() => !!focusedStatement.value),
    apply: async () => {
      console.log("move statement out");
    },
  });

  const moveUp = provideGlobalAction({
    id: "statement.moveCurrentUp",
    label: "Move statement up",
    shortcuts: ["alt+up", "meta+up"],
    enabled: computed(
      () => !!focusedStatement.value && (focusedStatement.value.parent != null || focusedStatementIndex.value > 0)
    ),
    apply: async () => {
      const statement = focusedStatement.value;
      const index = focusedStatementIndex.value;
      // if index is > 0, move up within parent
      if (focusedStatementIndex.value > 0) {
        await operations.statement.move(
          statement.id,
          { fileId: statement.file.id, parentId: statement.parent?.id, index: index },
          { fileId: statement.file.id, parentId: statement.parent?.id, index: index - 1 }
        );
      }
    },
  });

  const moveDown = provideGlobalAction({
    id: "statement.moveCurrentDown",
    label: "Move statement down",
    shortcuts: ["alt+down", "meta+down"],
    enabled: computed(() => !!focusedStatement.value),
    apply: async () => {
      const statement = focusedStatement.value;
      const index = focusedStatementIndex.value;
      // if index is < parent.children.length, move down within parent
      if (focusedStatementIndex.value < siblings.value.length) {
        await operations.statement.move(
          statement.id,
          { fileId: statement.file.id, parentId: statement.parent?.id, index: index },
          { fileId: statement.file.id, parentId: statement.parent?.id, index: index + 1 }
        );
      }
    },
  });

  return { moveIn, moveOut, moveUp, moveDown };
}
