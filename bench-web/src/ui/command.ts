import { canvas, supergraph } from "@/globals";
import { ReadNodeGraph } from "@/language/core/graph";
import {
  COSMOS_NODE_TYPES,
  NodeType,
  NodeTypeMapping,
  NodeTypeOptionInfo,
  RESOURCE_NODE_TYPES,
  RUNTIME_NODE_TYPES,
  ROOT_NODE_TYPES,
  type AnyNodeData,
  type IconData,
} from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { isDeveloperMode } from "@/system/client";
import type { ConnectionBase } from "@/system/connection";
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
export type OmnibarMode = "bench" | "commands";
export const OMNIBAR_MODES: OmnibarMode[] = ["bench", "commands"];

export const COMMAND_BUILTIN_IDS = [
  // omnibar
  // (:OmnibarModes)
  "space.omnibar.bench",
  "space.omnibar.commands",
  // launch
  "space.launch.discord",
  "space.launch.fullscreen",
  // create
  "space.create.page",
  "space.create.thread",
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
  "space.edit.archive",
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
  "runtime.run.replay",
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
  "flow.edit.splitTransition",
  // chat
  "chat.message.copy",
  "chat.message.reply",
  "chat.message.forward",
  "chat.message.edit",
  // text
  "text.format.bold",
  "text.format.italic",
  "text.format.strikethrough",
  "text.format.underline",
  "text.format.code",
  "text.format.spoiler",
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
  "developer.test.developerMode",
  "developer.test.retryAllFailed",
  "developer.test.addEmptyView",
  "developer.test.resetFocus",
  "developer.toast.success",
  "developer.toast.info",
  "developer.toast.debug",
  "developer.toast.error",
] as const;
export const COMMAND_BUILTIN_IDS_INDEX: Record<CommandBuiltinId, number> = COMMAND_BUILTIN_IDS.reduce(
  (acc, id, idx) => ({ ...acc, [id]: idx }),
  {},
) as Record<CommandBuiltinId, number>;
export type CommandBuiltinId = (typeof COMMAND_BUILTIN_IDS)[number];
export type CommandBuiltinCategory = FilterPrefix<CommandBuiltinId, string>;
export type CommandContext = {
  event?: KeyboardEvent | MouseEvent;
  nodes?: AnyNodeData[];
};
export type CommandCallable = (
  command: Command,
  context: CommandContext,
) => void | boolean | Promise<void> | Promise<boolean>;
export type CommandKind = "static" | "virtual";
export const COMMAND_TYPES = ["generic", "external-url", "toggle", "menu"] as const;
export type CommandType = (typeof COMMAND_TYPES)[number];
export const COMMAND_COMING_SOON: CommandCallable = (command: Command) =>
  toaster.debug({
    title: "Coming soon",
    text: `"${toValue(command.title)}" is not yet available.`,
    icon: command.icon,
  });

/**
 * An Command that can be performed by the User in the Space.
 * Commands can be declared and implemented in different places (e.g. for different behavior in various Views).
 */
export type Command = {
  kind: CommandKind;
  type: CommandType;
  id: CommandBuiltinId;
  icon?: IconData;
  title: MaybeRef<string>;
  text: string;
  shortcuts?: KeySignature[]; // NOTE :Incomplete: define shortcuts in per-Space/User keymap?
  isEnabled?: Ref<boolean> | ((command: Command, ctx?: CommandContext) => boolean | undefined);
  category: string;
  subcategory?: string;
  path: string;
  command?: CommandCallable;
  // additional metadata
  url?: string; // for external URLs
  isChecked?: Ref<boolean> | ((command: Command, ctx?: CommandContext) => boolean); // for toggle commands
};

export const DECLARED_COMMANDS_BY_ID: Ref<Partial<Record<string, Command>>> = shallowRef({});
export const DECLARED_COMMANDS: Ref<Command[]> = computed(
  () => Object.values(DECLARED_COMMANDS_BY_ID.value) as Command[],
);

