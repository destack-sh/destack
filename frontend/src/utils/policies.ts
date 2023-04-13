// TODO @Robustness: check if use incoming for merged arrays is always fine
const useIncoming = {
  merge: (existing: any, incoming: any) => incoming,
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
      records: useIncoming,
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
