import { isResourceNodeType, isRuntimeNodeType, RESOURCE_NODE_TYPES } from "@/language/const";
import { ReadNodeGraph } from "@/language/graph";
import { NodeType, type AnyNodeData, type IconData, type TextData } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { isDeveloperMode } from "@/system/client";
import type { ConnectionBase } from "@/system/connection";
import { canvas, space, supergraph } from "@/system/globals";
import { makeIcon } from "@/ui/icon";
import { keytrap, type KeySignature } from "@/ui/keymap";
import { toaster } from "@/ui/toast";
import { collectViewComponentsUp } from "@/ui/view";
import { type FilterPrefix } from "@/utils/functools";
import { IS_DEV } from "@/utils/globals";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import type { ViewComponent } from "@/views/common";
import { useKeyModifier } from "@vueuse/core";
import { computed, getCurrentInstance, shallowRef, toValue, triggerRef, watch, type MaybeRef, type Ref } from "vue";

export const IS_IN_ALT_MODE = useKeyModifier("Alt");

// :OmnibarModes
export type OmnibarMode = "everywhere" | "actions" | "bench";
export const OMNIBAR_MODES: OmnibarMode[] = ["everywhere", "actions", "bench"];

export const ACTION_BUILTIN_IDS = [
  // space
  // (:OmnibarModes)
  "space.omnibar.everywhere",
  "space.omnibar.actions",
  "space.omnibar.bench",
  "space.launch.discord",
  "space.launch.fullscreen",
  // history
  "space.history.undo",
  "space.history.redo",
  // edit
  "space.edit.rename",
  "space.edit.move",
  "space.edit.copy",
  "space.edit.cut",
  "space.edit.paste",
  "space.edit.duplicate",
  "space.edit.delete",
  // navigate
  "space.navigate.open",
  "space.navigate.up",
  "space.navigate.down",
  "space.navigate.left",
  "space.navigate.right",
  "space.navigate.zoomIn",
  "space.navigate.zoomOut",
  "space.navigate.enter",
  "space.navigate.exit",
  "space.navigate.reset",
  "space.navigate.pageUp",
  "space.navigate.pageDown",
  "space.navigate.goBackward",
  "space.navigate.goForward",
  // select
  "space.select.all",
  "space.select.up",
  "space.select.down",
  "space.select.left",
  "space.select.right",
  "space.select.clear",
  // move
  "space.move.up",
  "space.move.down",
  "space.move.left",
  "space.move.right",
  "space.move.inside",
  "space.move.outside",
  // search
  "space.search.findInView",
  "space.search.replaceInView",
  "space.search.findInSpace",
  "space.search.replaceInSpace",
  // runtime
  "runtime.run.start",
  "runtime.run.pause",
  "runtime.run.resume",
  "runtime.run.kill",
  "runtime.interruption.resume",
  "runtime.interruption.cancel",
  // list
  "list.create.above",
  "list.create.below",
  // tree
  "tree.create.above",
  "tree.create.below",
  "tree.create.inside",
  // table
  "table.create.record",
  "table.column.sortAscending",
  "table.column.sortDescending",
  "table.column.filter",
  "table.column.wrap",
  "table.column.hide",
  // flow
  "flow.edit.createAction",
  "flow.edit.splitPipe",
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
  // resource
  "resource.status.activate",
  "resource.status.suspend",
  "resource.status.decommission",
  // view
  "view.history.goBackward",
  "view.history.goForward",
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
  "user.security.signup",
  "user.security.login",
  "user.security.logout",
  "user.security.logoutAll",
  "user.navigate.activate",
  "user.navigate.goToHome",
  // developer
  "debug.test.developerMode",
  "debug.test.retryAllFailed",
  "debug.test.addEmptyView",
  "debug.toast.success",
  "debug.toast.info",
  "debug.toast.debug",
  "debug.toast.error",
] as const;
export const ACTION_BUILTIN_IDS_INDEX: Record<ActionBuiltinId, number> = ACTION_BUILTIN_IDS.reduce(
  (acc, id, idx) => ({ ...acc, [id]: idx }),
  {},
) as Record<ActionBuiltinId, number>;
export type ActionBuiltinId = (typeof ACTION_BUILTIN_IDS)[number];
export type ActionBuiltinCategory = FilterPrefix<ActionBuiltinId, string>;
export type ActionContext = {
  event?: KeyboardEvent | MouseEvent;
  nodes?: AnyNodeData[];
};
export type ActionCallable = (
  action: Action,
  context: ActionContext,
) => void | boolean | Promise<void> | Promise<boolean>;
export type ActionKind = "static" | "virtual";
export const ACTION_TYPES = ["generic", "external-url", "toggle", "menu"] as const;
export type ActionType = (typeof ACTION_TYPES)[number];
export const ACTION_COMING_SOON: ActionCallable = (action: Action) =>
  toaster.debug({ title: "Coming soon", text: `"${toValue(action.title)}" is not yet available.`, icon: action.icon });

