import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newFileId } from "@/state/operations/file";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();

  const create = provideGlobalAction({
    id: "file.new",
    label: "New file...",
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async () => {
      const randomName = getRandomName();
      const file = await operations.file.create(newFileId(), editor.currentProjectVersionId as string, randomName);
      editor.focusFile(file);
      editor.focusElement(file);
    },
  });

  return { create };
}
