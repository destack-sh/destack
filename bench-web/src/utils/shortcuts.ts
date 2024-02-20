import { useActionsStore } from "@/state/actions";
import Mousetrap from "mousetrap";
import { watch } from "vue";

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

  const actionsIndex = useActionsStore();

  const lastBoundShortcuts = [] as string[];
  const boundShortcuts = [] as string[];
  watch(
    () => actionsIndex.all,
    () => {
      if (
        lastBoundShortcuts.length === actionsIndex.all.length &&
        lastBoundShortcuts.every((s, i) => s === actionsIndex.all[i].id)
      ) {
        // no change (unfortunately can't just use actionsIndex.all without deep watch)
        return;
      }

      // remove previous shortcuts
      boundShortcuts.forEach((shortcut) => Mousetrap.unbind(shortcut));
      boundShortcuts.length = 0;

      // group actions by shortcut to test/evaluate and fire together
      const actionsByShortcut: Record<string, string[]> = {};
      actionsIndex.all.forEach((action) => {
        action.shortcuts.forEach((shortcut) => {
          if (actionsByShortcut[shortcut] == null) {
            actionsByShortcut[shortcut] = [];
          }
          actionsByShortcut[shortcut].push(action.id);
        });
      });

      // bind shortcuts
      Object.entries(actionsByShortcut).forEach(([shortcut, actions]) => {
        Mousetrap.bind(shortcut, (e) => {
          const enabledActions = actionsIndex.all.filter((a) => a.enabled && actions.includes(a.id));
          enabledActions.forEach((action) => action.apply());
          return enabledActions.length == 0;
        });
        boundShortcuts.push(shortcut);
      });

      lastBoundShortcuts.length = 0;
      lastBoundShortcuts.push(...actionsIndex.all.map((a) => a.id));
    },
    { immediate: true, deep: true }
  );

  // suppress undo/redo actions if mousetrap-no-do class is present
  // this is a custom implementation of Mousetrap.stopCallback, see https://craig.is/killing/mice
  const suppressedUndoRedoShortcuts = ["ctrl+z", "ctrl+shift+z"];
  const suppressedTabShortcuts = ["tab", "shift+tab"];
  Mousetrap.prototype.stopCallback = function (e: any, element: HTMLElement, combo: string) {
    // If the element has the class "mousetrap-ignore", stop the callback
    if ((" " + element.className + " ").indexOf(" mousetrap-ignore ") > -1) {
      return true;
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
      (element.contentEditable && (element.contentEditable == "true" || element.contentEditable == "plaintext-only"))
    );
  };
}
