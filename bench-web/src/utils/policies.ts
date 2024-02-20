import type { TypePolicies } from "@apollo/client";

const useIncoming = {
  merge: (existing: any, incoming: any) => incoming,
};

export const TYPE_POLICIES: TypePolicies = {
  User: {
    fields: {
      notifications: useIncoming,
    },
  },
  File: {
    fields: {
      statements: useIncoming,
    },
  },
  Statement: {
    fields: {
      fields: useIncoming,
      triggers: useIncoming,
      tags: useIncoming,
      descendants: useIncoming,
      children: useIncoming,
      referencedBy: useIncoming,
    },
  },
  InterpSymbol: {
    fields: {
      fields: useIncoming,
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
  RunConnection: {
    fields: {
      edges: useIncoming,
    },
  },
  Query: {
    fields: {
      searchRecords: useIncoming,
    },
  },
};
