import {
  BlockData,
  BlockType,
  EnumType,
  FieldZone,
  NodeType,
  ObjectType,
  TreeViewPreset,
  ViewType,
  type AnyNodeData,
  type IconData,
  type NodeReferenceData,
  type TextData,
} from "@/proto/wire";
import { describeNode, isNode, toPlainNodeRef, type AnyNodeReferenceData } from "@/proto/wiring";
import { isDeveloperMode, packagePtr } from "@/system/client";
import { makeIcon } from "@/ui/icon";
import { canvas, hasLocalBench, inspectionPtr, pkg, pkgConnection, pkgGraph, space } from "@/system/space";
import { toaster } from "@/ui/toast";
import { getAllTransactionBuffers } from "@/language/transaction";
import { packBuiltinObjectJson } from "@/language/value";
import { generateOrderKey } from "@/utils/fractional";
import { type FilterPrefix } from "@/utils/functools";
import { DISCORD_URL, IS_DEV } from "@/utils/globals";
import { keytrap, type KeySignature } from "@/ui/keymap";
import { log } from "@/utils/log";
import { generateRandomName } from "@/utils/naming";
import { Casing, toCasing } from "@/utils/string";
import { clearSpace, createDesktopAdvancedSpace, createDesktopDefaultSpace, createEmptySpace } from "@/ui/space";
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
import { EXPOSED_ANCHORS, getRandomEnumOption } from "@/ui/inspect";
import { getEditStack } from "@/language/edit";
import { collectViewComponentsUp, SPACE_DEFAULT_BAR_POSITION } from "@/ui/view";

export const IS_IN_ALT_MODE = useKeyModifier("Alt");

