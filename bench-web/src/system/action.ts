import { ViewType, type IconData, type NodeReferenceData, type TextData, LogLevel } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import { isDeveloperMode } from "@/system/client";
import { canvas, hasLocalBench, space } from "@/system/space";
import { toaster } from "@/system/toast";
import type { FilterPrefix as FilterPrefix } from "@/utils/functools";
import { DISCORD_URL, IS_DEBUG } from "@/utils/globals";
import { keytrap, type KeySignature } from "@/utils/keymap";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import type { ViewComponent } from "@/views";
import { clearCanvas, collectViewComponentsUp, setupDefaultCanvas, setupEmptyCanvas } from "@/views/canvas";
import {
  computed,
  getCurrentInstance,
  shallowRef,
  triggerRef,
  watch,
  type Ref,
  ref,
  type MaybeRef,
  toValue,
} from "vue";
import { graphConnections } from "@/system/connection";
import { flushTransactionBuffers } from "@/system/transaction";
import { generateRandomName } from "@/utils/naming";

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

export type ActionCategory = "bench" | "package" | "space" | "common" | "view" | "user" | "organization" | "developer";
export const ACTION_BUILTIN_IDS = [
  // bench
  "bench.go.goToBench",
  "bench.go.goToEnvironment",
  "bench.go.goToBranch",
  "bench.go.goToPackage",
  "bench.go.goToSpace",
  // package
  // ...
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
  "space.launch.notifications",
  "space.edit.create",
  "space.edit.resetCanvasDefault",
  // common
  "common.edit.undo",
  "common.edit.redo",
  "common.edit.delete",
  "common.edit.copy",
  "common.edit.cut",
  "common.edit.paste",
  "common.edit.duplicate",
  "common.edit.rename",
  "common.navigate.up",
  "common.navigate.down",
  "common.navigate.left",
  "common.navigate.right",
  "common.navigate.pageUp",
  "common.navigate.pageDown",
  "common.navigate.goBack",
  "common.navigate.goForward",
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
  "common.search.findInView",
  "common.search.replaceInView",
  "common.search.findInSpace",
  "common.search.replaceInSpace",
  "common.sense.focus",
  "common.sense.goToDefinition",
  "common.sense.findReferences",
  "common.sense.findImplementations",
  "common.session.run",
  "common.session.debug",
  "common.session.pause",
  "common.session.resume",
  "common.session.stop",
  "common.session.kill",
  "common.session.logs",
  // view
  "view.navigate.focusPreviousTab",
  "view.navigate.focusNextTab",
  "view.navigate.closeTab",
  "view.navigate.closeOtherTabs",
  "view.navigate.reopenClosedTab",
  "view.navigate.closeFrame",
  "view.navigate.focusPreviousFrame",
  "view.navigate.focusNextFrame",
  "view.navigate.reopenClosedFrame",
  "view.navigate.focusPreviousSplit",
  "view.navigate.focusNextSplit",
  "view.navigate.closeSplit",
  "view.navigate.reopenClosedSplit",
  "view.layout.splitUp",
  "view.layout.splitDown",
  "view.layout.splitLeft",
  "view.layout.splitRight",
  "view.layout.pinSplit",
  "view.layout.unpinSplit",
  // user
  "user.auth.signup",
  "user.auth.login",
  "user.auth.logout",
  "user.auth.logoutAll",
  "user.auth.activate",
  "user.misc.goToHome",
  "user.settings.editKeybindings",
  // organization
  "organization.create",
  // developer
  "developer.misc.toggleDeveloperMode",
  "developer.tx.pauseAllConnections",
  "developer.tx.resumeAllConnections",
  "developer.tx.pauseAllBuffers",
  "developer.tx.resumeAllBuffers",
  "developer.tx.flushBuffers",
  "developer.view.addMockView",
  "developer.view.resetCanvasEmpty",
  "developer.view.resetCanvasDefault",
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
  toaster.debug({ title: "Coming soon", text: `"${toValue(action.title)}" is not yet available.`, icon: action.icon });

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
  title: MaybeRef<string>;
  text: string | TextData;
  shortcuts?: KeySignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  enabled?: Ref<boolean>;
  source: ActionSource;
  category: string;
  subcategory?: string;
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

/** Adds an action (declaration or declaration+implementation) directly . */
export function addAction(kind: ActionKind, in_: ActionIn) {
  const idParts = in_.id.split(".").map((p) => toCasing(p, Casing.CAMEL));
  const action: Action = {
    kind,
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ name: in_.icon }) : in_.icon,
    source: { kind: "builtin", id: in_.id },
    id: in_.id,
    category: idParts[0],
    subcategory: idParts[1],
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
  if (action == null) throw new Error(`action is not declared: ${id}`);
  return action;
}

