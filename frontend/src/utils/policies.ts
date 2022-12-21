export const TYPE_POLICIES = {
  File: {
    fields: {
      statements: {
        merge: (existing: any[], incoming: any[]) => {
          // always use the incoming statements
          // TODO @Robustness: fix File.statements cache merge policy (consider filters, etc.)
          return [...incoming];
        },
      },
    },
  },
};
