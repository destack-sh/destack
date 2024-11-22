import { getEditStack } from "@/language/edit";
import { getAllTransactionBuffers } from "@/language/transaction";
import {
  TreeViewPreset,
  ViewType,
  type AnyNodeData,
  type IconData,
  type NodeReferenceData,
  type TextData,
} from "@/proto/wire";
import { type AnyNodeReferenceData } from "@/proto/wiring";
import { isDeveloperMode } from "@/system/client";
import { canvas, hasLocalBench, pkg, space } from "@/system/space";
import { makeIcon } from "@/ui/icon";
import { keytrap, type KeySignature } from "@/ui/keymap";
import { clearSpace, createDesktopDefaultSpace } from "@/ui/space";
import { toaster } from "@/ui/toast";
import { collectViewComponentsUp } from "@/ui/view";
import { type FilterPrefix } from "@/utils/functools";
import { DISCORD_URL, IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { generateRandomName } from "@/utils/naming";
import { Casing, toCasing } from "@/utils/string";
import type { ViewComponent } from "@/views/common";
import { useKeyModifier } from "@vueuse/core";
import {
  computed,
  getCurrentInstance,
  ref,
  shallowRef,
  toValue,
  triggerRef,
  watch,
  type MaybeRef,
  type Ref,
} from "vue";

export const IS_IN_ALT_MODE = useKeyModifier("Alt");

// :OmnibarModes
export const OMNIBAR_MODES = ["everywhere", "actions", "space", "views", "view"];
export type OmnibarMode = (typeof OMNIBAR_MODES)[number];

export const ACTION_BUILTIN_IDS = [
  // space
  // (:OmnibarModes)
  "space.omnibar.everywhere",
  "space.omnibar.actions",
  "space.omnibar.space",
  "space.omnibar.views",
  "space.omnibar.view",
  "space.launch.documentation",
  "space.launch.discord",
  "space.launch.notifications",
  "space.edit.inspect",
  "space.edit.create",
  "space.display.fullscreen",
  // common
  "common.create.above",
  "common.create.below",
  "common.create.block",
  "common.create.record",
  "common.create.step",
  "common.create.pipe",
  "common.create.trigger",
  "common.create.field",
  "common.create.field.input",
  "common.create.field.output",
  "common.history.undo",
  "common.history.redo",
  "common.edit.rename",
  "common.edit.move",
  "common.edit.copy",
  "common.edit.cut",
  "common.edit.paste",
  "common.edit.duplicate",
  "common.edit.delete",
  "common.navigate.open",
  "common.navigate.up",
  "common.navigate.down",
  "common.navigate.left",
  "common.navigate.right",
  "common.navigate.zoomIn",
  "common.navigate.zoomOut",
  "common.navigate.enter",
  "common.navigate.exit",
  "common.navigate.reset",
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
  "common.sense.goToDefinition",
  "common.sense.findReferences",
  "common.sense.findImplementations",
  // session
  "session.run.start",
  "session.run.pause",
  "session.run.resume",
  "session.run.kill",
  // type
  "type.edit.isList",
  "type.edit.isRequired",
  "type.edit.isSecret",
  // block
  // ...
  // database
  "database.column.sortAscending",
  "database.column.sortDescending",
  "database.column.filter",
  "database.column.wrap",
  "database.column.hide",
  // flow
  "flow.pipe.begin",
  // message
  "message.chat.reply",
  "message.edit.edit",
  "message.edit.pin",
  // text
  "text.format.bold",
  "text.format.italic",
  "text.format.strikethrough",
  "text.format.underline",
  "text.format.code",
  "text.edit.hardBreak",
  // code
  "code.edit.format",
  "code.edit.comment",
  // view
  "view.navigate.duplicateTab",
  "view.navigate.focusPreviousTab",
  "view.navigate.focusNextTab",
  "view.navigate.closeTab",
  "view.navigate.closeOtherTabs",
  "view.navigate.closeFrame",
  "view.navigate.focusPreviousFrame",
  "view.navigate.focusNextFrame",
  "view.navigate.focusPreviousSplit",
  "view.navigate.focusNextSplit",
  "view.navigate.closeSplit",
  "view.layout.splitUp",
  "view.layout.splitDown",
  "view.layout.splitLeft",
  "view.layout.splitRight",
  "view.layout.pinSplit",
  "view.space.resetDefault",
  "view.space.resetAdvanced",
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
  "developer.developerMode",
  "developer.tx.retryAllFailed",
  "developer.view.addEmptyView",
] as const;
export const ACTION_BUILTIN_IDS_INDEX: Record<ActionBuiltinId, number> = ACTION_BUILTIN_IDS.reduce(
  (acc, id, idx) => ({ ...acc, [id]: idx }),
  {},
) as Record<ActionBuiltinId, number>;
export type ActionBuiltinId = (typeof ACTION_BUILTIN_IDS)[number];
export type ActionBuiltinCategory = FilterPrefix<ActionBuiltinId, string>;
export type ActionSource = { kind: "builtin"; id: ActionBuiltinId } | { kind: "block"; block: NodeReferenceData };
export type ActionContext = {
  triggerNode?: AnyNodeData | AnyNodeReferenceData;
};
export type ActionCallable = (
  action: Action,
  context?: ActionContext,
) => void | boolean | Promise<void> | Promise<boolean>;
export type ActionKind = "static" | "virtual";
export const ACTION_TYPES = ["generic", "external-url", "toggle", "menu"] as const;
export type ActionType = (typeof ACTION_TYPES)[number];
export const ACTION_COMING_SOON: ActionCallable = (action: Action) =>
  toaster.debug({ title: "Coming soon", text: `"${toValue(action.title)}" is not yet available.`, icon: action.icon });

/**
 * An Action that can be performed by the user in the space.
 * Actions can be declared and implemented in different places (e.g. for different behavior in various Views).
 *
 * NOTE :Architecture: define Action as Struct so it can be provided by custom Views/...?
 *  provide actions by tagging runnable (no args) Blocks with Action?
 */
export type Action = {
  kind: ActionKind;
  type: ActionType;
  id: ActionBuiltinId;
  icon?: IconData;
  title: MaybeRef<string>;
  text: string | TextData;
  shortcuts?: KeySignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  isEnabled?: Ref<boolean> | (() => boolean);
  source: ActionSource;
  category: string;
  subcategory?: string;
  path: string;
  action: ActionCallable;
  // additional metadata
  url?: string; // for external URLs
  isChecked?: Ref<boolean> | ((action: Action, ctx?: ActionContext) => boolean); // for toggle actions
};

export const DECLARED_ACTIONS_BY_ID: Ref<Partial<Record<string, Action>>> = shallowRef({});
export const DECLARED_ACTIONS: Ref<Action[]> = computed(() => Object.values(DECLARED_ACTIONS_BY_ID.value) as Action[]);

type ActionIn = Pick<Action, "title" | "text" | "shortcuts" | "isEnabled" | "action" | "url" | "isChecked"> & {
  type?: ActionType;
  id: ActionBuiltinId;
  icon?: string | IconData;
};
export type ActionDeclaration = Omit<ActionIn, "id" | "enabled" | "action">;
export type ActionMapDeclaration<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionDeclaration>;
export type ActionImplementation = Pick<Action, "isEnabled" | "action" | "isChecked">;
export type ActionMapImplementation<T extends string> = Record<
  FilterPrefix<ActionBuiltinId, T>,
  ActionImplementation | ActionCallable
>;
export type ActionContribution = ActionDeclaration & ActionImplementation;
export type ActionMapContribution<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionContribution>;

/** Adds an action (declaration or declaration+implementation) directly . */
export function addAction(kind: ActionKind, in_: ActionIn) {
  const idParts = in_.id.split(".").map((p) => toCasing(p, Casing.CAMEL));
  const action: Action = {
    kind,
    type: "generic",
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ faName: in_.icon }) : in_.icon,
    source: { kind: "builtin", id: in_.id },
    id: in_.id,
    category: idParts[0],
    subcategory: idParts[1],
    path: idParts.slice(0, -1).join(" / "),
  };
  if (DECLARED_ACTIONS_BY_ID.value[in_.id] != null && (!IS_DEV || getCurrentInstance() == null))
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
  // wildcard match (lowercase, with '*' for variable length wildcard)
  wildcard?: string | string[];
};

