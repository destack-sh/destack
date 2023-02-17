import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState, type FileHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newFileId } from "@/state/operations/file";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();

  const create = provideGlobalAction({
    id: "file.new",
    label: "New file...",
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async (name: string = getRandomName()) => {
      const fileId = newFileId();
      const create = operations.file.create(fileId, editor.currentProjectVersionId as string, name, name, null, false);
      editor.focusFile({ __typename: "File", id: fileId, name, path: name } as FileHeader);
      await create;
    },
  });

  return { create };
}
