import { ReadNodeGraph } from "@/language/core/graph";
import { Transaction } from "@/language/core/transaction";
import { ClaimData, NodeType } from "@/proto/wire";

/** Create a Claim */
export function createClaim(tx: Transaction, graph: ReadNodeGraph, options: { claim: Partial<ClaimData> }): ClaimData {
  const siblings = graph.getChildren(options.claim.parentPtr!, NodeType.CLAIM);
  const claim = tx.create({ metatype: NodeType.CLAIM, ...options.claim });
  return claim;
}
