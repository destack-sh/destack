import type { IconData, NodeReferenceData, TextData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import type { FIlterPrefix as FilterPrefix } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { keytrap, type KeySignature } from "@/utils/keymap";
import { log } from "@/utils/log";
import { getCurrentInstance, shallowRef, type Ref, triggerRef, watch } from "vue";

export type ActionCategory = "user" | "space" | "view" | "editor";
export type ActionBuiltinId =
  // user
  | "user.signup"
  | "user.login"
  | "user.logout"
  // space
  | "space.open.omnibar.universal"
  | "space.open.omnibar.action"
  | "space.open.omnibar.space"
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
  // view
  | "view.edit.do"
  | "view.edit.undo"
  | "view.edit.delete"
  | "view.edit.copy"
  | "view.edit.cut"
  | "view.edit.paste"
  | "view.edit.duplicate"
  | "view.navigate.up"
  | "view.navigate.down"
  | "view.navigate.left"
  | "view.navigate.right"
  | "view.select.all"
  | "view.select.up"
  | "view.select.down"
  | "view.select.left"
  | "view.select.right"
  | "view.select.clear"
  | "view.move.up"
  | "view.move.down"
  | "view.move.left"
  | "view.move.right"
  | "view.layout.closeTab"
  | "view.layout.focusNextWindow"
  | "view.layout.splitWindowHorizontal"
  | "view.layout.splitWindowVertical"
  | "view.analyze.goToDefinition"
  | "view.analyze.findReferences";

export type ActionBuiltinCategory = FilterPrefix<ActionBuiltinId, string>;

export type ActionSource = { kind: "builtin"; id: ActionBuiltinId } | { kind: "block"; block: NodeReferenceData };
// export type ActionKind = "global" | "contextual";

/**
 * An Action that can be performed by the user in the space.
 *
 * TODO :Architecture: define Action as Struct so it can be provided by custom Views/...?
 *  provide actions by tagging runnable (no args) Blocks with Action?
 */
export type Action = {
  // kind: ActionKind;
  icon?: IconData;
  key: string; // some unique identifier for the action
  title: string;
  aliases?: string[];
  text?: string | TextData;
  shortcuts?: KeySignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  enabled?: Ref<boolean>;
  source: ActionSource;
  category: string;
  action: () => void | boolean | Promise<void> | Promise<boolean>;
  url?: string; // for external URLs
  excludeInOmnibar?: boolean; // don't show in omnibar
};

export const BUILTIN_ACTIONS: Ref<Partial<Record<ActionBuiltinId, Action>>> = shallowRef({});

type ActionIn = Pick<
  Action,
  "title" | "aliases" | "text" | "shortcuts" | "enabled" | "action" | "url" | "excludeInOmnibar"
> & {
  id: ActionBuiltinId;
  icon?: string | IconData;
};
export type BuiltinActionMap<T extends string> = Record<FilterPrefix<ActionBuiltinId, T>, Omit<ActionIn, "id">>;

export function contributeAction(in_: ActionIn) {
  const action: Action = {
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ name: in_.icon }) : in_.icon,
    source: { kind: "builtin", id: in_.id },
    key: in_.id,
    category: in_.id.split(".")[0],
  };
  if (BUILTIN_ACTIONS.value[in_.id] != null && (!IS_DEBUG || getCurrentInstance() == null))
    // hot-reloading re-registers actions
    throw new Error(`action already exists: ${in_.id} (${in_} != ${BUILTIN_ACTIONS.value[in_.id]})`);
  BUILTIN_ACTIONS.value[in_.id as ActionBuiltinId] = action;
  triggerRef(BUILTIN_ACTIONS);
}

export function contributeActionMap<T extends string>(map: Partial<BuiltinActionMap<T>>) {
  Object.entries(map).forEach(([id, in_]) => contributeAction({ ...(in_ as ActionIn), id: id as ActionBuiltinId }));
}

export function getAction(id: ActionBuiltinId): Action {
  const action = BUILTIN_ACTIONS.value[id];
  if (action == null) throw new Error(`no such action: ${id}`);
  return action;
}

export function runAction(id: ActionBuiltinId) {
  getAction(id).action();
}

function fireAction(action: Action): boolean {
  if (action.enabled != null && !action.enabled.value) return false;
  const ret = action.action();
  return typeof ret === "boolean" ? ret : true;
}

// register actions with keytrap
const bindings: Array<() => void> = [];
watch(BUILTIN_ACTIONS, (actions) => {
  log.debug("action.keymap", Object.keys(actions));
  bindings.forEach((unbind) => unbind());
  Object.values(actions)
    .filter((a) => (a.shortcuts?.length ?? 0) > 0)
    .forEach((action) => {
      const unbind = keytrap.bind(action.shortcuts!, () => fireAction(action));
      bindings.push(unbind);
    });
});
