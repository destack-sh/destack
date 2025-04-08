import { NodeType, ObjectType, ViewData } from "@/proto/wire";
import { EMPTY_SCOPE, toNodeRef } from "@/proto/wiring";
import { NodeGraph } from "@/language/core/graph";
import { fabricate } from "@/language/core/graph.test";
import { TransactionBuilder, TransactionState, editGraph } from "@/language/core/transaction";
import { uuidt } from "@/utils/uuidt";
import { describe, expect, test } from "vitest";
import { fixOrderKeys } from "@/language/core/order";

describe("order keys", () => {
  test("fix", () => {
    // a1: 1 duplicate, a2: 2 duplicates
    // NOTE :Robustness: should fix order keys if some keys are invalid
    const badNodes = ["a0", "a1", "a1", "a2", "a2", "a2", "a3"].map((orderKey) =>
      fabricate(ObjectType.VIEW, { set: { orderKey }, unset: ["parentPtr"] }),
    );

    // 'fix' order keys with minimal edits
    const tx = new TransactionBuilder({
      subject: toNodeRef(fabricate(ObjectType.USER)),
      state: new TransactionState(uuidt(), EMPTY_SCOPE),
    });
    fixOrderKeys(tx, badNodes);
    expect(tx.edits.length).toBe(3);

    // apply edits to graph and check the order is as given
    const graph = new NodeGraph({ scope: EMPTY_SCOPE, nodeTypes: new Set([NodeType.VIEW]) });
    graph.extend(...badNodes);
    editGraph(graph, tx.edits);
    const fixedNodes = graph.nodes as ViewData[];
    expect(fixedNodes.map((n) => n.id)).toEqual(badNodes.map((n) => n.id));
  });
});