type CommandIn = Pick<Command, "title" | "text" | "shortcuts" | "isEnabled" | "command" | "url" | "isChecked"> & {
  type?: CommandType;
  id: CommandBuiltinId;
  icon?: string | IconData;
};
export type CommandDeclaration = Omit<CommandIn, "id" | "enabled" | "command"> & { command?: CommandCallable };
export type CommandMapDeclaration<T extends string> = Record<FilterPrefix<CommandBuiltinId, T>, CommandDeclaration>;
export type CommandKit = Pick<Command, "isEnabled" | "command" | "isChecked">;
export type CommandMapKit<T extends string> = Record<FilterPrefix<CommandBuiltinId, T>, CommandKit | CommandCallable>;
export type CommandContribution = CommandDeclaration & CommandKit;
export type CommandMapContribution<T extends string> = Record<FilterPrefix<CommandBuiltinId, T>, CommandContribution>;

/** Adds an command (declaration or declaration+implementation) directly . */
export function addCommand(kind: CommandKind, in_: CommandIn) {
  const idParts = in_.id.split(".").map((p) => toCasing(p, Casing.CAMEL));
  const command: Command = {
    kind,
    type: "generic",
    ...in_,
    icon: typeof in_.icon === "string" ? makeIcon({ faName: in_.icon }) : in_.icon,
    id: in_.id,
    category: idParts[0],
    subcategory: idParts[1],
    path: idParts.slice(0, -1).join(" / "),
  };
  if (DECLARED_COMMANDS_BY_ID.value[in_.id] != null && (!IS_DEV || getCurrentInstance() == null)) {
    // hot-reloading re-registers commands (sometimes)
    throw new Error(`command already exists: ${in_.id} (${in_} != ${DECLARED_COMMANDS_BY_ID.value[in_.id]})`);
  }
  DECLARED_COMMANDS_BY_ID.value[in_.id as CommandBuiltinId] = command;
  triggerRef(DECLARED_COMMANDS_BY_ID);
}

/** Contributes commands globally (same implementation everywhere) */
export function provideCommands<T extends string>(map: Partial<CommandMapContribution<T>>) {
  Object.entries(map).forEach(([id, command]) =>
    addCommand("static", { ...(command as CommandIn), id: id as CommandBuiltinId }),
  );
}

/** Declares commands to be implemented virtually. */
export function declareCommands<T extends string>(
  map: Partial<CommandMapDeclaration<T>>,
): Partial<CommandMapDeclaration<T>> {
  Object.entries(map).forEach(([id, command]) =>
    addCommand("virtual", { ...(command as CommandIn), id: id as CommandBuiltinId }),
  );
  return map;
}

export function getCommand(id: CommandBuiltinId): Command {
  const command = DECLARED_COMMANDS_BY_ID.value[id];
  if (command == null) throw new Error(`command is not declared: ${id}`);
  return command;
}

/** A simple OR filter for Commands */
export type CommandFilter = {
  // wildcard match (lowercase, with '*' for variable length wildcard)
  wildcard?: string | string[];
};

/**
 * Filters commands with a simple OR filter of clauses.
 * The order of the input filter is preserved in order of matching.
 */
export function getCommandsLike(like: CommandFilter | string[]): Command[] {
  like = Array.isArray(like) ? { wildcard: like } : like;
  const wildcards = (like.wildcard == null ? [] : Array.isArray(like.wildcard) ? like.wildcard : [like.wildcard]).map(
    (w) => new RegExp("^" + w.toLowerCase().replace(/\*/g, ".*") + "$"),
  );
  const matches: Record<string, Command> = {};
  for (const wildcard of wildcards) {
    for (const command of DECLARED_COMMANDS.value) {
      if (matches[command.id] != null) continue; // already matched (preserve order)
      const idNorm = command.id.toLowerCase();
      if (wildcard.test(idNorm)) matches[command.id] = command;
    }
  }
  return Object.values(matches);
}

