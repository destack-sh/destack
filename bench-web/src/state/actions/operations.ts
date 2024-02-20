import { provideGlobalAction } from "@/state/actions";
import { useOperationsStore } from "@/state/operations";
import { toRef } from "vue";

export function useOperationsActions() {
  const operations = useOperationsStore();

  // undo & redo
  const undo = provideGlobalAction({
    id: "operations.undo",
    label: "Undo",
    shortcuts: ["ctrl+z", "meta+z"],
    enabled: toRef(operations, "canUndo"),
    apply: () => operations.undo(),
  });
  const redo = provideGlobalAction({
    id: "operations.redo",
    label: "Redo",
    shortcuts: ["ctrl+shift+z", "meta+shift+z"],
    enabled: toRef(operations, "canRedo"),
    apply: () => operations.redo(),
  });

  return { undo, redo };
}
