import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type FileHeader } from "@/state/bench";
import { useOperations } from "@/state/operations";
import { newFileId } from "@/state/operations/file";
import { computed } from "vue";

export function useFileActions() {
  const bench = useBenchState();
  const ops = useOperations();

  const create = provideGlobalAction({
    id: "file.new",
    label: "New file...",
    enabled: computed(() => bench.projectVersionId != null && !bench.readonly),
    shortcuts: ["ctrl+n", "meta+n"],
    apply: async (name = "") => {
      const fileId = newFileId();
      const create = ops.file.create(null, fileId, bench.projectVersionId as string, name, null, false);
      const optimisticFile = { __typename: "File", id: fileId, name } as FileHeader;
      const optimisticEditor = bench.focusFile(optimisticFile);
      try {
        await create;
      } catch (e) {
        bench.closePanel(optimisticEditor);
      }
    },
  });

  return { create };
}
