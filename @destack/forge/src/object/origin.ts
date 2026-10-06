import { principal } from "@destack/access";
import { ObjectReference, Subject } from "@destack/sync";
import { schema } from "@destack/schema";

/** A machine as a subject, read from the key in the repository's machine field: the machine itself in its account, never a set of machines. */
const MachineSubject = Subject.omit({ relation: true }).extend({
    packageId: schema.literal(principal.machine.definition.packageId),
    type: schema.literal(principal.machine.name),
    id: schema.identifier("machine"),
});

/** Where a repository's authoritative history lives: platform storage, GitHub, any Git remote, or a machine. */
export const HOSTINGS = ["platform", "github", "git", "machine"] as const;

/** The ways the platform authenticates to a Git remote. */
export const AUTHENTICATIONS = ["anonymous", "secret"] as const;

/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
export const RepositoryOrigin = schema.union([
    schema.object({ hosting: schema.literal("platform") }),
    schema.object({ hosting: schema.literal("machine"), machine: MachineSubject }),
    schema.object({
        hosting: schema.literal("github"),
        remote: schema.url(),
        connectedAccountId: schema.identifier("connected-account"),
    }),
    schema.object({
        hosting: schema.literal("git"),
        remote: schema.url(),
        authentication: schema.literal("anonymous"),
    }),
    schema.object({
        hosting: schema.literal("git"),
        remote: schema.url(),
        authentication: schema.literal("secret"),
        secret: ObjectReference,
    }),
]);
/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
export type RepositoryOrigin = schema.Infer<typeof RepositoryOrigin>;

/** The fields naming a repository's origin, as callers write them. */
export const ORIGIN_FIELDS = [
    "hosting",
    "machine",
    "remote",
    "authentication",
    "connectedAccountId",
    "secret",
] as const;

/** The origin fields a repository's update may change: every one but its machine, which keeps reporting. */
export const MOVABLE_ORIGIN_FIELDS = ORIGIN_FIELDS.filter(
    (name): name is Exclude<(typeof ORIGIN_FIELDS)[number], "machine"> => name !== "machine",
);

/** The columns naming a repository's origin and its identity in storage or at GitHub. */
export const OriginColumns = schema.object({
    /** Where the repository is hosted. */
    hosting: schema.enum(HOSTINGS),
    /** The machine serving a machine repository. */
    machine: schema.string().nullable(),
    /** The remote of a GitHub, Git or platform repository. */
    remote: schema.string().nullable(),
    /** How a Git repository authenticates. */
    authentication: schema.enum(AUTHENTICATIONS).nullable(),
    /** The connected account opening a GitHub repository. */
    connectedAccountId: schema.identifier("connected-account").nullable(),
    /** The vault secret a Git repository authenticates with. */
    secret: ObjectReference.nullable(),
    /** The storage provider of a platform repository. */
    provider: schema.string().nullable(),
    /** The repository's identity in storage or at GitHub. */
    providerRepositoryId: schema.string().nullable(),
});
/** The columns naming a repository's origin and its identity in storage or at GitHub. */
export type OriginColumns = schema.Infer<typeof OriginColumns>;

/** The origin an update moves to, prepared at the revision the transaction requires, none when it keeps its origin. */
export const OriginMove = schema
    .object({
        /** The revision the move was prepared at. */
        revision: schema.int(),
        /** The origin columns the update records. */
        columns: OriginColumns,
    })
    .nullable();
/** The origin an update moves to, prepared at the revision the transaction requires, none when it keeps its origin. */
export type OriginMove = schema.Infer<typeof OriginMove>;
