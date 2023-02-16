import { getRandomName } from "@/composables/useRandomName";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState } from "@/state/editor";
import { useNotifications } from "@/state/notifications";
import { useOperations, useOperationsStore } from "@/state/operations";
import { computed } from "vue";

export function useVersionActions() {
  const editor = useEditorState();
  const ops = useOperations();
  const opsState = useOperationsStore();
  const notifications = useNotifications();

  const commit = provideGlobalAction({
    id: "version.commit",
    label: "Commit...",
    shortcuts: ["ctrl+k"],
    enabled: computed(
      () => editor.currentProjectVersionId != null && !opsState.hasInflightLike({ types: ["version.commit"] })
    ),
    apply: async () => {
      const randomName = getRandomName();
      opsState.reset();
      await ops.version.commit(editor.currentProjectVersionId as string, randomName);
      notifications.show({
        type: "commit.succes",
        kind: "success",
        message: `Committed version "${randomName}"`,
      });
    },
  });

  return { commit };
}