/**
 * An Action that can be performed by the User in the Space.
 * Actions can be declared and implemented in different places (e.g. for different behavior in various Views).
 *
 * NOTE :Architecture: define Action as Struct so it can be provided by custom Views/...?
 */
export type Action = {
  kind: ActionKind;
  type: ActionType;
  id: ActionBuiltinId;
  icon?: IconData;
  title: MaybeRef<string>;
  text: string;
  shortcuts?: KeySignature[]; // NOTE :Incomplete: define shortcuts in per-Space/User keymap?
  isEnabled?: Ref<boolean> | ((action: Action, ctx?: ActionContext) => boolean | undefined);
  category: string;
  subcategory?: string;
  path: string;
  action?: ActionCallable;
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
export type ActionDeclaration = Omit<ActionIn, "id" | "enabled" | "action"> & { action?: ActionCallable };
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
    id: in_.id,
    category: idParts[0],
    subcategory: idParts[1],
    path: idParts.slice(0, -1).join(" / "),
  };
  if (DECLARED_ACTIONS_BY_ID.value[in_.id] != null && (!IS_DEV || getCurrentInstance() == null)) {
    // hot-reloading re-registers actions (sometimes)
    throw new Error(`action already exists: ${in_.id} (${in_} != ${DECLARED_ACTIONS_BY_ID.value[in_.id]})`);
  }
  DECLARED_ACTIONS_BY_ID.value[in_.id as ActionBuiltinId] = action;
  triggerRef(DECLARED_ACTIONS_BY_ID);
}

/** Contributes actions globally (same implementation everywhere) */
export function provideActions<T extends string>(map: Partial<ActionMapContribution<T>>) {
  Object.entries(map).forEach(([id, action]) =>
    addAction("static", { ...(action as ActionIn), id: id as ActionBuiltinId }),
  );
}

