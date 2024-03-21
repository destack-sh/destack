import type { IconData, NodeReferenceData, TextData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { canvas } from "@/system/space";
import { toaster } from "@/system/toast";
import type { FilterPrefix as FilterPrefix } from "@/utils/functools";
import { DISCORD_URL, IS_DEBUG } from "@/utils/globals";
import { keytrap, type KeySignature } from "@/utils/keymap";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import type { ViewComponent } from "@/views";
import { collectViewComponentsUp } from "@/views/canvas";
import { computed, getCurrentInstance, shallowRef, triggerRef, watch, type Ref } from "vue";

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

export type ActionCategory = "space" | "common" | "view" | "user";
export const ACTION_BUILTIN_IDS = [
  // space
  // (:OmnibarModes)
  "space.omnibar.everywhere",
  "space.omnibar.actions",
  "space.omnibar.space",
  "space.omnibar.views",
  "space.omnibar.view",
  "space.omnibar.module",
  "space.omnibar.package",
  "space.omnibar.bench",
  "space.launch.chat",
  "space.launch.inspector",
  "space.launch.library",
  "space.launch.explorer",
  "space.launch.outline",
  "space.launch.docs",
  "space.launch.logs",
  "space.launch.discord",
  // common
  "common.edit.undo",
  "common.edit.redo",
  "common.edit.delete",
  "common.edit.copy",
  "common.edit.cut",
  "common.edit.paste",
  "common.edit.duplicate",
  "common.navigate.up",
  "common.navigate.down",
  "common.navigate.left",
  "common.navigate.right",
  "common.navigate.pageUp",
  "common.navigate.pageDown",
  "common.select.all",
  "common.select.up",
  "common.select.down",
  "common.select.left",
  "common.select.right",
  "common.select.clear",
  "common.move.up",
  "common.move.down",
  "common.move.left",
  "common.move.right",
  "common.sense.focus",
  "common.sense.goToDefinition",
  "common.sense.findReferences",
  "common.sense.findImplementations",
  "common.sense.rename",
  "common.session.run",
  "common.session.debug",
  "common.session.pause",
  "common.session.resume",
  "common.session.stop",
  "common.session.kill",
  // view
  "view.navigate.focusPreviousTab",
  "view.navigate.focusNextTab",
  "view.navigate.focusPreviousWindow",
  "view.navigate.focusNextWindow",
  "view.navigate.closeTab",
  "view.navigate.closeOtherTabs",
  "view.navigate.reopenClosedTab",
  "view.navigate.closeWindow",
  "view.navigate.closeOtherWindows",
  "view.navigate.reopenClosedWindow",
  "view.layout.splitHorizontal",
  "view.layout.splitVertical",
  // user
  "user.signup",
  "user.login",
  "user.logout",
  "user.activate",
  "user.goHome",
  "organization.create",
  "bench.create",
] as const;
export const ACTION_BUILTIN_IDS_INDEX: Record<ActionBuiltinId, number> = ACTION_BUILTIN_IDS.reduce(
  (acc, id, idx) => ({ ...acc, [id]: idx }),
  {},
) as Record<ActionBuiltinId, number>;
export type ActionBuiltinId = (typeof ACTION_BUILTIN_IDS)[number];
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
  id: ActionBuiltinId;
  icon?: IconData;
  title: string;
  text?: string | TextData;
  shortcuts?: KeySignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  enabled?: Ref<boolean>;
  source: ActionSource;
  category: string;
  path: string;
  action: ActionCallable;
  url?: string; // for external URLs
};

