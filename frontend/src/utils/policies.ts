const useIncoming = {
  merge: (existing: any, incoming: any) => incoming,
};
const mergePaginated = {
  keyArgs: ["filters"],
  merge: (existing: any, incoming: any) => {
    // merge existing.edges and incoming.edges, deduplicate by node.id
    let edges: Record<string, any> = {};
    if (existing?.edges) {
      for (const edge of existing.edges) {
        edges[edge.node.__ref] = edge;
      }
    }
    if (incoming?.edges) {
      for (const edge of incoming.edges) {
        edges[edge.node.__ref] = edge;
      }
    }
    edges = Object.values(edges);
    return {
      ...incoming,
      edges,
    };
  },
  read: (existing: any) => existing,
};

export const TYPE_POLICIES = {
  User: {
    fields: {
      notifications: {
        edges: useIncoming,
      },
    },
  },
  File: {
    fields: {
      statements: useIncoming,
    },
  },
  Statement: {
    fields: {
      typeNodes: useIncoming,
      records: mergePaginated,
    },
  },
  InterpSymbol: {
    fields: {
      typeNodes: useIncoming,
    },
  },
  InterpFile: {
    fields: {
      symbols: useIncoming,
    },
  },
  InterpModule: {
    fields: {
      files: useIncoming,
      errors: useIncoming,
      dependencies: useIncoming,
      staleSymbols: useIncoming,
    },
  },
  Project: {
    fields: {
      versions: useIncoming,
    },
  },
  ProjectVersion: {
    fields: {
      files: useIncoming,
    },
  },
  ExecutionConnection: {
    fields: {
      edges: useIncoming,
    },
  },
};