/**
 * Commands can be suppressed in the DOM with data-suppress-commands='mask1,mask2,...'.
 * If not specified, we default to the  DEFAULT_SUPPRESSED_COMMANDS per tag.
 * Suppressions accumulate up the DOM tree, and for simplicity you cannot un-suppress an command.
 * The mask is simply a prefix match.
 */
export const DEFAULT_SUPPRESSED_COMMANDS: Record<string, string[]> = {
  input: ["space.edit", "space.navigate", "space.select", "space.move"],
  textarea: ["space.edit", "space.navigate", "space.select", "space.move"],
  contenteditable: ["space.edit", "space.navigate", "space.select", "space.move"],
};

/** Finds an ancestor element suppressing the given command */
function getCommandSuppressor(id: CommandBuiltinId, el: HTMLElement): HTMLElement | null {
  while (el != null) {
    const elTag = el.tagName.toLowerCase();
    const suppress = el.getAttribute("data-suppress-commands");
    let masks: string[];
    if (suppress != null) {
      masks = suppress.split(",");
    } else if (DEFAULT_SUPPRESSED_COMMANDS[elTag] != null) {
      masks = DEFAULT_SUPPRESSED_COMMANDS[elTag];
    } else if (el.contentEditable == "true") {
      // we handle delete in our prosemirror views via custom commands, see src/ui/prosemirror/editor.ts
      masks = DEFAULT_SUPPRESSED_COMMANDS.contenteditable;
    } else {
      masks = [];
    }

    if (masks.some((mask) => id.startsWith(mask))) {
      return el; // suppressed
    } else {
      el = el.parentElement!;
    }
  }
  return null;
}

/** Whether the given command is enabled in the context. */
export function isCommandEnabled(command: Command, context?: CommandContext): boolean {
  if (command.isEnabled == null) {
    return true;
  } else if (typeof command.isEnabled == "function") {
    return command.isEnabled(command, context) ?? false;
  } else {
    return toValue(command.isEnabled);
  }
}

/** Triggers the bound command by id. */
export function fireCommandById(id: CommandBuiltinId, context?: CommandContext) {
  const command = getCommand(id);
  fireCommand(command, context, canvas.focusedViewComponents);
}

/** Triggers the bound command from a keyboard event. */
export function fireCommandFromEvent(command: Command, e: KeyboardEvent): boolean {
  // assemble current context
  const context: CommandContext = { event: e };
  if (canvas.selection != null && supergraph.getManyMaybe(canvas.selection.nodesPtr).length > 0) {
    context.nodes = supergraph.getManyMaybe(canvas.selection.nodesPtr);
  } // :IgnoreInspectionForCommand

  // bail if command is disabled or suppressed
  if (!isCommandEnabled(command, context)) {
    log.trace("command.disabled", command.id);
    return false;
  }
  const suppressor = getCommandSuppressor(command.id, e.target as HTMLElement);
  if (suppressor) {
    log.trace("command.suppressed", command.id, suppressor);
    return false;
  }

  // fire command
  const localViewsInOrder = collectViewComponentsUp(e.target as HTMLElement);
  // NOTE: concat local views and focused view components so local ones are preferred, but all are available
  return fireCommand(command, context, [...localViewsInOrder, ...canvas.focusedViewComponents]);
}

/**
 * Checks whether the context implements the command.
 * NOTE: does not 'call' the command, so we can't know if the command is refused dynamically.
 */
export function getImplementingCommand(
  command: Command,
  viewsInContext: ViewComponent[],
  context: CommandContext,
): CommandKit | null {
  if (command.kind == "static") {
    if (isCommandEnabled(command, context)) {
      return command;
    } else {
      return null;
    }
  } else if (command.kind == "virtual") {
    for (const view of viewsInContext) {
      const impl = view.exposed?.commands?.[command.id];
      if (typeof impl == "function") {
        return { command: impl };
      }
      if (impl != null && isCommandEnabled(command, context)) {
        return impl;
      }
    }
    if (command.command != null) {
      return { command: command.command };
    }
    return null;
  } else {
    throw new Error(`unexpected command kind: ${command.kind}`);
  }
}

