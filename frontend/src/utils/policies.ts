// TODO @Robustness: check if use incoming for merged arrays is always fine
const useIncoming = {
  merge: (existing: any, incoming: any) => incoming,
};

export const TYPE_POLICIES = {
  File: {
    fields: {
      statements: useIncoming,
    },
  },
  Statement: {
    fields: {
      typeNodes: useIncoming,
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
};
