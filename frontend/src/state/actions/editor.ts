import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type Editor } from "@/state/bench";
import { computed } from "vue";

export function useEditorActions() {
  const bench = useBenchState();

  // jump around
  const focusNextEditorGroup = provideGlobalAction({
    id: "bench.focusNextEditorGroup",
    label: "Focus Next Editor Group",
    shortcuts: ["meta+shift+space"],
    enabled: computed(() => bench.focusedEditor != null),
    apply: () => {
      if (bench.focusedGroup == null) return;
      const currentGroup = bench.focusedGroup;
      const nextGroup = bench.nextGroup(currentGroup);
      if (nextGroup != null && nextGroup != currentGroup) {
        bench.focusGroup(nextGroup);
      }
    },
  });

  // move editor
  const moveEditorLeft = provideGlobalAction({
    id: "bench.moveEditorLeft",
    label: "Move Editor Left",
    shortcuts: ["ctrl+shift+left", "meta+shift+left"],
    enabled: computed(() => bench.focusedEditor != null),
    apply: () => bench.moveEditor(bench.focusedEditor as Editor, bench.left),
  });
  const moveEditorRight = provideGlobalAction({
    id: "bench.moveEditorRight",
    label: "Move Editor Right",
    shortcuts: ["ctrl+shift+right", "meta+shift+right"],
    enabled: computed(() => bench.focusedEditor != null),
    apply: () => bench.moveEditor(bench.focusedEditor as Editor, bench.right),
  });

  // close editor
  const closeEditor = provideGlobalAction({
    id: "bench.closeEditor",
    label: "Close Editor",
    shortcuts: ["alt+w", "ctrl+w", "meta+w"],
    enabled: computed(() => bench.focusedEditor != null),
    apply: () => bench.closeEditor(bench.focusedEditor as Editor),
  });

  // toggle debug mode
  const toggleDebugMode = provideGlobalAction({
    id: "bench.toggleDebugMode",
    label: "Toggle Debug Mode",
    shortcuts: ["ctrl+shift+d", "meta+shift+d"],
    enabled: computed(() => true),
    apply: () => {
      console.log("toggle debug mode");
      bench.debug = !bench.debug;
    },
  });

  return {
    focusNextEditorGroup,
    moveEditorLeft,
    moveEditorRight,
    closeEditor,
    toggleDebugMode,
  };
}
