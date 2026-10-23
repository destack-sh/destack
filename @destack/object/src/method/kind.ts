/** The kinds of method. */
export const METHOD_KINDS = [
    "get",
    "list",
    "create",
    "createMany",
    "update",
    "updateMany",
    "delete",
    "deleteMany",
    "custom",
    "restore",
    "purge",
    "move",
    "transition",
    "relationships",
    "grant",
    "revoke",
    "invitations",
    "invite",
    "accept",
    "withdraw",
    "explain",
    "bind",
    "revert",
    "edit",
    "contains",
] as const;
/** The kind of a method. */
export type MethodKind = (typeof METHOD_KINDS)[number];

/** The kinds whose methods always load their target before they run. */
export type TargetKind =
    | "get"
    | "update"
    | "delete"
    | "custom"
    | "restore"
    | "purge"
    | "move"
    | "transition"
    | "edit"
    | "contains"
    | "revert";
