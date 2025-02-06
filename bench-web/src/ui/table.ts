import { NodeTypeMapping } from "@/proto/wire";
import { ViewData } from "@/proto/wire";
import { ReadNodeGraph } from "@/language/core/graph";
import { NodeType } from "@/proto/wire";
import { ActionMapImplementation, declareActions } from "@/ui/action";
import { useNodeListActions } from "@/ui/list";
import { Ref } from "vue";
import { Transaction } from "@/language/runtime/transaction";

// table
declareActions<"table">({
  // create
  "table.create.record": {
    icon: "fas fa-plus",
    title: "Create Record",
    text: "Create a new record",
  },
  // column
  "table.column.sortAscending": {
    icon: "fas fa-arrow-up",
    title: "Sort Ascending",
    text: "Sort this column in ascending order",
  },
  "table.column.sortDescending": {
    icon: "fas fa-arrow-down",
    title: "Sort Descending",
    text: "Sort this column in descending order",
  },
  "table.column.filter": {
    icon: "fas fa-filter",
    title: "Filter",
    text: "Filter this column",
  },
  "table.column.wrap": {
    icon: "fas fa-align-justify",
    title: "Wrap",
    text: "Wrap this column",
  },
  "table.column.hide": {
    type: "toggle",
    icon: "fas fa-eye-slash",
    title: "Hide",
    text: "Hide this column",
  },
});

/** Actions for a table */
export function useNodeTableActions<T extends NodeType>(options: {
  nodeType: T;
  self: Ref<ViewData | null>;
  graph: ReadNodeGraph;
  list: Ref<NodeTypeMapping[T][]>;
  txFactory: () => Transaction;
  create?: (anchor: "before" | "after", node: NodeTypeMapping[T] | null) => NodeTypeMapping[T];
  enabled?: Ref<boolean>;
  ignoreOrder?: boolean;
}): Partial<ActionMapImplementation<"space.move" | "space.navigate" | "space.select" | "list.create">> {
  return useNodeListActions({
    ...options,
    graph: options.graph,
    list: options.list,
    txFactory: options.txFactory,
    create: options.create,
    enabled: options.enabled,
    ignoreOrder: options.ignoreOrder,
  });
}
