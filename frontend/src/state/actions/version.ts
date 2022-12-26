import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useOperations, useOperationsStore } from "@/state/operations";
import { computed } from "vue";

export function useVersionActions() {
  const editor = useEditorState();
  const ops = useOperations();
  const opsState = useOperationsStore();

  const commit = provideGlobalAction({
    id: "version.commit",
    label: "Commit...",
    shortcuts: ["ctrl+s", "meta+s"],
    enabled: computed(() => editor.currentProjectVersionId != null && !opsState.hasInflightLike("version.commit")),
    apply: () => {
      const randomName = getRandomName();
      opsState.reset();
      return ops.version.commit(editor.currentProjectVersionId as string, randomName);
    },
  });

  return { commit };
}
