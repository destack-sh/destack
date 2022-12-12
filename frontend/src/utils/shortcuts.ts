import { useActionsStore } from "@/utils/actions";
import Mousetrap from "mousetrap";
import { watchEffect } from "vue";

/**
 * Apply shortcuts for all available actions.
 * Should be called once in the root component.
 */
let applyShortcutsCalled = false;
export function applyShortcuts() {
  // error if called multiple times
  if (applyShortcutsCalled) {
    throw new Error("applyShortcuts should only be called once");
  }
  applyShortcutsCalled = true;

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
}
