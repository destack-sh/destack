import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { computed } from "vue";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();

  const create = provideGlobalAction({
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

  // delete
  const delete_ = provideGlobalAction({
    id: "file.delete",
    label: "Delete file",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => !editor.editingElement && editor.focusedElementTypename == "File"),
    apply: async () => {
      if (editor.focusedElementId) {
        await operations.file.delete(editor.focusedElementId);
      }
    },
  });

  return { create, delete: delete_ };
}