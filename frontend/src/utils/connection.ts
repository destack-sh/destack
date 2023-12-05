import type { PageInfo } from "@/gql/graphql";

export type Connection<T, C = string> = {
  __typename?: C;
  totalCount?: number;
  edges: { __typename: string; cursor: string; node: T & { id: string } }[];
  pageInfo: PageInfo;
};

export function emptyConnection<T, C>(typename: C): Connection<T, C> {
  return {
    __typename: typename,
    totalCount: 0,
    edges: [],
    pageInfo: {
      __typename: "PageInfo",
      hasNextPage: false,
      hasPreviousPage: false,
      startCursor: "",
      endCursor: "",
    },
  };
}

export function getUpdatedConnectionQuery<T, C = string>(
  node: T & { id: string },
  prev: Connection<T> | undefined,
  maxLength?: number,
  insertAt?: "start" | "end",
  typename?: string
): Connection<T> {
  // cursor is base64-encoded Connection:{nodeId}
  const newEdge = {
    __typename: typename ?? "NodeEdge",
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
  const edges = insertAt == "start" ? [newEdge, ...prev.edges] : [...prev.edges, newEdge];
  return {
    ...prev,
    totalCount: (prev.totalCount ?? 0) + 1,
    edges: maxLength ? edges.slice(0, maxLength) : edges,
    pageInfo,
  };
}

export function getUpdatedConnectionQueryMany<T>(
  nodes: (T & { id: string })[],
  prev: Connection<T> | undefined,
  maxLength?: number
): Connection<T> {
  return nodes
    .slice()
    .reverse()
    .reduce((prev, node) => getUpdatedConnectionQuery(node, prev, maxLength), prev) as Connection<T>;
}
