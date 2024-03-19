import type { IconData, NodeReferenceData, TextData, ViewData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { spaceRegistry } from "@/system/space";
import { toaster } from "@/system/toast";
import type { FIlterPrefix as FilterPrefix } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { keytrap, type KeySignature, type ParsedKeySignature } from "@/utils/keymap";
import { log } from "@/utils/log";
import { onUnmountedStrict } from "@/utils/ref";
import { getCurrentInstance, shallowRef, triggerRef, watch, type Ref } from "vue";

// :OmnibarModes
export type OmnibarMode = "everywhere" | "actions" | "space" | "views" | "view" | "module" | "package" | "bench";
export const OMNIBAR_MODES: OmnibarMode[] = [
  "everywhere",
  "actions",
  "space",
  "views",
  "view",
  "module",
  "package",
  "bench",
];

export type ActionCategory = "space" | "user" | "common" | "view";
export const ACTION_CATEGORIES: ActionCategory[] = ["space", "user", "view"];
export type ActionBuiltinId =
  // space
  // (:OmnibarModes)
  | "space.open.omnibar.everywhere"
  | "space.open.omnibar.actions"
  | "space.open.omnibar.space"
  | "space.open.omnibar.views"
  | "space.open.omnibar.view"
  | "space.open.omnibar.module"
  | "space.open.omnibar.package"
  | "space.open.omnibar.bench"
  | "space.open.chat"
  | "space.open.inspector"
  | "space.open.library"
  | "space.open.explorer"
  | "space.open.outline"
  | "space.open.docs"
  | "space.open.logs"
  | "space.open.discord"
  // user
  | "user.signup"
  | "user.login"
  | "user.logout"
  // common
  | "common.edit.undo"
  | "common.edit.redo"
  | "common.edit.delete"
  | "common.edit.copy"
  | "common.edit.cut"
  | "common.edit.paste"
  | "common.edit.duplicate"
  | "common.navigate.up"
  | "common.navigate.down"
  | "common.navigate.left"
  | "common.navigate.right"
  | "common.navigate.pageUp"
  | "common.navigate.pageDown"
  | "common.select.all"
  | "common.select.up"
  | "common.select.down"
  | "common.select.left"
  | "common.select.right"
  | "common.select.clear"
  | "common.move.up"
  | "common.move.down"
  | "common.move.left"
  | "common.move.right"
  | "common.analyze.goToDefinition"
  | "common.analyze.findReferences"
  // view
  | "view.navigate.focusPreviousTab"
  | "view.navigate.focusNextTab"
  | "view.navigate.focusPreviousWindow"
  | "view.navigate.focusNextWindow"
  | "view.navigate.closeTab"
  | "view.navigate.closeOtherTabs"
  | "view.navigate.reopenClosedTab"
  | "view.navigate.closeWindow"
  | "view.navigate.closeOtherWindows"
  | "view.navigate.reopenClosedWindow"
  | "view.layout.splitHorizontal"
  | "view.layout.splitVertical";
export type ActionBuiltinCategory = FilterPrefix<ActionBuiltinId, string>;
export type ActionSource = { kind: "builtin"; id: ActionBuiltinId } | { kind: "block"; block: NodeReferenceData };
export type ActionCallable = (action: Action) => void | boolean | Promise<void> | Promise<boolean>;
export type ActionKind = "static" | "virtual";

export const ACTION_COMING_SOON: ActionCallable = (action: Action) =>
  toaster.debug({ title: "Coming soon", text: `"${action.title}" is not yet available.`, icon: action.icon });

/**
 * An Action that can be performed by the user in the space.
 * Actions can be declared and implemented in different places (e.g. for different behavior in various Views).
 *
 * TODO :Architecture: define Action as Struct so it can be provided by custom Views/...?
 *  provide actions by tagging runnable (no args) Blocks with Action?
 */
export type Action = {
  kind: ActionKind;
  id: string; // some unique identifier for the action
  icon?: IconData;
  title: string;
  text?: string | TextData;
  shortcuts?: KeySignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  enabled?: Ref<boolean>;
  source: ActionSource;
  category: string;
  action: ActionCallable;
  url?: string; // for external URLs
};

export const ACTIONS: Ref<Partial<Record<string, Action>>> = shallowRef({});
export const VIRTUAL_ACTIONS: Ref<Partial<Record<string, ActionImplementation>>> = shallowRef({}); // for virtual actions

type ActionIn = Pick<Action, "title" | "text" | "shortcuts" | "enabled" | "action" | "url"> & {
  id: ActionBuiltinId;
  icon?: string | IconData;
};
export type ActionDeclaration = Omit<ActionIn, "id" | "enabled" | "action">;
export type ActionMapDeclaration<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionDeclaration>;
export type ActionImplementation = Pick<Action, "enabled" | "action">;
export type ActionMapImplementation<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionImplementation>;
export type ActionContribution = ActionDeclaration & ActionImplementation;
export type ActionMapContribution<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionContribution>;

/** Adds an action directly to the map. */
export function addAction(kind: ActionKind, in_: ActionIn) {
  const action: Action = {
    kind,
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ name: in_.icon }) : in_.icon,
    source: { kind: "builtin", id: in_.id },
    id: in_.id,
    category: in_.id.split(".")[0],
  };
  if (ACTIONS.value[in_.id] != null && (!IS_DEBUG || getCurrentInstance() == null))
    // hot-reloading re-registers actions
    throw new Error(`action already exists: ${in_.id} (${in_} != ${ACTIONS.value[in_.id]})`);
  ACTIONS.value[in_.id as ActionBuiltinId] = action;
  triggerRef(ACTIONS);
}

