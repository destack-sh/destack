import { getRandomName } from "@/composables/useRandomName";
import { FileType } from "@/gql/graphql";
import { provideGlobalAction } from "@/utils/actions";
import { useEditorState } from "@/utils/editor";
import { useIntelliSense } from "@/utils/intellisense";
import { useOperations } from "@/utils/operations";
import { computed } from "vue";

export function useFileActions() {
  const editor = useEditorState();
  const operations = useOperations();
  const sense = useIntelliSense();

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

  // actions for currently focused file (as element)
  const file = computed(() => sense.filesById[editor.focusedElementId as string]);

  // delete
  const delete_ = provideGlobalAction({
    id: "file.delete",
    label: "Delete file",
    shortcuts: ["backspace", "delete"],
    enabled: computed(() => !editor.editingElement && !!file.value && file.value?.type == FileType.Instruct),
    apply: async () => {
      await operations.file.delete(file.value.id);
    },
  });

  return { create, delete: delete_ };
}