// :OmnibarModes
export const OMNIBAR_MODES = ["everywhere", "actions", "space", "views", "view"];
export type OmnibarMode = (typeof OMNIBAR_MODES)[number];

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
  "space.launch.chat",
  "space.launch.inspect",
  "space.launch.create",
  "space.launch.explorer",
  "space.launch.outline",
  "space.launch.docs",
  "space.launch.logs",
  "space.launch.start",
  "space.launch.discord",
  "space.launch.notifications",
  "space.edit.inspect",
  "space.edit.create",
  "space.display.fullscreen",
  // common
  "common.create.here",
  "common.create.above",
  "common.create.below",
  "common.create.link",
  "common.create.block",
  "common.create.trigger",
  "common.create.field",
  "common.create.field.option",
  "common.create.field.input",
  "common.create.field.output",
  "common.create.record",
  "common.create.query",
  "common.create.view",
  "common.create.step",
  "common.history.undo",
  "common.history.redo",
  "common.edit.rename",
  "common.edit.move",
  "common.edit.morph",
  "common.edit.copy",
  "common.edit.cut",
  "common.edit.paste",
  "common.edit.duplicate",
  "common.edit.archive",
  "common.edit.delete",
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
  "common.sense.focus",
  "common.sense.goToDefinition",
  "common.sense.findReferences",
  "common.sense.findImplementations",
  // session
  "session.run.start",
  "session.run.pause",
  "session.run.resume",
  "session.run.kill",
  "session.run.logs",
  // type
  "type.edit.isList",
  "type.edit.isRequired",
  "type.edit.isSecret",
  // block
  "block.edit.isPaused",
  // message
  "message.chat.message",
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
  "view.space.resetBlank",
  "view.space.resetDefault",
  "view.space.resetAdvanced",
  "view.space.rotateBarPosition",
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
  "developer.create.addRootPages",
  "developer.create.addRandomBlocks",
  "developer.create.addRandomFields",
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
export type ActionMapImplementation<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, ActionImplementation>;
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
    log.info("action.static", action.id);
    const ret = action.action(action, context);
    return typeof ret === "boolean" ? ret : true;
  } else if (action.kind == "virtual") {
    // virtual: find first component implementing that action
    for (const view of viewsInOrder ?? []) {
      const impl = view.exposed?.actions?.[action.id];
      if (impl != null && (impl.isEnabled == null || toValue(impl.isEnabled) == true)) {
        log.info("action.virtual", action.id);
        const ret = impl.action(action, context);
        if (typeof ret != "boolean" || ret === true) return true;
        /** else: keep searching up */
      }
    }
    log.debug("action.virtual", action.id, "no implementing view", viewsInOrder);
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
  "common.create.here": {
    icon: "fas fa-plus",
    title: "Create Here",
    text: "Create a new item here",
  },
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
  "common.edit.morph": {
    icon: "fas fa-shuffle",
    title: "Turn Into",
    text: "Change the type of this item",
    shortcuts: ["mod+m"],
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
  "common.edit.archive": {
    icon: "fas fa-archive",
    title: "Archive",
    text: "Archive this item",
  },
  "common.edit.delete": {
    icon: "fas fa-trash",
    title: "Delete",
    text: "Delete this item",
    shortcuts: ["del", "backspace"],
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
  "common.sense.focus": {
    icon: "fas fa-magnifying-glass-plus",
    title: "Focus",
    text: "Focus on this node in a new view",
    shortcuts: ["mod+enter"],
  },
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
  "session.run.logs": {
    icon: "fas fa-clipboard-list",
    title: "View Logs",
    text: "View the logs of this run",
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

// block
declareActionMap<"block">({
  // block
  "block.edit.isPaused": {
    type: "toggle",
    icon: "fas fa-pause",
    title: "Paused",
    text: "Mark this block as paused",
  },
});

// message
declareActionMap<"message">({
  // handle
  "message.chat.message": {
    icon: "fas fa-message",
    title: "Message",
    text: "Add to the message thread",
  },
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
    text: "Close this frame",
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
  "view.space.resetBlank": {
    isEnabled: computed(() => isDeveloperMode.value && space.value != null),
    icon: "fas fa-window",
    title: "Clear Space",
    text: "Clear the space and start blank",
    action: () => {
      if (pkg.value == null) return;
      if (space.value == null) throw new Error(`${describeNode(pkg.value)} has no space`);
      const tx = canvas.tx();
      clearSpace(tx, canvas.graph, space.value);
      createEmptySpace(tx, space.value);
    },
  },
  "view.space.resetDefault": {
    isEnabled: hasLocalBench,
    title: "Restore Default Space",
    text: "Reset the space to the default layout",
    icon: "fas fa-browser",
    action: () => {
      if (pkg.value == null || space.value == null) return;
      const tx = canvas.tx();
      clearSpace(tx, canvas.graph, space.value);
      createDesktopDefaultSpace(tx, space.value);
    },
  },
  "view.space.resetAdvanced": {
    isEnabled: hasLocalBench,
    title: "Restore Advanced Space",
    text: "Reset the space to the advanced layout",
    icon: "fas fa-browser",
    action: () => {
      if (pkg.value == null || space.value == null) return;
      const tx = canvas.tx();
      clearSpace(tx, canvas.graph, space.value);
      createDesktopAdvancedSpace(tx, space.value);
    },
  },
  "view.space.rotateBarPosition": {
    isEnabled: hasLocalBench,
    title: "Rotate the Bar",
    text: "Rotate the bar position in this space",
    icon: "fas fa-rotate",
    action: () => {
      if (space.value == null) throw new Error("no space");
      const tx = canvas.tx();
      const barPosition = space.value.barPosition ?? SPACE_DEFAULT_BAR_POSITION;
      const nextBarPosition = EXPOSED_ANCHORS[(EXPOSED_ANCHORS.indexOf(barPosition) + 1) % EXPOSED_ANCHORS.length];
      tx.update(space.value, { barPosition: nextBarPosition });
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
    icon: "fas fa-window",
    title: "Add Empty View",
    text: "Adds an empty debug view to this root",
    action: () => {
      const name = toCasing(generateRandomName().toUpperCase(), Casing.CAMEL, true);
      canvas.addView({ type: ViewType.EMPTY, name, title: name });
    },
  },
  "developer.create.addRootPages": {
    isEnabled: computed(() => isDeveloperMode.value && hasLocalBench.value),
    icon: "fas fa-folder-plus",
    title: "Add Root Pages",
    text: "Add some root pages to the space",
    action: () => {
      const tx = pkgConnection.tx;
      const pages: BlockData[] = [];
      const existingRootNames = pkgGraph.getChildren(packagePtr.value!, NodeType.BLOCK).map((b) => b.name);
      for (const [pageName, icon] of [
        ["System", "fas fa-gear"],
        ["Library", "fas fa-cubes"],
        ["Mirror", "fas fa-map"],
        ["Applications", "fas fa-compass-drafting"],
        ["Sandbox", "fas fa-game-board"],
      ]) {
        if (existingRootNames.includes(pageName)) continue;
        const page = tx.create({
          metatype: NodeType.BLOCK,
          parentPtr: packagePtr.value!,
          packagePtr: packagePtr.value!,
          type: BlockType.PAGE,
          name: pageName,
          orderKey: generateOrderKey(pages[pages.length - 1]?.orderKey ?? null, null),
          icon: makeIcon({ faName: icon }),
        });
        pages.push(page);
      }
    },
  },
  "developer.create.addRandomBlocks": {
    isEnabled: computed(() => isDeveloperMode.value && hasLocalBench.value),
    icon: "fas fa-cube",
    title: "Add Random Blocks",
    text: "Add some random blocks to the space",
    action: () => {
      const tx = pkgConnection.tx;
      const existingNodes = pkgGraph.nodes.filter(
        (n) => n.metatype == ObjectType.BLOCK || n.metatype == ObjectType.PACKAGE,
      );
      for (let i = 0; i < 10; i++) {
        const name = generateRandomName();
        const parent = existingNodes[Math.floor(Math.random() * existingNodes.length)];
        const existingChildren = pkgGraph.getChildren(parent, NodeType.BLOCK);
        const prevOrderKey = (existingChildren[existingChildren.length - 1] as any)?.orderKey ?? null;
        const type = getRandomEnumOption(EnumType.BLOCK_TYPE);
        const node = tx.create({
          metatype: NodeType.BLOCK,
          parentPtr: toPlainNodeRef(parent),
          packagePtr: packagePtr.value!,
          type,
          name,
          orderKey: generateOrderKey(prevOrderKey, null),
        });
      }
    },
  },
  "developer.create.addRandomFields": {
    isEnabled: computed(() => isDeveloperMode.value && hasLocalBench.value),
    icon: "fas fa-cube",
    title: "Add Random Fields",
    text: "Add some random fields to this block",
    action: () => {
      const tx = pkgConnection.tx;
      const block = pkgGraph.getMaybe(inspectionPtr.value);
      if (!isNode(block, NodeType.BLOCK)) return false;
      for (let i = 0; i < 5; i++) {
        const name = generateRandomName();
        let zone: FieldZone;
        if (block.type == BlockType.CLASS) {
          zone = FieldZone.MEMBER;
        } else if (block.type == BlockType.CHOICE) {
          zone = FieldZone.OPTION;
        } else {
          zone = getRandomEnumOption(EnumType.FIELD_ZONE);
        }
        const existingChildren = pkgGraph.getChildren(block, NodeType.FIELD);
        const field = tx.create({
          metatype: NodeType.FIELD,
          parentPtr: toPlainNodeRef(block),
          packagePtr: packagePtr.value!,
          name,
          zone,
          benchType: getRandomEnumOption(EnumType.BENCH_TYPE),
          isList: Math.random() > 0.5,
          isRequired: Math.random() > 0.4,
          isSecret: Math.random() > 0.8,
          orderKey: generateOrderKey((existingChildren[existingChildren.length - 1] as any)?.orderKey ?? null, null),
        });
      }
    },
  },
});

// space actions
contributeActionMap<"space">({
  "space.launch.chat": {
    title: "Open Chat",
    text: "Chat on your Bench",
    icon: "fas fa-message",
    action: () => {
      canvas.addView({ type: ViewType.CHAT, title: "Chat" }, { ifPresent: "upsertAndFocus" });
    },
  },
  "space.launch.inspect": {
    title: "Open Inspector",
    text: "Open the Inspector View",
    icon: "fas fa-eye",
    action: () => {
      canvas.addView({ type: ViewType.INSPECT, title: "Inspect" }, { ifPresent: "upsertAndFocus" });
    },
  },
  "space.launch.create": {
    title: "Open Creator",
    text: "Get relevant blocks and templates",
    icon: "fas fa-plus",
    action: () => {
      canvas.addView({ type: ViewType.CREATE, title: "Create" }, { ifPresent: "upsertAndFocus" });
    },
  },
  "space.launch.explorer": {
    title: "Open Explorer",
    text: "Navigate nodes in the space",
    icon: "fas fa-compass",
    action: () => {
      canvas.addView(
        {
          type: ViewType.TREE,
          title: "Explore",
          valuePacked: packBuiltinObjectJson({ metatype: ObjectType.TREE_VIEW_STATE, preset: TreeViewPreset.EXPLORE }),
        },
        { ifPresent: "upsertAndFocus", predicate: (view) => view.title != null && view.title.includes("Explore") },
      );
    },
  },
  "space.launch.outline": {
    title: "Open Outline",
    text: "Navigate an outline of nodes",
    icon: "fas fa-list-tree",
    action: () => {
      canvas.addView(
        {
          type: ViewType.TREE,
          title: "Outline",
          valuePacked: packBuiltinObjectJson({ metatype: ObjectType.TREE_VIEW_STATE, preset: TreeViewPreset.OUTLINE }),
        },
        { ifPresent: "upsertAndFocus", predicate: (view) => view.title != null && view.title.includes("Outline") },
      );
    },
  },
  "space.launch.docs": {
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
      window.open(DISCORD_URL, "_blank");
    },
  },
  "space.launch.notifications": {
    title: "Open Notifications",
    text: "View your notifications",
    icon: "fas fa-bell",
    isEnabled: ref(false),
    action: ACTION_COMING_SOON,
  },
  "space.launch.logs": {
    title: "Open Logs",
    text: "Read the Logs",
    icon: "fas fa-clipboard-list",
    action: () => {
      canvas.addView({ type: ViewType.FEED, title: "Logs" }, { ifPresent: "upsertAndFocus" });
    },
  },
  "space.launch.start": {
    title: "Open Start",
    text: "Start a new Run",
    icon: "fas fa-play",
    action: () => {
      canvas.addView({ type: ViewType.START, title: "Start" }, { ifPresent: "upsertAndFocus" });
    },
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

// bench actions
contributeActionMap<"bench">({
  "bench.go.goToBench": {
    title: "Switch Bench",
    text: "Open another Bench",
    icon: "fas fa-circle-dot",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToBranch": {
    isEnabled: ref(false), // not yet implemented
    title: "Switch Branch",
    text: "Go to another Branch in this Bench",
    icon: "fas fa-code-branch",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToEnvironment": {
    isEnabled: ref(false), // not yet implemented
    title: "Switch Environment",
    text: "Go to another Environment in this Bench",
    icon: "fas fa-cloud",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToPackage": {
    isEnabled: hasLocalBench,
    title: "Switch Package",
    text: "Go to another Package in this Bench",
    icon: "fas fa-box",
    action: ACTION_COMING_SOON,
  },
  "bench.go.goToSpace": {
    isEnabled: hasLocalBench,
    title: "Switch Space",
    text: "Go to another Space of this Bench",
    icon: "fas fa-galaxy",
    action: ACTION_COMING_SOON,
  },
});