/**
 * Filters actions with a simple OR filter of clauses.
 * The order of the input filter is preserved in order of matching.
 */
export function getActionsLike(like: ActionFilter | string[]): Action[] {
  like = Array.isArray(like) ? { wildcard: like } : like;
  const wildcards = (like.wildcard == null ? [] : Array.isArray(like.wildcard) ? like.wildcard : [like.wildcard]).map(
    (w) => new RegExp("^" + w.toLowerCase().replace(/\*/g, ".*") + "$"),
  );
  const matches: Record<string, Action> = {};
  for (const wildcard of wildcards) {
    for (const action of DECLARED_ACTIONS.value) {
      if (matches[action.id] != null) continue; // already matched (preserve order)
      const idNorm = action.id.toLowerCase();
      if (wildcard.test(idNorm)) matches[action.id] = action;
    }
  }
  return Object.values(matches);
}

/**
 * Actions can be suppressed in the DOM with data-suppress-actions='mask1,mask2,...'.
 * If not specified, we default to the  DEFAULT_SUPPRESSED_ACTIONS per tag.
 * Suppressions accumulate up the DOM tree, and for simplicity you cannot un-suppress an action.
 * The mask is simply a prefix match.
 */
export const DEFAULT_SUPPRESSED_ACTIONS: Record<string, string[]> = {
  input: ["common.edit", "common.navigate", "common.select", "common.move"],
  textarea: ["common.edit", "common.navigate", "common.select", "common.move"],
  contenteditable: ["common.edit", "common.navigate", "common.select", "common.move"],
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

    if (masks.some((mask) => id.startsWith(mask)))
      return el; // suppressed
    else el = el.parentElement!;
  }
  return null;
}

