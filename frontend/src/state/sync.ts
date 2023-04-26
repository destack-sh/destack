import { useEditorState } from "@/state/editor";
import { createSharedComposable } from "@vueuse/core";
import { toRef } from "vue";

function _useProjectSync() {
  const editor = useEditorState();
  const projectVersionId = toRef(editor, "currentProjectVersionId");
  // TODO @Incomplete: implement basic sync
}

export const useProjectSync = createSharedComposable(_useProjectSync);
