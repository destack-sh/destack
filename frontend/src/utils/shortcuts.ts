import { useActionsStore } from "@/state/actions";
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
    console.warn("applyShortcuts should only be called once");
    return;
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
      Mousetrap.bind(action.shortcuts, () => {
        action.apply();
        return false;
      });
      boundShortcuts.push(...action.shortcuts);
    });
  });

  // supprres undo/redo actions if mousetrap-no-do class is present
  // this is a custom implementation of Mousetrap.stopCallback, see https://craig.is/killing/mice
  const suppressedUndoRedoShortcuts = ["ctrl+z", "ctrl+shift+z"];
  const suppressedTabShortcuts = ["tab", "shift+tab"];
  Mousetrap.prototype.stopCallback = function (e: any, element: HTMLElement, combo: string) {
    // If the element has the class "mousetrap-no-do", stop the callback for the specified shortcuts
    if ((" " + element.className + " ").indexOf(" mousetrap-no-do ") > -1) {
      if (suppressedUndoRedoShortcuts.includes(combo)) {
        return true;
      }
    }
    // If the element has the class "mousetrap-no-tab", stop the callback for the specified shortcuts
    if ((" " + element.className + " ").indexOf(" mousetrap-no-tab ") > -1) {
      if (suppressedTabShortcuts.includes(combo)) {
        return true;
      }
    }

    // If the element has the class "mousetrap" then no need to stop
    if ((" " + element.className + " ").indexOf(" mousetrap ") > -1) {
      return false;
    }

    // Stop for input, select, and textarea
    return (
      element.tagName == "INPUT" ||
      element.tagName == "SELECT" ||
      element.tagName == "TEXTAREA" ||
      (element.contentEditable && element.contentEditable == "true")
    );
  };
}