export const DECLARED_ACTIONS_BY_ID: Ref<Partial<Record<string, Action>>> = shallowRef({});
export const DECLARED_ACTIONS: Ref<Action[]> = computed(() => Object.values(DECLARED_ACTIONS_BY_ID.value) as Action[]);

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
  let idParts = in_.id.split(".").map((p) => toCasing(p, Casing.CAMEL));
  if (idParts[0] == "common") idParts = idParts.slice(1);
  const action: Action = {
    kind,
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ name: in_.icon }) : in_.icon,
    source: { kind: "builtin", id: in_.id },
    id: in_.id,
    category: idParts[0],
    path: idParts.slice(0, -1).join(" / "),
  };
  if (DECLARED_ACTIONS_BY_ID.value[in_.id] != null && (!IS_DEBUG || getCurrentInstance() == null))
    // hot-reloading re-registers actions
    throw new Error(`action already exists: ${in_.id} (${in_} != ${DECLARED_ACTIONS_BY_ID.value[in_.id]})`);
  DECLARED_ACTIONS_BY_ID.value[in_.id as ActionBuiltinId] = action;
  triggerRef(DECLARED_ACTIONS_BY_ID);
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

export function getAction(id: ActionBuiltinId): Action {
  const action = DECLARED_ACTIONS_BY_ID.value[id];
  if (action == null) throw new Error(`no such action: ${id}`);
  return action;
}

/**
 * Actions can be suppressed in the DOM with data-suppress-actions='mask1,mask2,...'.
 * If not specified, we default to the  DEFAULT_SUPPRESSED_ACTIONS per tag.
 * Suppressions accumulate up the DOM tree, and for simplicity you cannot un-suppress an action.
 * The mask is simply a prefix match.
 */
export const DEFAULT_SUPPRESSED_ACTIONS: Record<string, string[]> = {
  input: ["common.navigate", "common.select", "common.move"],
  textarea: ["common.navigate", "common.select", "common.move"],
  contenteditable: ["common.navigate", "common.select", "common.move"],
};

function getActionSuppressor(id: ActionBuiltinId, el: HTMLElement): HTMLElement | null {
  while (el != null) {
    const elTag = el.tagName.toLowerCase();
    const suppress = el.getAttribute("data-suppress-actions");
    let masks: string[];
    if (suppress != null) masks = suppress.split(",");
    else if (DEFAULT_SUPPRESSED_ACTIONS[elTag] != null) masks = DEFAULT_SUPPRESSED_ACTIONS[elTag];
    else if (el.contentEditable == "true") masks = DEFAULT_SUPPRESSED_ACTIONS.contenteditable;
    else masks = [];

    if (masks.some((mask) => id.startsWith(mask))) return el;
    else el = el.parentElement!;
  }
  return null;
}

export function runAction(id: ActionBuiltinId) {
  const action = getAction(id);
  fireAction(action);
}

/** Triggers the bound action from a keyboard event. */
export function fireActionFromEvent(action: Action, e: KeyboardEvent): boolean {
  if (action.enabled != null && !action.enabled.value) {
    log.debug("action.disabled", action.id);
    return false;
  }
  const suppressor = getActionSuppressor(action.id, e.target as HTMLElement);
  if (suppressor) {
    log.debug("action.suppressed", action.id, suppressor);
    return false;
  }
  const chain = collectViewComponentsUp(e.target as HTMLElement);
  return fireAction(action, chain);
}

/** Triggers the bound action from a given view (as starting point). */
export function fireAction(action: Action, viewsInOrder?: ViewComponent[] | null) {
  if (action.enabled != null && !action.enabled.value) return false;
  if (action.kind == "static") {
    // static: just call callback directly
    log.info("action.static", action.id);
    const ret = action.action(action);
    return typeof ret === "boolean" ? ret : true;
  } else if (action.kind == "virtual") {
    // virtual: find first component implementing that action
    for (const view of viewsInOrder ?? []) {
      const impl = view.exposed?.actions?.[action.id];
      if (impl != null && (impl.enabled == null || impl.enabled.value == true)) {
        log.info("action.virtual", action.id);
        const ret = impl.action(action);
        return typeof ret === "boolean" ? ret : true;
      }
    }
    log.debug("action.virtual", action.id, "no implementing view", viewsInOrder);
    toaster.debug({ title: "Action not available", text: `No active view supports "${action.title}".` });
    return false; // no action found
  } else {
    throw new Error(`unexpected action kind: ${action.kind}`);
  }
}