/** A simple OR filter for Actions */
export type ActionFilter = {
  // exact prefix match (lowercase)
  prefix?: string | string[];
  // wildcard match (lowercase, with '*' for variable length wildcard)
  wildcard?: string | string[];
  category?: ActionBuiltinCategory | ActionBuiltinCategory[];
};

/** Filters actions with a simple OR filter of clauses */
export function getActionsLike(like: ActionFilter): Action[] {
  const prefix = (like.prefix == null ? [] : Array.isArray(like.prefix) ? like.prefix : [like.prefix]).map((s) =>
    s.toLowerCase(),
  );
  const wildcard = (like.wildcard == null ? [] : Array.isArray(like.wildcard) ? like.wildcard : [like.wildcard]).map(
    (w) => new RegExp("^" + w.toLowerCase().replace(/\*/g, ".*") + "$"),
  );
  const category = like.category == null ? [] : Array.isArray(like.category) ? like.category : [like.category];
  const filter = (action: Action) => {
    // OR filter, any match is enough
    const idNorm = action.id.toLowerCase();
    if (prefix.some((p) => idNorm.startsWith(p))) return true;
    if (wildcard.some((w) => w.test(idNorm))) return true;
    if (category.length > 0 && !category.includes(action.category as ActionBuiltinCategory)) return false;
  };
  return DECLARED_ACTIONS.value.filter(filter);
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

/** Finds an ancestor element suppressing the given action */
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

export function fireActionById(id: ActionBuiltinId) {
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

/**
 * Checks whether the context implements the action.
 * NOTE: does not 'call' the action, so we can't know if the action is refused dynamically.
 */
export function isActionImplemented(action: Action, context: ViewComponent[]): boolean {
  if (action.kind == "static") return action.enabled == null || action.enabled.value == true;
  for (const view of context) {
    const impl = view.exposed?.actions?.[action.id];
    if (impl != null && (impl.enabled == null || impl.enabled.value == true)) return true;
  }
  return false;
}

/** Triggers the bound action from a given view (as starting point). */
export function fireAction(action: Action, viewsInOrder: ViewComponent[] | null = canvas.focusedViewComponents) {
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
        if (typeof ret != "boolean" || ret === true) return true;
        /** else: keep searching up */
      }
    }
    log.debug("action.virtual", action.id, "no implementing view", viewsInOrder);
    toaster.debug({
      title: `${toValue(action.title)} is unavailable`,
      text: `No active view supports ${action.id}.`,
    });
    return false; // no action found
  } else {
    throw new Error(`unexpected action kind: ${action.kind}`);
  }
}