export function fireActionById(id: ActionBuiltinId, context?: ActionContext) {
  const action = getAction(id);
  fireAction(action, canvas.focusedViewComponents, context);
}

/** Triggers the bound action from a keyboard event. */
export function fireActionFromEvent(action: Action, e: KeyboardEvent, context?: ActionContext): boolean {
  if (action.isEnabled != null && !toValue(action.isEnabled)) {
    log.debug("action.disabled", action.id);
    return false;
  }
  const suppressor = getActionSuppressor(action.id, e.target as HTMLElement);
  if (suppressor) {
    log.trace("action.suppressed", action.id, suppressor);
    return false;
  } else {
    const localViewsInOrder = collectViewComponentsUp(e.target as HTMLElement);
    // NOTE: concat local views and focused view components so local ones are preferred, but all are available
    return fireAction(action, [...localViewsInOrder, ...canvas.focusedViewComponents], context);
  }
}

/**
 * Checks whether the context implements the action.
 * NOTE: does not 'call' the action, so we can't know if the action is refused dynamically.
 */
export function getImplementingAction(action: Action, context: ViewComponent[]): ActionImplementation | null {
  if (action.kind == "static") {
    if (action.isEnabled == null || toValue(action.isEnabled)) return action;
    else return null;
  } else if (action.kind == "virtual") {
    for (const view of context) {
      const impl = view.exposed?.actions?.[action.id];
      if (typeof impl == "function") return { action: impl };
      if (impl != null && (impl.isEnabled == null || toValue(impl.isEnabled) == true)) return impl;
    }
    return null;
  } else {
    throw new Error(`unexpected action kind: ${action.kind}`);
  }
}