/** Declares actions to be implemented virtually. */
export function declareActions<T extends string>(
  map: Partial<ActionMapDeclaration<T>>,
): Partial<ActionMapDeclaration<T>> {
  Object.entries(map).forEach(([id, action]) =>
    addAction("virtual", { ...(action as ActionIn), id: id as ActionBuiltinId }),
  );
  return map;
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
  input: ["space.edit", "space.navigate", "space.select", "space.move"],
  textarea: ["space.edit", "space.navigate", "space.select", "space.move"],
  contenteditable: ["space.edit", "space.navigate", "space.select", "space.move"],
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

/** Whether the given action is enabled in the context. */
export function isActionEnabled(action: Action, context?: ActionContext): boolean {
  if (action.isEnabled == null) {
    return true;
  } else if (typeof action.isEnabled == "function") {
    return action.isEnabled(action, context) ?? false;
  } else {
    return toValue(action.isEnabled);
  }
}

/** Triggers the bound action by id. */
export function fireActionById(id: ActionBuiltinId, context?: ActionContext) {
  const action = getAction(id);
  fireAction(action, canvas.focusedViewComponents, context);
}

/** Triggers the bound action from a keyboard event. */
export function fireActionFromEvent(action: Action, e: KeyboardEvent): boolean {
  // assemble current context
  const context: ActionContext = { event: e };
  if (canvas.selection != null && supergraph.getManyMaybe(canvas.selection.nodesPtr).length > 0) {
    context.nodes = supergraph.getManyMaybe(canvas.selection.nodesPtr);
  } else if (space.value?.inspectionPtr != null) {
    const node = supergraph.get(space.value.inspectionPtr);
    if (node != null) context.nodes = [node];
  }

  // bail if action is disabled or suppressed
  if (!isActionEnabled(action, context)) {
    log.trace("action.disabled", action.id);
    return false;
  }
  const suppressor = getActionSuppressor(action.id, e.target as HTMLElement);
  if (suppressor) {
    log.trace("action.suppressed", action.id, suppressor);
    return false;
  }

  // fire action
  const localViewsInOrder = collectViewComponentsUp(e.target as HTMLElement);
  // NOTE: concat local views and focused view components so local ones are preferred, but all are available
  return fireAction(action, [...localViewsInOrder, ...canvas.focusedViewComponents], context);
}

/**
 * Checks whether the context implements the action.
 * NOTE: does not 'call' the action, so we can't know if the action is refused dynamically.
 */
export function getImplementingAction(
  action: Action,
  viewsInContext: ViewComponent[],
  context: ActionContext,
): ActionImplementation | null {
  if (action.kind == "static") {
    if (isActionEnabled(action, context)) {
      return action;
    } else {
      return null;
    }
  } else if (action.kind == "virtual") {
    for (const view of viewsInContext) {
      const impl = view.exposed?.actions?.[action.id];
      if (typeof impl == "function") {
        return { action: impl };
      }
      if (impl != null && isActionEnabled(action, context)) {
        return impl;
      }
    }
    if (action.action != null) {
      return { action: action.action };
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
  context: ActionContext = {},
) {
  if (!isActionEnabled(action, context)) return false;
  if (action.kind == "static") {
    // static: just call callback directly
    log.trace("action.static", action.id);
    const ret = action.action!(action, context);
    return typeof ret === "boolean" ? ret : true;
  } else if (action.kind == "virtual") {
    // virtual: find first component implementing that action
    for (const view of viewsInOrder ?? []) {
      let impl = view.exposed?.actions?.[action.id];
      if (typeof impl == "function") impl = { action: impl };
      if (impl?.action != null && isActionEnabled(action, context)) {
        log.trace("action.virtual", action.id);
        const ret = impl.action(action, context);
        if (typeof ret != "boolean" || ret === true) return true;
        /** else: keep searching up */
      }
    }
    if (action.action != null) {
      // default to static implementation if we have one
      const ret = action.action(action, context);
      if (typeof ret != "boolean" || ret === true) return true;
    }

    log.trace("action.virtual", action.id, "no implementing view", { context, viewsInOrder });
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
// NOTE: implemented actions may contain disabled actions, we filter those at a later action to avoid updating this too often
//  (we evaluate the actual action to call only when firing the callback anyway)
const IMPLEMENTED_ACTIONS_BY_ID: Ref<Record<string, Action>> = shallowRef({});
export const IMPLEMENTED_ACTIONS: Readonly<Ref<Action[]>> = computed(() =>
  Object.values(IMPLEMENTED_ACTIONS_BY_ID.value),
);
let _isWatching = false;

/** Watches for changes in actions and updates the keytrap accordingly. Should only be called once. */
export function watchActions() {
  if (_isWatching) throw new Error("watchActions already called");
  _isWatching = true;
  return watch(
    [DECLARED_ACTIONS, canvas.focusedViewComponentsById],
    () => {
      const implementedActions: Record<string, Action> = {};

      // static actions
      Object.values(DECLARED_ACTIONS.value)
        .filter((a) => a.kind == "static" || (a.kind == "virtual" && a.action != null))
        .forEach((a) => (implementedActions[a.id] = a));

      // collect virtual actions bottom up
      for (const view of Object.values(canvas.focusedViewComponentsById.value)) {
        for (const [actionId, action] of Object.entries(view.exposed?.actions ?? {})) {
          if (implementedActions[actionId] != null) continue; // already declared (either static or by lower view)
          const declaration = DECLARED_ACTIONS_BY_ID.value[actionId];
          if (declaration == null) throw new Error(`no declaration for virtual action: ${actionId}`);
          implementedActions[actionId] = { ...declaration, ...action };
        }
      }

      // diff & update keytrap
      const oldImplemented = IMPLEMENTED_ACTIONS_BY_ID.value;
      const removedActions = Object.keys(oldImplemented).filter((id) => implementedActions[id] == null);
      const addedActions = Object.keys(implementedActions).filter((id) => oldImplemented[id] == null);
      removedActions.forEach((id) => keytrap.unbind(oldImplemented[id].shortcuts ?? []));
      addedActions.forEach((id) => {
        const action = implementedActions[id];
        if ((action.shortcuts?.length ?? 0) > 0) {
          keytrap.bind(action.shortcuts!, (e) => fireActionFromEvent(action, e), { key: id, replace: true });
        }
      });

      IMPLEMENTED_ACTIONS_BY_ID.value = implementedActions;
    },
    { immediate: true },
  );
}

/** Gets the nodes in the context of an Action. Only returns the nodes if they are part of tbe same connection. */
export function getNodesForAction(
  action: Action,
  context: ActionContext | undefined,
): {
  connection: ConnectionBase<any, any> | null;
  graph: ReadNodeGraph | null;
  nodes: AnyNodeData[];
} {
  // gather 'context' nodes
  let nodesPtr = [];
  if (
    canvas.selection != null &&
    supergraph.getManyMaybe(canvas.selection.nodesPtr).length > 0 &&
    (context?.nodes == null ||
      // expand context nodes to selection if there is overlap
      canvas.selection.nodesPtr.some((nodePtr) => context.nodes!.some((n) => n.id == nodePtr.id)))
  ) {
    nodesPtr = canvas.selection.nodesPtr;
  } else if (context?.nodes != null) {
    nodesPtr = context.nodes.map(toNodeRef);
  } else if (canvas.inspection != null) {
    nodesPtr = [canvas.inspection];
  } else {
    return { connection: null, graph: null, nodes: [] };
  }

  // find link
  let link = null;
  for (const nodePtr of nodesPtr) {
    link = supergraph.getLink(nodePtr);
    if (link == null) break;
  }
  if (link == null) return { connection: null, graph: null, nodes: [] };
  const { connection, graph } = link;

  // get and deduplicate nodes
  const nodes = [];
  const nodesById: Record<string, AnyNodeData> = {};
  for (const nodePtr of nodesPtr) {
    const node = graph.get(nodePtr);
    if (node != null && nodesById[node.id!] == null) {
      nodes.push(node);
      nodesById[node.id!] = node;
    }
  }
  return { connection, graph, nodes };
}

//
// Node context actions
//

export const NON_DUPLICATABLE_NODE_TYPES = [NodeType.PIPE];

export const SCALAR_CONTEXT_ACTIONS: ActionBuiltinId[] = ["space.edit.rename"];
export const NODE_CONTEXT_ACTIONS: ActionBuiltinId[] = ["space.edit.duplicate", "space.edit.delete"];
export const FIELD_CONTEXT_ACTIONS: ActionBuiltinId[] = [];
export const BLOCK_CONTEXT_ACTIONS: ActionBuiltinId[] = [];
export const RECORD_CONTEXT_ACTIONS: ActionBuiltinId[] = [];
export const ACTION_CONTEXT_ACTIONS: ActionBuiltinId[] = [];
export const RUN_CONTEXT_ACTIONS: ActionBuiltinId[] = ["runtime.run.pause", "runtime.run.resume", "runtime.run.kill"];
export const INTERRUPTION_CONTEXT_ACTIONS: ActionBuiltinId[] = ["runtime.interruption.resume", "runtime.interruption.cancel"];
export const PIPE_CONTEXT_ACTIONS: ActionBuiltinId[] = ["flow.edit.splitPipe"];
export const RESOURCE_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "resource.status.activate",
  "resource.status.suspend",
  "resource.status.decommission",
];

export const CONTEXT_ACTIONS_BY_TYPE: Partial<Record<NodeType, ActionBuiltinId[]>> = {
  [NodeType.BLOCK]: BLOCK_CONTEXT_ACTIONS,
  [NodeType.RECORD]: RECORD_CONTEXT_ACTIONS,
  [NodeType.ACTION]: ACTION_CONTEXT_ACTIONS,
  [NodeType.PIPE]: PIPE_CONTEXT_ACTIONS,
  [NodeType.RUN]: RUN_CONTEXT_ACTIONS,
  [NodeType.INTERRUPTION]: INTERRUPTION_CONTEXT_ACTIONS,
};
for (const nodeType of RESOURCE_NODE_TYPES) {
  CONTEXT_ACTIONS_BY_TYPE[nodeType] = [...(CONTEXT_ACTIONS_BY_TYPE[nodeType] ?? []), ...RESOURCE_CONTEXT_ACTIONS];
}

/** Gets the base Actions for a Node. */
export function getNodeActions(node: AnyNodeData): Action[] {
  const actions: ActionBuiltinId[] = [];
  const nodeType = node.metatype as unknown as NodeType;
  const isResource = isResourceNodeType(nodeType);
  const isRuntime = isRuntimeNodeType(nodeType);
  // duplicate/delete
  if (!isResource && !isRuntime) {
    if (!NON_DUPLICATABLE_NODE_TYPES.includes(nodeType)) {
      actions.push("space.edit.duplicate");
    }
    actions.push("space.edit.delete");
  }
  // rename
  if ("title" in node || "name" in node) {
    actions.push("space.edit.rename");
  }
  // general context actions
  if (CONTEXT_ACTIONS_BY_TYPE[nodeType] != null) {
    actions.push(...CONTEXT_ACTIONS_BY_TYPE[nodeType]!);
  }
  return actions.map(getAction);
}