// track implemented actions (and register with keytrap)
// NOTE: implemented actions may contain disabled actions, we filter those at a later step to avoid updating this too often
//  (we evaluate the actual action to call only when firing the callback anyway)
export const IMPLEMENTED_ACTIONS_BY_ID: Ref<Record<string, Action>> = shallowRef({});
export const IMPLEMENTED_ACTIONS: Ref<Action[]> = computed(() => Object.values(IMPLEMENTED_ACTIONS_BY_ID.value));
watch(
  [DECLARED_ACTIONS, canvas.focusedViewComponentsById],
  () => {
    const implemented: Record<string, Action> = {};

    // all static actions
    Object.values(DECLARED_ACTIONS.value)
      .filter((a) => a.kind == "static")
      .forEach((a) => (implemented[a.id] = a));

    // collect virtual actions bottom up
    for (const view of Object.values(canvas.focusedViewComponentsById.value)) {
      for (const [actionId, action] of Object.entries(view.exposed?.actions ?? {})) {
        if (implemented[actionId] != null) continue; // already declared (either static or by lower view)
        const declaration = DECLARED_ACTIONS_BY_ID.value[actionId];
        if (declaration == null) throw new Error(`no declaration for virtual action: ${actionId}`);
        implemented[actionId] = { ...declaration, ...action };
      }
    }

    // diff & update keytrap
    const oldImplemented = IMPLEMENTED_ACTIONS_BY_ID.value;
    const removedActions = Object.keys(oldImplemented).filter((id) => implemented[id] == null);
    const addedActions = Object.keys(implemented).filter((id) => oldImplemented[id] == null);
    removedActions.forEach((id) => keytrap.unbind(oldImplemented[id].shortcuts ?? []));
    addedActions.forEach((id) => {
      const action = implemented[id];
      if ((action.shortcuts?.length ?? 0) > 0)
        keytrap.bind(action.shortcuts!, (e) => fireActionFromEvent(action, e), id);
    });

    IMPLEMENTED_ACTIONS_BY_ID.value = implemented;
  },
  { immediate: true },
);

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
    icon: "fas fa-square-caret-up",
    title: "Select Up",
    text: "Select up",
    shortcuts: ["shift+up"],
  },
  "common.select.down": {
    icon: "fas fa-square-caret-down",
    title: "Select Down",
    text: "Select down",
    shortcuts: ["shift+down"],
  },
  "common.select.left": {
    icon: "fas fa-square-caret-left",
    title: "Select Left",
    text: "Select left",
    shortcuts: ["shift+left"],
  },
  "common.select.right": {
    icon: "fas fa-square-caret-right",
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
    icon: "fas fa-square-up",
    title: "Move Up",
    text: "Move up",
    shortcuts: ["mod+up"],
  },
  "common.move.down": {
    icon: "fas fa-square-down",
    title: "Move Down",
    text: "Move down",
    shortcuts: ["mod+down"],
  },
  "common.move.left": {
    icon: "fas fa-square-left",
    title: "Move Left",
    text: "Move left",
    shortcuts: ["mod+left", "shift+tab"],
  },
  "common.move.right": {
    icon: "fas fa-square-right",
    title: "Move Right",
    text: "Move right",
    shortcuts: ["mod+right", "tab"],
  },
  // sense
  "common.sense.goToDefinition": {
    icon: "fas fa-turn-down-right",
    title: "Go to Definition",
    text: "Go to definition of the current node",
    shortcuts: ["mod+b"],
  },
  "common.sense.findReferences": {
    icon: "fas fa-turn-down-left",
    title: "Find References",
    text: "Find references of the current node",
    shortcuts: ["mod+shift+b"],
  },
  "common.sense.findImplementations": {
    icon: "fas fa-turn-down-left",
    title: "Find Implementations",
    text: "Find implementations of the current node",
  },
  "common.sense.focus": {
    icon: "fas fa-magnifying-glass-plus",
    title: "Focus",
    text: "Focus on the current node in a new view",
    shortcuts: ["mod+enter"],
  },
  "common.sense.rename": {
    icon: "fas fa-pencil",
    title: "Rename",
    text: "Rename the current node",
    shortcuts: ["f2"],
  },
  // session
  "common.session.run": {
    icon: "fas fa-play",
    title: "Run",
    text: "Run the current node",
    shortcuts: ["ctrl+r", "f5"],
  },
  "common.session.debug": {
    icon: "fas fa-bug",
    title: "Debug",
    text: "Debug the current node",
    shortcuts: ["ctrl+d", "f6"],
  },
  "common.session.pause": {
    icon: "fas fa-pause",
    title: "Pause",
    text: "Pause the current node",
  },
  "common.session.resume": {
    icon: "fas fa-play",
    title: "Resume",
    text: "Resume the current node",
  },
  "common.session.stop": {
    icon: "fas fa-stop",
    title: "Stop",
    text: "Stop the current node",
  },
  "common.session.kill": {
    icon: "fas fa-skull",
    title: "Kill",
    text: "Kill the current node",
  },
});