/** Triggers the bound action from a given view (as starting point). */
export function fireAction(
  action: Action,
  viewsInOrder: ViewComponent[] | null = canvas.focusedViewComponents,
  context?: ActionContext,
) {
  if (action.isEnabled != null && !toValue(action.isEnabled)) return false;
  if (action.kind == "static") {
    // static: just call callback directly
    log.trace("action.static", action.id);
    const ret = action.action(action, context);
    return typeof ret === "boolean" ? ret : true;
  } else if (action.kind == "virtual") {
    // virtual: find first component implementing that action
    for (const view of viewsInOrder ?? []) {
      let impl = view.exposed?.actions?.[action.id];
      if (typeof impl == "function") impl = { action: impl };
      if (impl != null && (impl.isEnabled == null || toValue(impl.isEnabled) == true)) {
        log.trace("action.virtual", action.id);
        const ret = impl.action(action, context);
        if (typeof ret != "boolean" || ret === true) return true;
        /** else: keep searching up */
      }
    }
    log.trace("action.virtual", action.id, "no implementing view", viewsInOrder);
    if (isDeveloperMode.value) {
      toaster.debug({
        title: `Can't ${toValue(action.title)} Here`,
        text: `No view supports ${action.id}.`,
      });
    }
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
      if ((action.shortcuts?.length ?? 0) > 0) {
        keytrap.bind(action.shortcuts!, (e) => fireActionFromEvent(action, e), { key: id, replace: true });
      }
    });

    IMPLEMENTED_ACTIONS_BY_ID.value = implemented;
  },
  { immediate: true },
);

// declare common actions
declareActionMap<"common">({
  // create
  "common.create.above": {
    icon: "fas fa-angles-up",
    title: "Create Above",
    text: "Create a new item above this item",
  },
  "common.create.below": {
    icon: "fas fa-angles-down",
    title: "Create Below",
    text: "Create a new item below this item",
  },
  "common.create.block": {
    icon: "fas fa-cube",
    title: "Create Block",
    text: "Create a new Block",
  },
  "common.create.record": {
    icon: "fas fa-record-vinyl",
    title: "Create Record",
    text: "Create a new Record",
  },
  "common.create.step": {
    icon: "fas fa-step-forward",
    title: "Create Step",
    text: "Create a new Step",
  },
  "common.create.pipe": {
    icon: "fas fa-pipe-section",
    title: "Create Pipe",
    text: "Create a new Pipe",
  },
  // edit
  "common.edit.rename": {
    icon: "fas fa-pencil",
    title: "Rename",
    text: "Rename this item",
    shortcuts: ["f2"],
  },
  "common.edit.move": {
    icon: "fas fa-arrows-turn-right",
    title: "Move",
    text: "Move this item",
  },
  "common.edit.copy": {
    icon: "fas fa-copy",
    title: "Copy",
    text: "Copy this item",
    shortcuts: ["mod+c"],
  },
  "common.edit.cut": {
    icon: "fas fa-scissors",
    title: "Cut",
    text: "Cut this item",
    shortcuts: ["mod+x"],
  },
  "common.edit.paste": {
    icon: "fas fa-paste",
    title: "Paste",
    text: "Paste this item",
    shortcuts: ["mod+v"],
  },
  "common.edit.duplicate": {
    icon: "fas fa-clone",
    title: "Duplicate",
    text: "Duplicate this item",
    shortcuts: ["mod+d"],
  },
  "common.edit.delete": {
    icon: "fas fa-trash-can",
    title: "Delete",
    text: "Delete this item",
    shortcuts: ["del", "backspace"],
  },
  // navigate
  "common.navigate.open": {
    icon: "fas fa-magnifying-glass-plus",
    title: "Open",
    text: "Open this node in a new view",
    shortcuts: ["mod+enter"],
  },
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
  "common.navigate.zoomIn": {
    icon: "fas fa-search-plus",
    title: "Zoom In",
    text: "Zoom in",
    shortcuts: ["plus", "mod+plus"],
  },
  "common.navigate.zoomOut": {
    icon: "fas fa-search-minus",
    title: "Zoom Out",
    text: "Zoom out",
    shortcuts: ["minus", "mod+minus"],
  },
  "common.navigate.reset": {
    icon: "fas fa-arrows-to-dot",
    title: "Reset",
    text: "Reset",
    shortcuts: ["0"],
  },
  "common.navigate.enter": {
    icon: "fas fa-arrow-in",
    title: "Navigate In",
    text: "Navigate in",
    shortcuts: ["enter"],
  },
  "common.navigate.exit": {
    icon: "fas fa-arrow-out",
    title: "Navigate Out",
    text: "Navigate out",
    shortcuts: ["esc"],
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
    shortcuts: ["alt+up"],
  },
  "common.move.down": {
    icon: "fas fa-square-down",
    title: "Move Down",
    text: "Move down",
    shortcuts: ["alt+down"],
  },
  "common.move.left": {
    icon: "fas fa-square-left",
    title: "Move Left",
    text: "Move left",
    shortcuts: ["alt+left", "shift+tab"],
  },
  "common.move.right": {
    icon: "fas fa-square-right",
    title: "Move Right",
    text: "Move right",
    shortcuts: ["alt+right", "tab"],
  },
  // search
  "common.search.findInView": {
    icon: "fas fa-magnifying-glass",
    title: "Search in View",
    text: "Find in this view",
    shortcuts: ["mod+f"],
  },
  "common.search.replaceInView": {
    icon: "fas fa-right-left",
    title: "Replace in View",
    text: "Replace in this view",
    shortcuts: ["mod+r"],
  },
  "common.search.findInSpace": {
    icon: "fas fa-magnifying-glass",
    title: "Search in Space",
    text: "Find in this space",
    shortcuts: ["mod+shift+f"],
  },
  "common.search.replaceInSpace": {
    icon: "fas fa-right-left",
    title: "Replace in Space",
    text: "Replace in this space",
    shortcuts: ["mod+shift+r"],
  },
  // sense

  "common.sense.goToDefinition": {
    icon: "fas fa-turn-down-right",
    title: "Go to Definition",
    text: "Go to definition of this node",
    shortcuts: ["mod+b"],
  },
  "common.sense.findReferences": {
    icon: "fas fa-turn-down-left",
    title: "Find References",
    text: "Find references of this node",
    shortcuts: ["mod+shift+b"],
  },
  "common.sense.findImplementations": {
    icon: "fas fa-turn-down-left",
    title: "Find Implementations",
    text: "Find implementations of this node",
  },
});