/** Triggers the bound command from a given view (as starting point). */
export function fireCommand(
  command: Command,
  context: CommandContext = {},
  viewsInOrder: ViewComponent[] | null = canvas.focusedViewComponents,
) {
  if (!isCommandEnabled(command, context)) return false;
  if (command.kind == "static") {
    // static: just call callback directly
    log.trace("command.static", command.id);
    const ret = command.command!(command, context);
    return typeof ret === "boolean" ? ret : true;
  } else if (command.kind == "virtual") {
    // virtual: find first component implementing that command
    for (const view of viewsInOrder ?? []) {
      let impl = view.exposed?.commands?.[command.id];
      if (typeof impl == "function") impl = { command: impl };
      if (impl?.command != null && isCommandEnabled(command, context)) {
        log.trace("command.virtual", command.id);
        const ret = impl.command(command, context);
        if (typeof ret != "boolean" || ret === true) return true;
        /** else: keep searching up */
      }
    }
    if (command.command != null) {
      // default to static implementation if we have one
      const ret = command.command(command, context);
      if (typeof ret != "boolean" || ret === true) return true;
    }

    log.trace("command.virtual", command.id, "no implementing view", { context, viewsInOrder });
    if (isDeveloperMode.value) {
      toaster.debug({
        title: `Can't ${toValue(command.title)} Here`,
        text: `No view supports ${command.id}.`,
      });
    }
    return false; // no command found
  } else {
    throw new Error(`unexpected command kind: ${command.kind}`);
  }
}

// track implemented commands & maintain keybindings
// NOTE: implemented commands may contain disabled commands, we filter those at a later command to avoid updating this too often
//  (we evaluate the actual command to call only when firing the callback anyway)
const IMPLEMENTED_COMMANDS_BY_ID: Ref<Record<string, Command>> = shallowRef({});
export const IMPLEMENTED_COMMANDS: Readonly<Ref<Command[]>> = computed(() =>
  Object.values(IMPLEMENTED_COMMANDS_BY_ID.value),
);
let _isWatching = false;

/** Watches for changes in commands and updates the keytrap accordingly. Should only be called once. */
export function watchCommands() {
  if (_isWatching) throw new Error("watchCommands already called");
  _isWatching = true;
  return watch(
    [DECLARED_COMMANDS, canvas.focusedViewComponentsById],
    () => {
      const implementedCommands: Record<string, Command> = {};

      // static commands
      Object.values(DECLARED_COMMANDS.value)
        .filter((a) => a.kind == "static" || (a.kind == "virtual" && a.command != null))
        .forEach((a) => (implementedCommands[a.id] = a));

      // collect virtual commands bottom up
      for (const view of Object.values(canvas.focusedViewComponentsById.value)) {
        for (const [commandId, command] of Object.entries(view.exposed?.commands ?? {})) {
          if (implementedCommands[commandId] != null) continue; // already declared (either static or by lower view)
          const declaration = DECLARED_COMMANDS_BY_ID.value[commandId];
          if (declaration == null) throw new Error(`no declaration for virtual command: ${commandId}`);
          implementedCommands[commandId] = { ...declaration, ...command };
        }
      }

      // diff & update keytrap
      const oldImplemented = IMPLEMENTED_COMMANDS_BY_ID.value;
      const removedCommands = Object.keys(oldImplemented).filter((id) => implementedCommands[id] == null);
      const addedCommands = Object.keys(implementedCommands).filter((id) => oldImplemented[id] == null);
      removedCommands.forEach((id) => keytrap.unbind(oldImplemented[id].shortcuts ?? []));
      addedCommands.forEach((id) => {
        const command = implementedCommands[id];
        if ((command.shortcuts?.length ?? 0) > 0) {
          keytrap.bind(command.shortcuts!, (e) => fireCommandFromEvent(command, e), { key: id, replace: true });
        }
      });

      IMPLEMENTED_COMMANDS_BY_ID.value = implementedCommands;
    },
    { immediate: true },
  );
}

