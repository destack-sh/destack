import { useActionsStore } from "@/utils/actions";
import Mousetrap from "mousetrap";
import { onUnmounted, watchEffect } from "vue";

/**
 * Apply shortcuts for all available actions.
 * Should be called once in the root component.
 */
export function applyShortcuts() {
  const actions = useActionsStore();

  const boundShortcuts = [] as string[];
  watchEffect(() => {
    // remove previous shortcuts
    boundShortcuts.forEach((shortcut) => Mousetrap.unbind(shortcut));
    boundShortcuts.length = 0;

    // add new shortcuts
    actions.available.forEach((action) => {
      Mousetrap.bind(action.shortcuts, action.apply);
      boundShortcuts.push(...action.shortcuts);
    });
  });

  // clear when unmounted
  onUnmounted(() => {
    boundShortcuts.forEach((shortcut) => Mousetrap.unbind(shortcut));
  });
}