// space actions
contributeActionMap<"space">({
  "space.launch.inspector": {
    title: "Inspect Node",
    text: "Open the Inspector View",
    icon: "fas fa-eye-dropper",
    action: ACTION_COMING_SOON,
  },
  "space.launch.library": {
    title: "Open Library",
    text: "Get building blocks from the library",
    icon: "fas fa-books",
    action: ACTION_COMING_SOON,
  },
  "space.launch.docs": {
    title: "Read the Docs",
    text: "Get help from our examples and guides",
    icon: "fas fa-book-open",
    action: ACTION_COMING_SOON,
  },
  "space.launch.discord": {
    title: "Discuss on Discord",
    text: "Join the community on Discord",
    icon: "fab fa-discord",
    url: DISCORD_URL,
    action: () => {
      // open in new tab
      window.open(DISCORD_URL, "_blank");
    },
  },
});

// view actions
declareActionMap<"view">({
  // navigate
  "view.navigate.focusPreviousTab": {
    icon: "fas fa-chevron-left",
    title: "Focus Previous Tab",
    text: "Navigate to the previous tab",
    shortcuts: ["alt+shift+tab", "ctrl+shift+tab"],
  },
  "view.navigate.focusNextTab": {
    icon: "fas fa-chevron-right",
    title: "Focus Next Tab",
    text: "Navigate to the next tab",
    shortcuts: ["alt+tab", "ctrl+tab"],
  },
  "view.navigate.focusPreviousWindow": {
    icon: "fas fa-chevrons-left",
    title: "Focus Previous Window",
    text: "Navigate to the previous window",
    shortcuts: ["ctrl+mod+shift+space"],
  },
  "view.navigate.focusNextWindow": {
    icon: "fas fa-chevrons-right",
    title: "Focus Next Window",
    text: "Navigate to the next window",
    shortcuts: ["shift+mod+space"],
  },
  "view.navigate.closeTab": {
    icon: "fas fa-xmark",
    title: "Close Tab",
    text: "Close the current tab",
    shortcuts: ["mod+w", "ctrl+w"],
  },
  "view.navigate.closeOtherTabs": {
    icon: "fas fa-xmark",
    title: "Close Other Tabs",
    text: "Close all other tabs",
  },
  "view.navigate.reopenClosedTab": {
    icon: "fas fa-arrow-rotate-left",
    title: "Reopen Closed Tab",
    text: "Reopen the last closed tab",
    shortcuts: ["mod+shift+t", "ctrl+shift+t"],
  },
  "view.navigate.closeWindow": {
    icon: "fas fa-xmark",
    title: "Close Window",
    text: "Close the current window",
    shortcuts: ["mod+shift+w"],
  },
  "view.navigate.reopenClosedWindow": {
    icon: "fas fa-arrow-rotate-left",
    title: "Reopen Closed Window",
    text: "Reopen the last closed window",
    shortcuts: ["mod+shift+n"],
  },
  // layout
  "view.layout.splitVertical": {
    icon: "fas fa-reflect-vertical",
    title: "Split Vertical",
    text: "Split the current window vertically",
  },
  "view.layout.splitHorizontal": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Horizontal",
    text: "Split the current window horizontally",
  },
});
