import type { PageInfo } from "@/gql/graphql";

type Connection<T> = {
  totalCount: number;
  edges: { cursor: string; node: T & { id: string } }[];
  pageInfo: PageInfo;
};

export function getUpdatedConnectionQuery<T>(
  node: T & { id: string },
  prev?: Connection<T>,
  maxLength?: number
): Connection<T> {
  // cursor is base64-encoded Connection:{nodeId}
  const newEdge = {
    __typename: "NodeEdge",
    // not sure what to put here, it's a strawberry internal
    // should probably update all other edges' cursors as well
    // :ArrayConnections
    cursor: btoa(`arrayconnection:0`),
    node,
  };

  if (prev == null) {
    // first node, return directly
    return {
      totalCount: 1,
      edges: [newEdge],
      pageInfo: {
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: newEdge.cursor,
        endCursor: newEdge.cursor,
      },
    };
  }

  const index = prev.edges.findIndex((edge) => edge.node.id === node.id);
  if (index >= 0) {
    // update is automatic in Apollo cache
    return prev;
  }
  // insert into edges if it's new, update count and page info
  const pageInfo = {
    ...prev.pageInfo,
    startCursor: newEdge.cursor,
    hasPreviousPage: false,
  };
  const edges = [newEdge, ...prev.edges];
  return {
    ...prev,
    totalCount: (prev.totalCount ?? 0) + 1,
    edges: maxLength ? edges.slice(0, maxLength) : edges,
    pageInfo,
  };
}
