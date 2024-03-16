import type { IconData, NodeReferenceData, TextData } from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import type { FIlterPrefix as FilterPrefix } from "@/utils/functools";
import type { KeymapSignature } from "@/utils/keymap";
import type { Ref } from "vue";

export type ActionCategory = "space" | "user" | "view" | "editor" | "custom";
export type ActionBuiltinId =
  // space
  | "space.openActionPalette"
  | "space.openSearch"
  | "space.openChat"
  | "space.openInspector"
  | "space.openLibrary"
  | "space.openExplorer"
  | "space.openOutline"
  | "space.openDocs"
  | "space.openLogs"
  | "space.openDiscord"
  // user
  | "user.signup"
  | "user.login"
  | "user.logout"
  // view
  | "view.closeTab"
  | "view.openTab"
  | "view.closeWindow"
  | "view.splitWindowHorizontal"
  | "view.splitWindowVertical"
  // editor
  | "editor.navigate.up"
  | "editor.navigate.down"
  | "editor.navigate.left"
  | "editor.navigate.right"
  | "editor.select.all"
  | "editor.select.up"
  | "editor.select.down"
  | "editor.select.left"
  | "editor.select.right"
  | "editor.select.clear"
  | "editor.indent"
  | "editor.unindent"
  | "editor.move.up"
  | "editor.move.down"
  | "editor.move.left"
  | "editor.move.right"
  | "editor.delete"
  | "editor.copy"
  | "editor.cut"
  | "editor.paste"
  | "editor.duplicate"
  | "editor.goToDefinition"
  | "editor.findReferences";

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
  shortcuts?: KeymapSignature[]; // TODO :Feature: define shortcuts in per-Space & per-User keymap
  enabled?: Ref<boolean>;
  source: ActionSource;
  category: string;
  action: () => void;
};

export const BUILTIN_ACTIONS: Partial<Record<ActionBuiltinId, Action>> = {};

type ActionIn = Pick<Action, "title" | "aliases" | "text" | "shortcuts" | "enabled" | "action"> & {
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
  if (BUILTIN_ACTIONS[in_.id] != null)
    throw new Error(`action already exists: ${in_.id} (${in_} != ${BUILTIN_ACTIONS[in_.id]})`);
  BUILTIN_ACTIONS[in_.id as ActionBuiltinId] = action;
}

export function contributeActionMap<T extends string>(map: Partial<BuiltinActionMap<T>>) {
  Object.entries(map).forEach(([id, in_]) => contributeAction({ ...(in_ as ActionIn), id: id as ActionBuiltinId }));
}

export function getAction(id: ActionBuiltinId): Action {
  const action = BUILTIN_ACTIONS[id];
  if (action == null) throw new Error(`no such action: ${id}`);
  return action;
}

export function runAction(id: ActionBuiltinId) {
  getAction(id).action();
}