// history
contributeActionMap<"common.history">({
  "common.history.undo": {
    icon: "fas fa-arrow-turn-left",
    title: "Undo",
    text: "Undo the last action or edit",
    shortcuts: ["mod+z"],
    isEnabled: () => getEditStack(canvas.graph, canvas.focusedView).canUndo,
    action: (action, ctx) => {
      const stack = getEditStack(canvas.graph, canvas.focusedView);
      stack.undo();
    },
  },
  "common.history.redo": {
    icon: "fas fa-arrow-turn-right",
    title: "Redo",
    text: "Redo the last undone action or edit",
    shortcuts: ["mod+shift+z"],
    isEnabled: () => getEditStack(canvas.graph, canvas.focusedView).canRedo,
    action: (action, ctx) => {
      const stack = getEditStack(canvas.graph, canvas.focusedView);
      stack.redo();
    },
  },
});

// session
declareActionMap<"session">({
  // session
  // TODO :Incomplete: session.* action handling (in Block/Step/Page/...)
  "session.run.start": {
    icon: "fas fa-play",
    title: "Run",
    text: "Run this node",
    shortcuts: ["ctrl+r", "meta+enter"],
  },
  "session.run.pause": {
    icon: "fas fa-pause",
    title: "Pause",
    text: "Pause this node",
  },
  "session.run.resume": {
    icon: "fas fa-play",
    title: "Resume",
    text: "Resume this node",
  },
  "session.run.kill": {
    icon: "fas fa-stop",
    title: "Kill",
    text: "Kill this node",
  },
});

// type
declareActionMap<"type">({
  // edit
  "type.edit.isList": {
    type: "toggle",
    icon: "fas fa-list",
    title: "List",
    text: "Mark this type as a list",
  },
  "type.edit.isRequired": {
    type: "toggle",
    icon: "fas fa-shield-check",
    title: "Required",
    text: "Mark this type as required",
  },
  "type.edit.isSecret": {
    type: "toggle",
    icon: "fas fa-lock",
    title: "Secret",
    text: "Mark this type as secret",
  },
});

