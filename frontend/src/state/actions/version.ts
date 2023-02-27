import { useEditorState } from "@/state/editor";
import { useNotifications } from "@/state/notifications";
import { useOperations, useOperationsStore } from "@/state/operations";

export function useVersionActions() {
  const editor = useEditorState();
  const ops = useOperations();
  const opsState = useOperationsStore();
  const notifications = useNotifications();

  return {};
}