/** Gets the nodes in the context of an Command. Only returns the nodes if they are part of tbe same connection. */
export function getNodesForCommand<T extends NodeType>(
  command: Command,
  context: CommandContext | undefined,
  nodeFilter?: T[] | ((node: AnyNodeData) => node is NodeTypeMapping[T]),
): {
  connection: ConnectionBase<any, any> | null;
  graph: ReadNodeGraph | null;
  nodes: NodeTypeMapping[T][];
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
  } else {
    return { connection: null, graph: null, nodes: [] };
  }
  // NOTE: we ignore the current inspection because it leads to unexpected behavior :IgnoreInspectionForCommand

  // find link
  let link = null;
  for (const nodePtr of nodesPtr) {
    link = supergraph.getLink(nodePtr);
    if (link == null) break;
  }
  if (link == null) return { connection: null, graph: null, nodes: [] };
  const { connection, graph } = link;

  // get and deduplicate nodes
  const nodes: NodeTypeMapping[T][] = [];
  const nodesById: Record<string, AnyNodeData> = {};
  for (const nodePtr of nodesPtr) {
    const node = graph.get(nodePtr);
    if (node == null) continue;
    if (Array.isArray(nodeFilter) && !nodeFilter.includes(node.metatype as unknown as T)) {
      continue;
    } else if (typeof nodeFilter == "function" && !nodeFilter(node)) {
      continue;
    } else if (nodesById[node.id!] == null) {
      nodes.push(node as NodeTypeMapping[T]);
      nodesById[node.id!] = node;
    }
  }
  return { connection, graph, nodes };
}

//
// Node context commands
//

export const NON_DUPLICATABLE_NODE_TYPES = [
  ...ROOT_NODE_TYPES,
  NodeType.PACKAGE,
  NodeType.TRANSITION,
  NodeType.THREAD,
  NodeType.MESSAGE,
  NodeType.MEMBERSHIP,
  ...RUNTIME_NODE_TYPES,
  ...RESOURCE_NODE_TYPES,
  ...COSMOS_NODE_TYPES,
];
export const NON_ARCHIVEABLE_NODE_TYPES = [
  ...ROOT_NODE_TYPES,
  ...RESOURCE_NODE_TYPES,
  NodeType.PACKAGE,
  NodeType.MEMBERSHIP,
  NodeType.RUN,
  NodeType.SPAN,
  NodeType.THREAD, // NOTE :Incomplete: removing Threads causes annoying issues
  // NodeType.MESSAGE,
  NodeType.MEMBERSHIP,
];
export const NON_DELETABLE_NODE_TYPES = [
  ...ROOT_NODE_TYPES,
  NodeType.PACKAGE,
  NodeType.MESSAGE,
  NodeType.RUN,
  NodeType.SPAN,
  NodeType.THREAD, // NOTE :Incomplete: removing Threads causes annoying issues
  ...RESOURCE_NODE_TYPES,
  ...COSMOS_NODE_TYPES,
];

export const SCALAR_CONTEXT_COMMANDS: CommandBuiltinId[] = ["space.edit.rename"];
export const NODE_CONTEXT_COMMANDS: CommandBuiltinId[] = [
  "space.edit.duplicate",
  "space.edit.archive",
  "space.edit.delete",
];
export const FIELD_CONTEXT_COMMANDS: CommandBuiltinId[] = [];
export const BLOCK_CONTEXT_COMMANDS: CommandBuiltinId[] = [];
export const RECORD_CONTEXT_COMMANDS: CommandBuiltinId[] = [];
export const ACTION_CONTEXT_COMMANDS: CommandBuiltinId[] = [];
export const RUN_CONTEXT_COMMANDS: CommandBuiltinId[] = ["runtime.run.pause", "runtime.run.resume", "runtime.run.kill"];
export const INTERRUPTION_CONTEXT_COMMANDS: CommandBuiltinId[] = [
  "runtime.interruption.resume",
  "runtime.interruption.cancel",
];
export const TRANSITION_CONTEXT_COMMANDS: CommandBuiltinId[] = ["flow.edit.splitTransition"];
export const RESOURCE_CONTEXT_COMMANDS: CommandBuiltinId[] = [
  "resource.status.activate",
  // "resource.status.suspend", // (not supported yet)
  "resource.status.decommission",
];
export const MESSAGE_CONTEXT_COMMANDS: CommandBuiltinId[] = [
  "chat.message.edit",
  "chat.message.copy",
  "chat.message.reply",
];

