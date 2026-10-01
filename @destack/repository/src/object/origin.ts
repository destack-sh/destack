import { principal } from "@destack/access";
import { Subject } from "@destack/sync";
import { identifier, schema } from "@destack/schema";

/** A host as a subject, read from the key in the repository's host field: the host itself in its account, never a set of hosts. */
const HostSubject = Subject.omit({ relation: true }).extend({
    packageId: schema.literal(principal.host.definition.packageId),
    type: schema.literal(principal.host.name),
    id: identifier("host"),
});

/** Where a repository's authoritative history lives: platform storage, GitHub, any Git remote, or a host. */
export const HOSTINGS = ["platform", "github", "git", "host"] as const;

/** The ways the platform authenticates to a Git remote. */
export const AUTHENTICATIONS = ["anonymous", "secret"] as const;

/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
export const RepositoryOrigin = schema.union([
    schema.object({ hosting: schema.literal("platform") }),
    schema.object({ hosting: schema.literal("host"), host: HostSubject }),
    schema.object({
        hosting: schema.literal("github"),
        remote: schema.url(),
        connectedAccountId: identifier("connected-account"),
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
        secretSpaceId: identifier("space"),
        secretId: identifier("secret"),
    }),
]);
/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
export type RepositoryOrigin = schema.Infer<typeof RepositoryOrigin>;

/** The fields naming a repository's origin, as callers write them. */
export const ORIGIN_FIELDS = [
    "hosting",
    "host",
    "remote",
    "authentication",
    "connectedAccountId",
    "secretSpaceId",
    "secretId",
] as const;

/** The origin fields a repository's update may change: every one but its host, which keeps reporting. */
export const UPDATED_ORIGIN_FIELDS = ORIGIN_FIELDS.filter(
    (name): name is Exclude<(typeof ORIGIN_FIELDS)[number], "host"> => name !== "host",
);

/** The columns naming a repository's origin and its identity in storage or at GitHub. */
export const ORIGIN_COLUMNS = [...ORIGIN_FIELDS, "provider", "providerRepositoryId"] as const;
