import { provideGlobalAction } from "@/utils/actions";
import { useEditorState, type Editor } from "@/utils/editor";
import { computed } from "vue";

export function useEditorActions() {
  const editor = useEditorState();

  // move editor
  const moveEditorLeft = provideGlobalAction({
    id: "editor.moveEditorLeft",
    label: "Move Editor Left",
    shortcuts: ["ctrl+shift+left"],
    enabled: computed(() => editor.focusedEditor != null),
    apply: () => editor.moveEditor(editor.focusedEditor as Editor, editor.left),
  });
  const moveEditorRight = provideGlobalAction({
    id: "editor.moveEditorRight",
    label: "Move Editor Right",
    shortcuts: ["ctrl+shift+right"],
    enabled: computed(() => editor.focusedEditor != null),
    apply: () => editor.moveEditor(editor.focusedEditor as Editor, editor.right),
  });

  // close editor
  const closeEditor = provideGlobalAction({
    id: "editor.closeEditor",
    label: "Close Editor",
    shortcuts: ["alt+w"],
    enabled: computed(() => editor.focusedEditor != null),
    apply: () => editor.closeEditor(editor.focusedEditor as Editor),
  });

  return {
    moveEditorLeft,
    moveEditorRight,
    closeEditor,
  };
}