export const CONTEXT_COMMANDS_BY_TYPE: Partial<Record<NodeType, CommandBuiltinId[]>> = {
  [NodeType.BLOCK]: BLOCK_CONTEXT_COMMANDS,
  [NodeType.RECORD]: RECORD_CONTEXT_COMMANDS,
  [NodeType.ACTION]: ACTION_CONTEXT_COMMANDS,
  [NodeType.TRANSITION]: TRANSITION_CONTEXT_COMMANDS,
  [NodeType.RUN]: RUN_CONTEXT_COMMANDS,
  [NodeType.INTERRUPTION]: INTERRUPTION_CONTEXT_COMMANDS,
};
for (const nodeType of RESOURCE_NODE_TYPES) {
  CONTEXT_COMMANDS_BY_TYPE[nodeType] = [...(CONTEXT_COMMANDS_BY_TYPE[nodeType] ?? []), ...RESOURCE_CONTEXT_COMMANDS];
}

/** Gets the base Commands for a Node. */
export function getNodeCommands(node: AnyNodeData): Command[] {
  const commands: CommandBuiltinId[] = [];
  const nodeType = node.metatype as unknown as NodeType;
  // duplicate/archive/delete
  if (!NON_DUPLICATABLE_NODE_TYPES.includes(nodeType)) {
    commands.push("space.edit.duplicate");
  }
  if (!NON_ARCHIVEABLE_NODE_TYPES.includes(nodeType)) {
    commands.push("space.edit.archive");
  }
  if (!NON_DELETABLE_NODE_TYPES.includes(nodeType)) {
    commands.push("space.edit.delete");
  }
  // rename
  if ("title" in node || "name" in node) {
    commands.push("space.edit.rename");
  }
  // general context commands
  if (CONTEXT_COMMANDS_BY_TYPE[nodeType] != null) {
    commands.push(...CONTEXT_COMMANDS_BY_TYPE[nodeType]!);
  }
  return commands.map(getCommand);
}

//
// Flow
//

// flow
declareCommands<"flow">({
  "flow.edit.createAction": {
    icon: makeIcon(NodeTypeOptionInfo[NodeType.ACTION]!.icon!),
    title: "Create Action",
    text: "Create a new action",
  },
  "flow.edit.splitTransition": {
    icon: "fas fa-scissors",
    title: "Split Link",
    text: "Split this link",
  },
});

//
// Code
//

declareCommands<"code">({
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

//
// Text
//

declareCommands<"text">({
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
    text: "Format as code",
    shortcuts: ["mod+e"],
  },
  "text.format.spoiler": {
    type: "toggle",
    icon: "fas fa-eye",
    title: "Spoiler",
    text: "Format as spoiler",
    shortcuts: ["mod+s"],
  },
});

//
// Chat
//

declareCommands<"chat">({
  "chat.message.copy": {
    title: "Copy",
    text: "Copy the Message",
    icon: "fas fa-copy",
  },
  "chat.message.reply": {
    title: "Reply",
    text: "Reply to the Message",
    icon: "fas fa-reply",
  },
  "chat.message.forward": {
    title: "Forward",
    text: "Forward the Message",
    icon: "fas fa-share",
  },
  "chat.message.edit": {
    title: "Edit",
    text: "Edit the Message",
    icon: "fas fa-pencil",
  },
});