/** Contributes actions globally (same implementation everywhere) */
export function contributeActionMap<T extends string>(map: Partial<ActionMapContribution<T>>) {
  Object.entries(map).forEach(([id, action]) =>
    addAction("static", { ...(action as ActionIn), id: id as ActionBuiltinId }),
  );
}

/** Declares actions to be implemented virtually. */
export function declareActionMap<T extends string>(map: Partial<ActionMapDeclaration<T>>) {
  Object.entries(map).forEach(([id, action]) =>
    addAction("virtual", { ...(action as ActionIn), id: id as ActionBuiltinId }),
  );
}

export function getVirtualActionKey(viewId: string, actionId: string) {
  return `${viewId}.${actionId}`;
}

/** Implements virtual actions in a certain component (view). */
export function implementActionMap<T extends string>(
  view: Ref<NodeReferenceData | null>,
  map: Partial<ActionMapImplementation<T>>,
) {
  const unregister = () => {
    if (view.value?.id == null) return;
    Object.keys(map).forEach((id) => {
      const key = getVirtualActionKey(view.value!.id!, id);
      delete VIRTUAL_ACTIONS.value[key];
    });
  };

  // register current actions into VIRTUAL_ACTIONS
  watch(
    view,
    (newView, oldView) => {
      if (newView?.id == oldView?.id) return; // same view
      if (view.value?.id == null) return; // doesn't have a view

      if (oldView?.id != null) unregister();

      Object.entries(map).forEach(([id, action]) => {
        const declared = ACTIONS.value[id as ActionBuiltinId];
        if (declared == null) throw new Error(`action was not declared: ${id}`);
        if (declared.kind !== "virtual") throw new Error(`action is not virtual: ${id}`);
        const key = getVirtualActionKey(view.value?.id!, id);
        if (VIRTUAL_ACTIONS.value[key] != null) throw new Error(`action already implemented: ${key}`);
        VIRTUAL_ACTIONS.value[key] = action as ActionImplementation;
      });
    },
    { immediate: true },
  );

  // unregister on unmount
  onUnmountedStrict(unregister);
}

export function getAction(id: ActionBuiltinId): Action {
  const action = ACTIONS.value[id];
  if (action == null) throw new Error(`no such action: ${id}`);
  return action;
}

export function runAction(id: ActionBuiltinId) {
  const action = getAction(id);
  action.action(action);
}

/** Triggers the bound action from a keyboard event. */
export function fireActionFromEvent(action: Action, e: KeyboardEvent): boolean {
  if (action.enabled != null && !action.enabled.value) return false;
  const view = action.kind == "virtual" && e.target != null ? spaceRegistry.findViewPtr(e.target as HTMLElement) : null;
  return fireAction(action, view);
}

/** Triggers the bound action from a given view (as starting point). */
export function fireAction(action: Action, view: NodeReferenceData | null) {
  if (action.enabled != null && !action.enabled.value) return false;
  if (action.kind == "static") {
    // static: just call callback directly
    log.debug("action.static", action.id);
    const ret = action.action(action);
    return typeof ret === "boolean" ? ret : true;
  } else if (action.kind == "virtual") {
    if (view == null) {
      log.debug("action.virtual.ignore", action.id, "no active view");
      return false;
    }
    // virtual: find closest component implementing that action
    const focusedViews = spaceRegistry.graph.getAncestors(view) as ViewData[];
    for (const view of focusedViews) {
      const key = getVirtualActionKey(view.id!, action.id);
      const impl = VIRTUAL_ACTIONS.value[key];
      if (impl != null && (impl.enabled == null || impl.enabled.value == true)) {
        log.debug("action.virtual", action.id, view.id);
        const ret = impl.action(action);
        return typeof ret === "boolean" ? ret : true;
      }
    }
    log.debug("action.virtual", action.id, view.id, "no implementing view", focusedViews);
    return false; // no action found
  } else {
    throw new Error(`unexpected action kind: ${action.kind}`);
  }
}

