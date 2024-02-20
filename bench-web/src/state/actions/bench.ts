import { provideGlobalAction } from "@/state/actions";
import { useBenchState, type Panel } from "@/state/bench";
import { computed } from "vue";

export function useBenchActions() {
  const bench = useBenchState();

  // jump around
  const focusNextPanelGroup = provideGlobalAction({
    id: "bench.focusNextPanelGroup",
    label: "Focus Next Panel Group",
    shortcuts: ["meta+shift+space"],
    enabled: computed(() => bench.focusedPanel != null),
    apply: () => {
      if (bench.focusedGroup == null) return;
      const currentGroup = bench.focusedGroup;
      const nextGroup = bench.nextGroup(currentGroup);
      if (nextGroup != null && nextGroup != currentGroup) {
        bench.focusGroup(nextGroup);
      }
    },
  });

  // move panel
  const movePanelLeft = provideGlobalAction({
    id: "bench.movePanelLeft",
    label: "Move Panel Left",
    shortcuts: ["ctrl+shift+left", "meta+shift+left"],
    enabled: computed(() => bench.focusedPanel != null),
    apply: () => bench.movePanel(bench.focusedPanel as Panel, bench.left),
  });
  const movePanelRight = provideGlobalAction({
    id: "bench.movePanelRight",
    label: "Move Panel Right",
    shortcuts: ["ctrl+shift+right", "meta+shift+right"],
    enabled: computed(() => bench.focusedPanel != null),
    apply: () => bench.movePanel(bench.focusedPanel as Panel, bench.right),
  });

  // close panel
  const closePanel = provideGlobalAction({
    id: "bench.closePanel",
    label: "Close Panel",
    shortcuts: ["alt+w", "ctrl+w", "meta+w"],
    enabled: computed(() => bench.focusedPanel != null),
    apply: () => bench.closePanel(bench.focusedPanel as Panel),
  });

  // reopen last closed panel
  const reopenLastClosedPanel = provideGlobalAction({
    id: "bench.reopenLastClosedPanel",
    label: "Reopen Last Closed Panel",
    shortcuts: ["ctrl+shift+t", "meta+shift+t", "alt+shift+t"],
    enabled: computed(() => bench.recentlyClosedPanels.length > 0),
    apply: () => bench.reopenLastClosedPanel({ focus: true }),
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
    focusNextPanelGroup,
    movePanelLeft,
    movePanelRight,
    closePanel,
    reopenLastClosedPanel,
    toggleDebugMode,
  };
}
