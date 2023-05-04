import { provideGlobalAction } from "@/state/actions";
import { useEditorState, type FileHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newFileId } from "@/state/operations/file";
import { computed } from "vue";

export function useFileActions() {
  const editor = useEditorState();
  const ops = useOperations();

  const create = provideGlobalAction({
    id: "file.new",
    label: "New file...",
    enabled: computed(() => editor.currentProjectVersionId != null && !editor.readonly),
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async (name = "") => {
      const fileId = newFileId();
      const create = ops.file.create(null, fileId, editor.currentProjectVersionId as string, name, name, null, false);
      const optimisticFile = { __typename: "File", id: fileId, name, path: name } as FileHeader;
      const optimisticEditor = editor.focusFile(optimisticFile);
      try {
        await create;
      } catch (e) {
        editor.closeEditor(optimisticEditor);
      }
    },
  });

  return { create };
}