// register actions with keytrap
const bindings: Array<() => void> = [];
watch(ACTIONS, (actions) => {
  const actionsWithShortcuts = Object.values(actions)
    .map((a) => a as Action)
    .filter((a) => (a.shortcuts?.length ?? 0) > 0);
  log.debug(
    "action.updateKeymap",
    actionsWithShortcuts.map((a) => a.id),
  );
  bindings.forEach((unbind) => unbind());
  actionsWithShortcuts.forEach((action) => {
    const unbind = keytrap.bind(action.shortcuts!, (e) => fireActionFromEvent(action, e));
    bindings.push(unbind);
  });
});

// declare common actions
declareActionMap<"common">({
  // edit
  "common.edit.undo": {
    icon: "fas fa-arrow-turn-left",
    title: "Undo",
    text: "Undo the last action or edit",
    shortcuts: ["mod+z"],
  },
  "common.edit.redo": {
    icon: "fas fa-arrow-turn-right",
    title: "Redo",
    text: "Redo the last undone action or edit",
    shortcuts: ["mod+shift+z"],
  },
  "common.edit.delete": {
    icon: "fas fa-delete-left",
    title: "Delete",
    text: "Delete the current item",
    shortcuts: ["del", "backspace"],
  },
  "common.edit.copy": {
    icon: "fas fa-copy",
    title: "Copy",
    text: "Copy the current item",
    shortcuts: ["mod+c"],
  },
  "common.edit.cut": {
    icon: "fas fa-scissors",
    title: "Cut",
    text: "Cut the current item",
    shortcuts: ["mod+x"],
  },
  "common.edit.paste": {
    icon: "fas fa-paste",
    title: "Paste",
    text: "Paste the current item",
    shortcuts: ["mod+v"],
  },
  "common.edit.duplicate": {
    icon: "fas fa-clone",
    title: "Duplicate",
    text: "Duplicate the current item",
    shortcuts: ["mod+d"],
  },
  // navigate
  "common.navigate.up": {
    icon: "fas fa-arrow-up",
    title: "Up",
    text: "Navigate up",
    shortcuts: ["up"],
  },
  "common.navigate.down": {
    icon: "fas fa-arrow-down",
    title: "Down",
    text: "Navigate down",
    shortcuts: ["down"],
  },
  "common.navigate.left": {
    icon: "fas fa-arrow-left",
    title: "Left",
    text: "Navigate left",
    shortcuts: ["left"],
  },
  "common.navigate.right": {
    icon: "fas fa-arrow-right",
    title: "Right",
    text: "Navigate right",
    shortcuts: ["right"],
  },
  "common.navigate.pageUp": {
    icon: "fas fa-arrow-up-to-line",
    title: "Page Up",
    text: "Page up",
    shortcuts: ["pageup"],
  },
  "common.navigate.pageDown": {
    icon: "fas fa-arrow-down-to-line",
    title: "Page Down",
    text: "Page down",
    shortcuts: ["pagedown"],
  },
  // select
  "common.select.all": {
    icon: "fas fa-check-square",
    title: "Select All",
    text: "Select all items",
    shortcuts: ["mod+a"],
  },
  "common.select.up": {
    icon: "fas fa-arrow-up",
    title: "Select Up",
    text: "Select up",
    shortcuts: ["shift+up"],
  },
  "common.select.down": {
    icon: "fas fa-arrow-down",
    title: "Select Down",
    text: "Select down",
    shortcuts: ["shift+down"],
  },
  "common.select.left": {
    icon: "fas fa-arrow-left",
    title: "Select Left",
    text: "Select left",
    shortcuts: ["shift+left"],
  },
  "common.select.right": {
    icon: "fas fa-arrow-right",
    title: "Select Right",
    text: "Select right",
    shortcuts: ["shift+right"],
  },
  "common.select.clear": {
    icon: "fas fa-times",
    title: "Clear Selection",
    text: "Clear selection",
    shortcuts: ["esc"],
  },
  // move
  "common.move.up": {
    icon: "fas fa-arrow-up",
    title: "Move Up",
    text: "Move up",
    shortcuts: ["mod+up"],
  },
  "common.move.down": {
    icon: "fas fa-arrow-down",
    title: "Move Down",
    text: "Move down",
    shortcuts: ["mod+down"],
  },
  "common.move.left": {
    icon: "fas fa-arrow-left",
    title: "Move Left",
    text: "Move left",
    shortcuts: ["mod+left"],
  },
  "common.move.right": {
    icon: "fas fa-arrow-right",
    title: "Move Right",
    text: "Move right",
    shortcuts: ["mod+right"],
  },
  // analyze
  "common.analyze.goToDefinition": {
    icon: "fas fa-turn-down-right",
    title: "Go to Definition",
    text: "Go to definition",
  },
  "common.analyze.findReferences": {
    icon: "fas fa-turn-down-left",
    title: "Find References",
    text: "Find references",
  },
});
