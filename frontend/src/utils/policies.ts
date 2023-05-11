import { relayStylePagination } from "@apollo/client/utilities";

const useIncoming = {
  merge: (existing: any, incoming: any) => incoming,
};

const filteredRelayPagination = relayStylePagination(["filters"]);
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
      records: {
        // proxy read/merge for filtered relay pagination to also store args for cache.modify  :StatementRecordsView
        // we need the arguments (filters & pagination args) to modify the cache properly
        // see https://github.com/apollographql/apollo-client/issues/6394#issuecomment-656193666 for the approach
        // and https://www.apollographql.com/docs/react/caching/cache-interaction/#using-cachemodify
        read(existing: any, options: any) {
          return filteredRelayPagination.read?.(existing?.value, options);
        },
        merge(existing: any, incoming: any, options: any) {
          return {
            value: filteredRelayPagination.merge?.(existing?.value, incoming, options),
            args: options.args,
          };
        },
      },
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