// database
declareActionMap<"database">({
  "database.column.sortAscending": {
    icon: "fas fa-arrow-up",
    title: "Sort Ascending",
    text: "Sort this column in ascending order",
  },
  "database.column.sortDescending": {
    icon: "fas fa-arrow-down",
    title: "Sort Descending",
    text: "Sort this column in descending order",
  },
  "database.column.filter": {
    icon: "fas fa-filter",
    title: "Filter",
    text: "Filter this column",
  },
  "database.column.wrap": {
    icon: "fas fa-align-justify",
    title: "Wrap",
    text: "Wrap this column",
  },
  "database.column.hide": {
    type: "toggle",
    icon: "fas fa-eye-slash",
    title: "Hide",
    text: "Hide this column",
  },
});

// flow
declareActionMap<"flow">({
  // pipe
  "flow.pipe.begin": {
    icon: "fas fa-arrow-right",
    title: "Pipe From Here",
    text: "Begin a Pipe at this Step",
  },
});

// pipe
declareActionMap<"pipe">({});

// message
declareActionMap<"message">({
  // handle
  "message.chat.reply": {
    icon: "fas fa-reply",
    title: "Reply",
    text: "Reply to this message",
  },
  "message.edit.edit": {
    icon: "fas fa-pencil",
    title: "Edit",
    text: "Edit this message",
  },
  "message.edit.pin": {
    type: "toggle",
    icon: "fas fa-thumbtack",
    title: "Pin",
    text: "Pin this message",
  },
});

// text
declareActionMap<"text">({
  // format
  "text.format.bold": {
    type: "toggle",
    icon: "fas fa-bold",
    title: "Bold",
    text: "Bold text",
    shortcuts: ["mod+b"],
  },
  "text.format.italic": {
    type: "toggle",
    icon: "fas fa-italic",
    title: "Italic",
    text: "Italicize text",
    shortcuts: ["mod+i"],
  },
  "text.format.strikethrough": {
    type: "toggle",
    icon: "fas fa-strikethrough",
    title: "Strikethrough",
    text: "Strikethrough text",
  },
  "text.format.underline": {
    type: "toggle",
    icon: "fas fa-underline",
    title: "Underline",
    text: "Underline text",
    shortcuts: ["mod+u"],
  },
  "text.format.code": {
    type: "toggle",
    icon: "fas fa-code",
    title: "Code",
    text: "Code text",
  },
  // edit
  "text.edit.hardBreak": {
    icon: "fas fa-arrow-down",
    title: "Hard Break",
    text: "Insert a hard break",
    shortcuts: ["shift+enter"],
  },
});

// code
declareActionMap<"code">({
  // edit
  "code.edit.format": {
    icon: "fas fa-code",
    title: "Format",
    text: "Reformat the code",
    shortcuts: ["mod+alt+l"],
  },
  "code.edit.comment": {
    icon: "fas fa-code",
    title: "Comment",
    text: "Comment/uncomment these lines",
    shortcuts: ["ctrl+t"],
  },
});

// view
declareActionMap<"view">({
  // navigate
  "view.navigate.duplicateTab": {
    icon: "fas fa-copy",
    title: "Duplicate Tab",
    text: "Duplicate the current tab",
  },
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
    text: "Close this tab",
    shortcuts: ["mod+w", "ctrl+w"],
  },
  "view.navigate.closeOtherTabs": {
    icon: "fas fa-xmark",
    title: "Close Other Tabs",
    text: "Close all other tabs",
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
    text: "Close this frame",
    shortcuts: ["mod+shift+w"],
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
    text: "Close this split",
  },
  // layout
  "view.layout.splitUp": {
    icon: "fas fa-reflect-vertical",
    title: "Split Up",
    text: "Split this view vertically (new split above)",
  },
  "view.layout.splitDown": {
    icon: "fas fa-reflect-vertical",
    title: "Split Down",
    text: "Split this view vertically (new split below)",
  },
  "view.layout.splitLeft": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Left",
    text: "Split this view horizontally (new split left)",
  },
  "view.layout.splitRight": {
    icon: "fas fa-reflect-horizontal",
    title: "Split Right",
    text: "Split this view horizontally (new split right)",
  },
  "view.layout.pinSplit": {
    type: "toggle",
    icon: "fas fa-thumbtack",
    title: "Pin Split",
    text: "Pin this split to an absolute size",
  },
});
contributeActionMap<"view">({
  // canvas
  "view.space.resetDefault": {
    isEnabled: hasLocalBench,
    title: "Restore Default Space",
    text: "Reset the space to the default layout",
    icon: "fas fa-galaxy",
    action: () => {
      if (pkg.value == null || space.value == null) return;
      const tx = canvas.tx();
      clearSpace(tx, canvas.graph, space.value);
      createDesktopDefaultSpace(tx, space.value);
    },
  },
});

