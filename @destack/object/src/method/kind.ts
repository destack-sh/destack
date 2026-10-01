/** The kinds of method. */
export const METHOD_KINDS = [
    "get",
    "list",
    "create",
    "update",
    "updateMany",
    "delete",
    "custom",
    "restore",
    "purge",
    "move",
    "transition",
    "relationships",
    "grant",
    "revoke",
    "proposals",
    "propose",
    "accept",
    "decline",
    "explain",
    "detach",
    "revert",
    "edit",
] as const;
/** The kind of a method. */
export type MethodKind = (typeof METHOD_KINDS)[number];
