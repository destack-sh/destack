import { relayStylePagination } from "@apollo/client/utilities";

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
      fields: useIncoming,
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
      searchDataset: relayStylePagination(["statementId", "query", "sort", "limit", "count"]),
    },
  },
};
