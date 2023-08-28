import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type FileHeader } from "@/state/bench";
import { newNodeIdentity, type NodeBase } from "@/state/module";
import { useOperations } from "@/state/operations";
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
      const identity = newNodeIdentity(bench.projectVersionId as string, "File");
      const create = ops.file.create(null, identity.id, identity.ck, bench.projectVersionId as string, name, null);
      const optimisticFile = { __typename: "File", id: identity.id, name } as FileHeader;
      const optimisticEditor = bench.focusFile(optimisticFile as NodeBase);
      try {
        await create;
      } catch (e) {
        bench.closePanel(optimisticEditor);
      }
    },
  });

  return { create };
}