// developer actions
contributeActionMap<"developer">({
  "developer.developerMode": {
    type: "toggle",
    icon: "fas fa-binary",
    title: "Developer Mode",
    text: "Developer Mode enables some advanced and some weird features.",
    isChecked: isDeveloperMode,
    action: () => {
      isDeveloperMode.value = !isDeveloperMode.value;
      toaster.info({
        override: "developer.toggleDeveloperMode",
        title: isDeveloperMode.value ? "Developer Mode Enabled" : "Developer Mode Disabled",
        text: isDeveloperMode.value ? "Welcome to the dark side." : "Back to the normal side.",
        icon: "fas fa-binary",
        actions: [
          {
            title: isDeveloperMode.value ? "Disable" : "Enable",
            icon: makeIcon({ faName: isDeveloperMode.value ? "fas fa-toggle-off" : "fas fa-toggle-on" }),
            action: () => {
              isDeveloperMode.value = !isDeveloperMode.value;
            },
          },
        ],
      });
    },
    shortcuts: ["alt+f12", "f12"],
  },
  "developer.tx.retryAllFailed": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-redo",
    title: "Retry All Failed Commits",
    text: "Retry all current failed transactions",
    action: () => {
      getAllTransactionBuffers().forEach((txBuffer) => {
        Object.values(txBuffer.failedCommits?.value ?? {}).forEach((commit) => txBuffer.retry!(commit.id));
      });
    },
  },
  "developer.view.addEmptyView": {
    isEnabled: isDeveloperMode,
    icon: "fas fa-window-frame",
    title: "Add Empty View",
    text: "Adds an empty debug view to this root",
    action: () => {
      const name = toCasing(generateRandomName().toUpperCase(), Casing.CAMEL, true);
      canvas.addView({ type: ViewType.EMPTY, name, title: name });
    },
  },
});

// space actions
contributeActionMap<"space">({
  "space.launch.documentation": {
    title: "Open Documentation",
    text: "Get help from our examples and guides",
    icon: "fas fa-book-open",
    action: ACTION_COMING_SOON,
  },
  "space.launch.discord": {
    title: "Open Discord",
    text: "Join the community on Discord",
    icon: "fab fa-discord",
    url: DISCORD_URL,
    action: () => {
      // open in new tab
    },
  },
  "space.launch.notifications": {
    title: "Open Notifications",
    text: "View your notifications",
    icon: "fas fa-bell",
    isEnabled: ref(false),
    action: ACTION_COMING_SOON,
  },
  // edit
  "space.edit.inspect": {
    title: "Inspect Node",
    text: "Inspect a selected node",
    icon: "fas fa-eye-dropper",
    action: ACTION_COMING_SOON,
  },
  "space.edit.create": {
    title: "Create Space",
    text: "Create a new separate Space",
    icon: "fas fa-plus",
    action: ACTION_COMING_SOON,
  },
  // full screen
  "space.display.fullscreen": {
    type: "toggle",
    icon: "fas fa-maximize",
    title: "Toggle Fullscreen",
    text: "Toggle fullscreen mode",
    action: () => {
      const isFullscreen = document.fullscreenElement != null;
      if (isFullscreen) document.exitFullscreen();
      else document.documentElement.requestFullscreen();
    },
  },
});
