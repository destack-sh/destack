import { PermissionReference } from "@destack/access";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";

/** Where a moved scope is served now: its scope and the machine serving it. */
export const ScopeMove = schema.object({
    /** The moved scope. */
    scope: schema.string().min(1),
    /** The machine serving it now. */
    machine: schema.identifier("machine"),
});
/** Where a moved scope is served now. */
export type ScopeMove = schema.Infer<typeof ScopeMove>;

/** Where a scope is served within its installation: its scope and the partition serving it. */
export const PartitionRoute = schema.object({
    /** The scope. */
    scope: schema.string().min(1),
    /** The partition serving it. */
    partition: schema.string().min(1),
});
/** Where a scope is served within its installation. */
export type PartitionRoute = schema.Infer<typeof PartitionRoute>;

/** How long until a refused call may be retried, as Retry-After states it. */
export const RetryAfter = schema.object({
    /** The wait, in milliseconds. */
    retryAfter: schema.number().nonnegative(),
});
/** How long until a refused call may be retried. */
export type RetryAfter = schema.Infer<typeof RetryAfter>;

/** A grant's refusal of a permission on an object: approval or a wider grant is required. */
export const GrantRefusal = schema.object({
    /** The permission the call needs. */
    permission: PermissionReference,
    /** The object the permission applies to. */
    object: ObjectReference,
    /** The grant that refused it. */
    grant: schema.identifier("grant"),
});
/** A grant's refusal of a permission on an object. */
export type GrantRefusal = schema.Infer<typeof GrantRefusal>;

/** The issues a schema finds in an input or output, each with its message and path. */
export const Issues = Object.assign(
    schema.object({
        /** Each issue's message and path. */
        issues: schema.array(
            schema.object({ message: schema.string(), path: schema.array(schema.string()) }),
        ),
    }),
    {
        /** Read a schema's issues as their messages and paths of property names. */
        of(
            issues: readonly {
                readonly message: string;
                readonly path?:
                    | readonly (PropertyKey | { readonly key: PropertyKey })[]
                    | undefined;
            }[],
        ): Issues {
            return {
                issues: issues.map((issue) => ({
                    message: issue.message,
                    path: (issue.path ?? []).map((part) =>
                        String(typeof part === "object" ? part.key : part),
                    ),
                })),
            };
        },
    },
);
/** The issues a schema finds in an input or output. */
export type Issues = schema.Infer<typeof Issues>;
