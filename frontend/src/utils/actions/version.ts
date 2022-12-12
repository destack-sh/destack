import { getRandomName } from "@/composables/useRandomName";
import { provideSharedAction } from "@/utils/actions";
import { useEditorState } from "@/utils/editor";
import { useOperations } from "@/utils/operations";
import { computed } from "vue";

export function useVersionActions() {
  const editor = useEditorState();
  const ops = useOperations();

  const commit = provideSharedAction({
    id: "version.commit",
    label: "Commit...",
    shortcuts: ["ctrl+s", "meta+s"],
    enabled: computed(() => editor.currentProjectVersionId != null),
    apply: () => {
      const randomName = getRandomName();
      return ops.version.commit(editor.currentProjectVersionId as string, randomName);
    },
  });

  return { commit };
}
