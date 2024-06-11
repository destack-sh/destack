import { NodeType, ObjectType, ViewData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { NodeGraph } from "@/system/graph";
import { fabricate } from "@/system/graph.test";
import { fixOrderKeys } from "@/system/lang";
import { TransactionBuilder, editGraph } from "@/system/transaction";
import { v4 } from "uuid";
import { describe, expect, test } from "vitest";

describe("order keys", () => {
  test("fix", () => {
    // a1: 1 duplicate, a2: 2 duplicates
    // TODO :Robustness: fix order keys if some keys are invalid
    const badNodes = ["a0", "a1", "a1", "a2", "a2", "a2", "a3"].map((orderKey) =>
      fabricate(ObjectType.VIEW, { set: { orderKey }, unset: ["parentPtr", "valuePacked"] }),
    );

    // 'fix' order keys with minimal edits
    const tx = new TransactionBuilder({}, v4(), toNodeReference(fabricate(ObjectType.USER)));
    fixOrderKeys(tx, badNodes);
    expect(tx.edits.length).toBe(3);

    // apply edits to graph and check the order is as given
    const graph = new NodeGraph({ scope: {}, nodeTypes: [NodeType.VIEW] });
    graph.extend(...badNodes);
    editGraph(graph, tx.edits);
    const fixedNodes = graph.nodes as ViewData[];
    expect(fixedNodes.map((n) => n.id)).toEqual(badNodes.map((n) => n.id));
  });
});