// track implemented actions & maintain keybindings
// NOTE: implemented actions may contain disabled actions, we filter those at a later step to avoid updating this too often
//  (we evaluate the actual action to call only when firing the callback anyway)
const IMPLEMENTED_ACTIONS_BY_ID: Ref<Record<string, Action>> = shallowRef({});
export const IMPLEMENTED_ACTIONS: Readonly<Ref<Action[]>> = computed(() =>
  Object.values(IMPLEMENTED_ACTIONS_BY_ID.value),
);
watch(
  [DECLARED_ACTIONS, canvas.focusedViewComponentsById],
  () => {
    const implemented: Record<string, Action> = {};

    // static actions
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
  "common.edit.rename": {
    icon: "fas fa-pencil",
    title: "Rename",
    text: "Rename the current node",
    shortcuts: ["f2"],
  },
  // navigate
  "common.navigate.up": {
    icon: "fas fa-arrow-up",
    title: "Navigate Up",
    text: "Navigate up",
    shortcuts: ["up"],
  },
  "common.navigate.down": {
    icon: "fas fa-arrow-down",
    title: "Navigate Down",
    text: "Navigate down",
    shortcuts: ["down"],
  },
  "common.navigate.left": {
    icon: "fas fa-arrow-left",
    title: "Navigate Left",
    text: "Navigate left",
    shortcuts: ["left"],
  },
  "common.navigate.right": {
    icon: "fas fa-arrow-right",
    title: "Navigate Right",
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
  "common.navigate.goBack": {
    icon: "fas fa-arrow-turn-left",
    title: "Go Back",
    text: "Go back in view history",
    shortcuts: ["mod+shift+delete"],
  },
  "common.navigate.goForward": {
    icon: "fas fa-arrow-turn-right",
    title: "Go Forward",
    text: "Go forward in view history",
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
  // search
  "common.search.findInView": {
    icon: "fas fa-magnifying-glass",
    title: "Search in View",
    text: "Find in the current view",
    shortcuts: ["mod+f"],
  },
  "common.search.replaceInView": {
    icon: "fas fa-right-left",
    title: "Replace in View",
    text: "Replace in the current view",
    shortcuts: ["mod+r"],
  },
  "common.search.findInSpace": {
    icon: "fas fa-magnifying-glass",
    title: "Search in Space",
    text: "Find in the current space",
    shortcuts: ["mod+shift+f"],
  },
  "common.search.replaceInSpace": {
    icon: "fas fa-right-left",
    title: "Replace in Space",
    text: "Replace in the current space",
    shortcuts: ["mod+shift+r"],
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
  "common.session.logs": {
    icon: "fas fa-clipboard-list",
    title: "View Logs",
    text: "View the logs of the current run",
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
  "view.navigate.focusPreviousFrame": {
    icon: "fas fa-chevrons-left",
    title: "Focus Previous Frame",
    text: "Navigate to the previous frame",
    shortcuts: ["ctrl+mod+shift+space"],
  },
  "view.navigate.focusNextFrame": {
    icon: "fas fa-chevrons-right",
    title: "Focus Next Frame",
    text: "Navigate to the next frame",
    shortcuts: ["shift+mod+space"],
  },
  "view.navigate.closeFrame": {
    icon: "fas fa-xmark",
    title: "Close Frame",
    text: "Close the current frame",
    shortcuts: ["mod+shift+w"],
  },
  "view.navigate.reopenClosedFrame": {
    icon: "fas fa-arrow-rotate-left",
    title: "Reopen Closed Frame",
    text: "Reopen the last closed Frame",
    shortcuts: ["mod+shift+n"],
  },
  "view.navigate.focusPreviousSplit": {
    icon: "fas fa-chevron-up",
    title: "Focus Previous Split",
    text: "Navigate to the previous split",
    shortcuts: ["ctrl+shift+up", "ctrl+shift+left"],
  },
  "view.navigate.focusNextSplit": {
    icon: "fas fa-chevron-down",
    title: "Focus Next Split",
    text: "Navigate to the next split",
    shortcuts: ["ctrl+shift+down", "ctrl+shift+right"],
  },
  "view.navigate.closeSplit": {
    icon: "fas fa-xmark",
    title: "Close Split",
    text: "Close the current split",
  },
  // layout
  "view.layout.splitUp": {
    icon: "fas fa-reflect-vertical",
    title: "Split Up",
    text: "Split the current view vertically (new split above)",
  },
  "view.layout.splitDown": {
    icon: "fas fa-reflect-vertical",
    title: "Split Down",
    text: "Split the current view vertically (new split below)",
  },
  "view.layout.splitLeft": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Left",
    text: "Split the current view horizontally (new split left)",
  },
  "view.layout.splitRight": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Right",
    text: "Split the current view horizontally (new split right)",
  },
  "view.layout.pinSplit": {
    icon: "fas fa-lock",
    title: "Pin Split",
    text: "Pin the current split to an absolute size",
  },
  "view.layout.unpinSplit": {
    icon: "fas fa-unlock",
    title: "Unpin Split",
    text: "Unpin the current split back to relative size",
  },
});

// developer actions
contributeActionMap<"developer">({
  "developer.misc.toggleDeveloperMode": {
    icon: "fas fa-bug",
    title: computed(() => (isDeveloperMode.value ? "Disable Developer Mode" : "Enable Developer Mode")),
    text: "Developer Mode enables some advanced and some weird features.",
    action: () => {
      isDeveloperMode.value = !isDeveloperMode.value;
      toaster.info({
        key: "developer.toggleDeveloperMode",
        override: true,
        title: isDeveloperMode.value ? "Developer Mode Enabled" : "Developer Mode Disabled",
        text: isDeveloperMode.value ? "Welcome to the dark side." : "Back to the normal side.",
        icon: "fas fa-bug",
        actions: [
          {
            title: isDeveloperMode.value ? "Disable" : "Enable",
            action: () => {
              isDeveloperMode.value = !isDeveloperMode.value;
            },
          },
        ],
      });
    },
    shortcuts: ["alt+f12", "f12"],
  },
  "developer.tx.pauseAllConnections": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Pause All Connections",
    text: "Pause all active connections",
    action: () => graphConnections.value.filter((c) => !c.isPaused.value).forEach((c) => c.togglePaused()),
  },
  "developer.tx.resumeAllConnections": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Resume All Connections",
    text: "Resume all paused connections",
    action: () => graphConnections.value.filter((c) => c.isPaused.value).forEach((c) => c.togglePaused()),
  },
  "developer.tx.pauseAllBuffers": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Pause All Buffers",
    text: "Pause all active transaction buffers",
    action: () =>
      graphConnections.value
        .map((c) => c.txBuffer)
        .filter((b) => !b.isPaused.value)
        .forEach((b) => b.togglePaused()),
  },
  "developer.tx.resumeAllBuffers": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Resume All Buffers",
    text: "Resume all paused transaction buffers",
    action: () =>
      graphConnections.value
        .map((c) => c.txBuffer)
        .filter((b) => b.isPaused.value)
        .forEach((b) => b.togglePaused()),
  },
  "developer.tx.flushBuffers": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Flush Buffers",
    text: "Flush all transaction buffers",
    action: () => flushTransactionBuffers({ force: true }),
  },
  "developer.view.addMockView": {
    enabled: isDeveloperMode,
    icon: "fas fa-bug",
    title: "Add Mock View",
    text: "Adds a debug view to the current root",
    action: () => {
      const name = toCasing(generateRandomName(), Casing.CAMEL, true);
      canvas.addView({ type: ViewType.MOCK, name, title: name });
    },
  },
  "developer.view.resetCanvasEmpty": {
    enabled: computed(() => isDeveloperMode.value && space.value != null),
    icon: "fas fa-bug",
    title: "Reset Canvas (Empty)",
    text: "Clear the canvas and start blank",
    action: () => {
      const tx = canvas.txFactory();
      clearCanvas(tx, canvas.graph, space.value!);
      setupEmptyCanvas(tx, space.value!);
    },
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
    title: "View Library",
    text: "Get building blocks from the library",
    icon: "fas fa-books",
    action: ACTION_COMING_SOON,
  },
  "space.launch.explorer": {
    title: "View Explorer",
    text: "Explore nodes in the space",
    icon: "fas fa-compass",
    action: ACTION_COMING_SOON,
  },
  "space.launch.outline": {
    title: "View Outline",
    text: "View the outline of the space",
    icon: "fas fa-list-tree",
    action: ACTION_COMING_SOON,
  },
  "space.launch.docs": {
    title: "View Documentation",
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
  "space.launch.notifications": {
    title: "Open Notifications",
    text: "View your notifications",
    icon: "fas fa-envelope",
    enabled: ref(false),
    action: ACTION_COMING_SOON,
  },
  "space.launch.logs": {
    title: "View Logs",
    text: "View all Logs in the Space",
    icon: "fas fa-clipboard-list",
    action: ACTION_COMING_SOON,
  },
  // edit
  "space.edit.create": {
    title: "Create Space",
    text: "Create a new separate Space",
    icon: "fas fa-plus",
    action: ACTION_COMING_SOON,
  },
  "space.edit.resetCanvasDefault": {
    title: "Reset Canvas (Default)",
    text: "Reset the canvas to the default state",
    icon: "fas fa-bug",
    action: () => {
      setupDefaultCanvas(canvas.txFactory(), space.value!);
    },
  },
});

// bench actions
contributeActionMap<"bench">({
  "bench.go.goToBench": {
    title: "Switch Bench",
    text: "Open another Bench",
    icon: "fas fa-fort",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToBranch": {
    enabled: ref(false), // not yet implemented
    title: "Switch Branch",
    text: "Go to another Branch in this Bench",
    icon: "fas fa-code-branch",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToEnvironment": {
    enabled: ref(false), // not yet implemented
    title: "Switch Environment",
    text: "Go to another Environment in this Bench",
    icon: "fas fa-cloud",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToPackage": {
    enabled: hasLocalBench,
    title: "Switch Package",
    text: "Go to another Package in this Bench",
    icon: "fas fa-box",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToSpace": {
    enabled: hasLocalBench,
    title: "Switch Space",
    text: "Go to another Space of this Bench",
    icon: "fas fa-galaxy",
    action: ACTION_COMING_SOON,
  },
});
