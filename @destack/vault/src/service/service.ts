import { schema, identifier } from "@destack/schema";
import { defineProcedure, Creation, Mutation } from "@destack/service/procedure";
import { PackageId } from "@destack/package";
import definition from "../../destack.json" with { type: "json" };
import { Secret, SecretVersion, SecretValue } from "../secret/index.ts";
import { PageRequest, page } from "@destack/service/page";

/** The space routing every vault request. */
export const VaultScope = schema.object({ spaceId: identifier("space") });
/** An exact secret within a space. */
export const SecretKey = VaultScope.extend({ secretId: identifier("secret") });
/** An exact stored value version. */
export const VersionKey = SecretKey.extend({ version: schema.number().int().positive() });
/** Cursor pagination ordered by immutable identity or version. */
const pagination = PageRequest.extend({
    limit: schema.number().int().min(1).max(100).optional(),
    cursor: schema.string().max(2048).optional(),
});
/** Optimistic secret mutation and durable retry identity. */
const mutation = SecretKey.extend(Mutation.shape);

/** Secret administration and explicit plaintext access. */
export const vaultService = {
    /** Provisioned vault metadata. */
    vault: {
        /** Inspect one vault resource. */
        get: procedure("vault", "get")
            .route({ method: "POST", path: "/vault/get" })
            .input(VaultScope.extend({ vaultId: identifier("resource") }))
            .output(
                schema.object({ spaceId: identifier("space"), resourceId: identifier("resource") }),
            ),
        /** List vault resources in a space. */
        list: procedure("vault", "list")
            .route({ method: "POST", path: "/vault/list" })
            .input(VaultScope.extend(pagination.shape))
            .output(
                page(
                    schema.object({
                        spaceId: identifier("space"),
                        resourceId: identifier("resource"),
                    }),
                ),
            ),
    },
    /** Secret metadata and availability. */
    secret: {
        /** Destroy due secrets in a bounded batch within one space. */
        purge: procedure("secret", "purge")
            .route({ method: "POST", path: "/secret/purge" })
            .input(VaultScope.extend({ limit: schema.number().int().min(1).max(1000) }))
            .output(schema.number().int().nonnegative()),
        /** Create metadata without assigning a value. */
        create: procedure("secret", "create")
            .route({ method: "POST", path: "/secret/create" })
            .input(
                VaultScope.extend(Creation.shape).extend({
                    vaultId: identifier("resource"),
                    name: Secret.shape.name,
                    tags: Secret.shape.tags.optional(),
                }),
            )
            .output(Secret),
        /** Read metadata without decrypting a value. */
        get: procedure("secret", "get")
            .route({ method: "POST", path: "/secret/get" })
            .input(SecretKey)
            .output(Secret),
        /** List metadata with a scope-bound continuation. */
        list: procedure("secret", "list")
            .route({ method: "POST", path: "/secret/list" })
            .input(
                VaultScope.extend({
                    vaultId: identifier("resource"),
                    ...pagination.shape,
                }),
            )
            .output(page(Secret)),
        /** Change the name and labels at the observed revision. */
        update: procedure("secret", "update")
            .route({ method: "POST", path: "/secret/update" })
            .input(mutation.extend({ name: Secret.shape.name, tags: Secret.shape.tags }))
            .output(Secret),
        /** Block subsequent value reads. */
        disable: procedure("secret", "disable")
            .route({ method: "POST", path: "/secret/disable" })
            .input(mutation)
            .output(Secret),
        /** Remove explicit disabling without undoing deletion. */
        enable: procedure("secret", "enable")
            .route({ method: "POST", path: "/secret/enable" })
            .input(mutation)
            .output(Secret),
        /** Schedule destruction after the configured recovery interval. */
        delete: procedure("secret", "delete")
            .route({ method: "POST", path: "/secret/delete" })
            .input(mutation)
            .output(Secret),
        /** Cancel deletion before its recovery deadline. */
        restore: procedure("secret", "restore")
            .route({ method: "POST", path: "/secret/restore" })
            .input(mutation)
            .output(Secret),
    },
    /** Immutable values and current-version selection. */
    version: {
        /** Rewrap one version with the host's active root key. */
        rewrap: procedure("version", "rewrap")
            .route({ method: "POST", path: "/version/rewrap" })
            .input(mutation.extend({ version: VersionKey.shape.version }))
            .output(SecretVersion),
        /** Read exact version metadata. */
        get: procedure("version", "get")
            .route({ method: "POST", path: "/version/get" })
            .input(VersionKey)
            .output(SecretVersion),
        /** List version metadata in ascending order. */
        list: procedure("version", "list")
            .route({ method: "POST", path: "/version/list" })
            .input(SecretKey.extend(pagination.shape))
            .output(page(SecretVersion)),
        /** Authorize and audit release of plaintext. */
        read: procedure("version", "read")
            .route({ method: "POST", path: "/version/read" })
            .input(SecretKey.extend({ version: VersionKey.shape.version.optional() }))
            .output(schema.object({ version: SecretVersion, value: SecretValue })),
        /** Append a value, selecting it as current unless promotion is disabled. */
        write: procedure("version", "write")
            .route({ method: "POST", path: "/version/write" })
            .input(
                mutation.extend({
                    value: SecretValue,
                    expiresAt: schema.number().int().optional(),
                    promote: schema.boolean().optional(),
                }),
            )
            .output(schema.object({ secret: Secret, version: SecretVersion })),
        /** Select an existing usable version as current. */
        promote: procedure("version", "promote")
            .route({ method: "POST", path: "/version/promote" })
            .input(mutation.extend({ version: VersionKey.shape.version }))
            .output(Secret),
        /** Block reads of one exact version. */
        disable: procedure("version", "disable")
            .route({ method: "POST", path: "/version/disable" })
            .input(mutation.extend({ version: VersionKey.shape.version }))
            .output(SecretVersion),
        /** Remove explicit disabling while retaining expiry and destruction. */
        enable: procedure("version", "enable")
            .route({ method: "POST", path: "/version/enable" })
            .input(mutation.extend({ version: VersionKey.shape.version }))
            .output(SecretVersion),
        /** Irreversibly remove ciphertext while retaining version metadata. */
        destroy: procedure("version", "destroy")
            .route({ method: "POST", path: "/version/destroy" })
            .input(mutation.extend({ version: VersionKey.shape.version }))
            .output(SecretVersion),
    },
    /** Protected mutation retry records. */
    request: {
        /** Rewrap a bounded batch before retiring a root key. */
        rewrap: procedure("request", "rewrap")
            .route({ method: "POST", path: "/request/rewrap" })
            .input(
                VaultScope.extend({
                    keyId: schema.string().min(1),
                    limit: schema.number().int().min(1).max(1000),
                }),
            )
            .output(schema.number().int().nonnegative()),
    },
};

/** Declare separately grantable, audited vault operations. */
function procedure(type: string, name: string) {
    return defineProcedure({
        authentication: "identity",
        permission: { packageId: PackageId.parse(definition.id), type, name },
        audit: true,
    });
}
