import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newFileId } from "@/state/operations/file";
import { newStatementId } from "@/state/operations/statement";
import { INTEGER_ZERO } from "@/utils/fractional";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();

  const create = provideGlobalAction({
    id: "file.new",
    label: "New file...",
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async (name: string = getRandomName()) => {
      const file = await operations.file.create(newFileId(), editor.currentProjectVersionId as string, name);
      editor.focusFile(file);
      // create default blank statement (likely we'll want more options later on like in Notion?)
      const statementId = newStatementId();
      const statementCreate = operations.statement.create(statementId, file.id, null, INTEGER_ZERO);
      editor.editElement({ __typename: "Statement", id: statementId } as any);
      await statementCreate;
      return file;
    },
  });

  return { create };
}
