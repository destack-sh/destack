import { ReadNodeGraph } from "@/language/core/graph";
import { moveNodes } from "@/language/core/node";
import { Transaction } from "@/language/runtime/transaction";
import { NodeTypeMapping, ObjectType, PROPERTY_ENUM_BY_TYPE, ViewData } from "@/proto/wire";

import { NodeType } from "@/proto/wire";
import { canvas } from "@/system/space";
import { CommandContext, CommandMapKit, declareCommands } from "@/ui/command";
import { computed, Ref } from "vue";

// list
declareCommands<"list">({
  // create
  "list.create.above": {
    icon: "fas fa-arrow-up",
    title: "Create Above",
    text: "Create a new item above this item",
  },
  "list.create.below": {
    icon: "fas fa-arrow-down",
    title: "Create Below",
    text: "Create a new item below this item",
  },
});

/** Commands to move up/down in a flat node level (assumes all the parents are the same, i.e. all nodes are siblings) */
export function useNodeListCommands<T extends NodeType>(options: {
  nodeType: T;
  self: Ref<ViewData | null>;
  graph: ReadNodeGraph;
  list: Ref<NodeTypeMapping[T][]>;
  txFactory: () => Transaction;
  create?: (anchor: "before" | "after", node: NodeTypeMapping[T] | null) => NodeTypeMapping[T];
  enabled?: Ref<boolean>;
  ignoreOrder?: boolean;
}): Partial<CommandMapKit<"space.move" | "space.navigate" | "space.select" | "list.create">> {
  const { nodeType, self, graph, list, txFactory, enabled, create, ignoreOrder } = options;

  const nodeProperties = PROPERTY_ENUM_BY_TYPE[nodeType as unknown as ObjectType];
  const shouldOrder = !ignoreOrder && (nodeProperties as any)?.["orderKey"] != null;

  // get index region of the given nodes in the list
  function getItems(ctx: CommandContext): { start: number; end: number; items: NodeTypeMapping[T][] } {
    const start = list.value.findIndex((node) => ctx.nodes?.some((n) => n.id == node.id));
    const end =
      list.value.length - 1 - [...list.value].reverse().findIndex((node) => ctx.nodes?.some((n) => n.id == node.id));
    return { start, end, items: list.value.slice(start, end + 1) };
  }

  const implementation: Partial<
    CommandMapKit<"space.move" | "space.navigate" | "space.select" | "list.create">
  > = {
    // navigate
    "space.navigate.up": {
      isEnabled: enabled,
      command: (command, ctx) => {
        // navigate focus/inspect one position up
        const { start } = getItems(ctx);
        if (start < 1) return;
        const nextItem = list.value[start - 1];
        if (nextItem != null) {
          if (self.value != null) {
            canvas.focus({ node: nextItem, view: self.value });
          }
          canvas.select([nextItem]);
          canvas.inspect({ node: nextItem, view: self.value });
        }
      },
    },
    "space.navigate.down": {
      isEnabled: enabled,
      command: (command, ctx) => {
        // navigate focus/inspect one position down
        const { end } = getItems(ctx);
        if (end >= list.value.length - 1) return;
        const nextItem = list.value[end + 1];
        if (nextItem != null) {
          if (self.value != null) {
            canvas.focus({ node: nextItem, view: self.value });
          }
          canvas.select([nextItem]);
          canvas.inspect({ node: nextItem, view: self.value });
        }
      },
    },
    // move
    "space.move.up": {
      isEnabled: computed(() => enabled?.value !== false && shouldOrder),
      command: (command, ctx) => {
        // move items one position up
        const { start, items } = getItems(ctx);
        if (start < 1) return;
        moveNodes(txFactory(), graph, items, { anchor: "before", target: list.value[start - 1] });
      },
    },
    "space.move.down": {
      isEnabled: computed(() => enabled?.value !== false && shouldOrder),
      command: (command, ctx) => {
        // move items one position down
        const { end, items } = getItems(ctx);
        if (end >= list.value.length - 1) return;
        moveNodes(txFactory(), graph, items, { anchor: "after", target: list.value[end + 1] });
      },
    },
    // select
    "space.select.all": {
      isEnabled: enabled,
      command: (command, ctx) => {
        canvas.select(list.value);
      },
    },
    "space.select.up": {
      isEnabled: enabled,
      command: (command, ctx) => {
        // select one item 'up' (and focus on that)
        const { start, end, items } = getItems(ctx);
        if (items.length > 1 && canvas.inspection?.id == list.value[end]?.id) {
          // shrinking up
          canvas.select(list.value.slice(start, end));
          if (list.value[end - 1] != null) canvas.inspect({ node: list.value[end - 1], view: self.value });
        } else if (start > 0) {
          // expanding up
          canvas.select(list.value.slice(start - 1, end + 1));
          if (list.value[start - 1] != null) canvas.inspect({ node: list.value[start - 1], view: self.value });
        }
      },
    },
    "space.select.down": {
      isEnabled: enabled,
      command: (command, ctx) => {
        // select one item 'down' (and focus on that)
        const { start, end, items } = getItems(ctx);
        if (items.length > 1 && canvas.inspection?.id == list.value[start]?.id) {
          // shrinking down
          canvas.select(list.value.slice(start + 1, end + 1));
          if (list.value[start + 1] != null) canvas.inspect({ node: list.value[start + 1], view: self.value });
        } else if (end < list.value.length - 1) {
          // expanding down
          canvas.select(list.value.slice(start, end + 2));
          if (list.value[end + 1] != null) canvas.inspect({ node: list.value[end + 1], view: self.value });
        }
      },
    },
    // list
    "list.create.above": {
      isEnabled: enabled,
      command: (command, ctx) => {
        if (create == null) return false;
        const { items } = getItems(ctx);
        const node = create?.("before", items[0]);
        canvas.select([node]);
        canvas.inspect({ node, view: self.value });
      },
    },
    "list.create.below": {
      isEnabled: enabled,
      command: (command, ctx) => {
        if (create == null) return false;
        const { items } = getItems(ctx);
        const node = create?.("after", items[items.length - 1]);
        canvas.select([node]);
        canvas.inspect({ node, view: self.value });
      },
    },
  };

  return implementation;
}
