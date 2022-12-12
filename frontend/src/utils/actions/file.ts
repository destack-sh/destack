import { getRandomName } from "@/composables/useRandomName";
import { provideSharedAction } from "@/utils/actions";
import { useEditorState } from "@/utils/editor";
import { useOperations } from "@/utils/operations";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();

  const create = provideSharedAction({
    id: "file.new",
    label: "New file...",
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async () => {
      const randomName = getRandomName();
      const file = await operations.file.create(editor.currentProjectVersionId as string, randomName);
      editor.focusFile(file);
      editor.focusElement(file);
    },
  });

  return { create };
}
