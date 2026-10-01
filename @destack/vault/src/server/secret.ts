import type { InstanceOf } from "@destack/object";
import { and, eq, isNotNull, isNull } from "@destack/db";
import { Call, recoverable, type Duration } from "@destack/object";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Binding, capture, Deployment, deployment, resource } from "@destack/space/object";
import type { Bindable } from "@destack/space/server";
import * as base from "../object/index.ts";
import { SecretVersion, secretVersion, vault } from "../object/index.ts";
import { SecretPromotion, SecretSelection, VersionWrite } from "../secret/index.ts";
import { type Keyring, VaultKey } from "../encryption/index.ts";
import { VaultValue } from "./value.ts";

/** Secrets a stack declares, deleted to the trash once no longer declared. */
export const secret = base.secret.declare({
    after: [resource],
    resolve: async (_name, declared, stack) => {
        // wait for the vault resource to be provisioned before writing into it
        const vaultId = identifier("resource").parse(await stack.require(resource, declared.vault));
        const [provisioned] = await stack.database
            .select({ id: vault.table.id })
            .from(vault.table)
            .where(eq(vault.table.id, vaultId));
        if (!provisioned) {
            stack.wait(`vault ${declared.vault} is not provisioned`);
        }

        return { parentId: vaultId, name: declared.name };
    },
    values: (_name, resolved) => resolved,
});

/** Secrets that deployments capture at their current version or at a binding's pinned one. */
export const secretBindable: Bindable = {
    object: secret,
    readable: [secret, secretVersion],
    version: (target, pin) => {
        // require a version to run with: the pinned one, else the current one
        const version = pin ?? (target as InstanceOf<typeof secret>).currentVersion;
        if (version === null) {
            throw new ServiceError("CONFLICT", {
                message: `secret ${String(target.id)} has no current version`,
            });
        }

        return version;
    },
    reads: async (captured, database) => {
        // read the captured version, refusing a capture whose version is gone
        const secretId = identifier("secret").parse(captured.target);
        const [version] = await database
            .select({ id: secretVersion.table.id })
            .from(secretVersion.table)
            .where(
                and(
                    eq(secretVersion.table.parentId, secretId),
                    eq(secretVersion.table.number, captured.version),
                ),
            );
        if (version === undefined) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `secret ${secretId} has no version ${captured.version} its capture keeps`,
            });
        }

        return [
            secret.reference(captured.scope, secretId),
            secretVersion.reference(captured.scope, version.id),
        ];
    },
};

/** Serve vaults, secrets and versions. */
export function servedObjects(keyring: Keyring, location: string, recovery: Duration) {
    // select a readable version as its secret's current one
    const select = async (call: Call<typeof secret.table>) => {
        // require a readable version
        const { version } = SecretPromotion.strip().parse(call.input);
        const selected = await SecretVersion.find(call.database, call.target!.id, version);
        if (selected === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "secret version not found" });
        }
        SecretVersion.requireReadable(selected, call.now);

        return call.revise({ currentVersion: version });
    };

    // erase a kept version's ciphertext and keep its record, refusing one a live deployment captured
    const destroy = async (call: Call<typeof secretVersion.table>) => {
        // require a kept version no live deployment runs with
        const target = call.target!;
        SecretVersion.requireKept(target);
        const [captured] = await call.database
            .select({ deploymentId: capture.table.deploymentId })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .where(
                and(
                    eq(capture.table.target, target.parentId),
                    eq(capture.table.version, target.number),
                    Deployment.live(),
                ),
            )
            .limit(1);
        if (captured !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `secret version ${target.number} is captured by ${captured.deploymentId}`,
            });
        }

        // erase its ciphertext
        await VaultValue.destroy(call.database, target.parentId, target.number);

        return call.revise({ destroyedAt: call.now });
    };

    // delete a vault with the records of its purged secrets
    const vaults = vault.handle({
        delete: async (call, next) => {
            // discard the records of its purged secrets
            const purged = await call.database
                .select({ id: secret.table.id })
                .from(secret.table)
                .where(
                    and(
                        eq(secret.table.parentId, call.target!.id),
                        isNotNull(secret.table.purgedAt),
                    ),
                );
            for (const { id } of purged) {
                await call.invoke(secret, "discard", { id });
            }

            return next();
        },
    });

    // serve the secret methods, destroying the values of a purged secret
    const secrets = secret.handle({
        disable: (call) => call.revise({ disabledAt: call.now }),
        enable: (call) => call.revise({ disabledAt: null }),
        promote: (call) => select(call),
        select: (call) => select(call),
        read: async (call) => {
            // require an enabled secret with the selected version, its current one by default
            const target = call.target!;
            const { version } = SecretSelection.strip().parse(call.input);
            const number = version ?? target.currentVersion;
            if (target.disabledAt !== null) {
                throw new ServiceError("FORBIDDEN", { message: "secret is unavailable" });
            } else if (number === null) {
                throw new ServiceError("NOT_FOUND", { message: "secret has no current version" });
            }

            // require reading the secret, or the exact version, answering a missing version only to readers of the secret
            const isSecretReader = (
                await call.authorization!.check(call.object.permission("read"), call.reference())
            ).isAllowed;
            const selected = await SecretVersion.find(call.database, target.id, number);
            const isVersionReader =
                version !== undefined &&
                selected !== undefined &&
                (
                    await call.authorization!.check(
                        secretVersion.permission("read"),
                        secretVersion.reference(call.scope, selected.id),
                    )
                ).isAllowed;
            if (!isSecretReader && !isVersionReader) {
                throw new ServiceError("FORBIDDEN", { message: "permission denied: read" });
            } else if (selected === undefined) {
                throw new ServiceError("NOT_FOUND", { message: "secret version not found" });
            }
            SecretVersion.requireReadable(selected, call.now);

            return {
                version: selected.number,
                value: await VaultValue.read(
                    call.database,
                    await VaultKey.load(call.database, keyring, location, target.parentId),
                    target,
                    selected.number,
                ),
            };
        },
        delete: async (call, next) => {
            // refuse deleting a secret a binding targets
            await Binding.requireUnbound(call.database, call.target!);

            return next();
        },
        discard: async (call, next) => {
            // discard the versions first
            const target = await call.revise({ currentVersion: null });
            const versions = await call.database
                .select({ id: secretVersion.table.id })
                .from(secretVersion.table)
                .where(eq(secretVersion.table.parentId, call.target!.id));
            for (const { id } of versions) {
                await call.invoke(secretVersion, "delete", { id });
            }

            return next(call.with({ target: target as never }));
        },
        purge: async (call, next) => {
            // purge only an unbound secret, at any time while it is in the trash
            const target = call.target!;
            await Binding.requireUnbound(call.database, target);

            // destroy the value of every kept version
            const kept = await call.database
                .select({ id: secretVersion.table.id })
                .from(secretVersion.table)
                .where(
                    and(
                        eq(secretVersion.table.parentId, target.id),
                        isNull(secretVersion.table.destroyedAt),
                    ),
                );
            for (const { id } of kept) {
                await call.invoke(secretVersion, "purge", { id });
            }

            return next();
        },
    });

    // write, read and manage versions with their sealed values
    const versions = secretVersion.handle({
        create: async (call, next) => {
            // require a secret outside the trash and an expiry in the future
            const { value, promote } = VersionWrite.strip().parse(call.input);
            const [owner] = await call.database
                .select()
                .from(secret.table)
                .where(eq(secret.table.id, identifier("secret").parse(call.input.parentId)));
            if (owner === undefined) {
                throw new ServiceError("NOT_FOUND", { message: "secret not found" });
            }
            const expiresAt = call.input.expiresAt;
            if (owner.deletionRequestedAt !== null) {
                throw new ServiceError("CONFLICT", { message: "secret is in the trash" });
            } else if (typeof expiresAt === "number" && expiresAt <= call.now) {
                throw new ServiceError("BAD_REQUEST", {
                    message: "secret expiry must be in the future",
                });
            }

            // number the version, store its value, and select it as current unless told not to
            const created = (await next()) as SecretVersion;
            await VaultValue.write(
                call.database,
                await VaultKey.load(call.database, keyring, location, owner.parentId),
                owner,
                created.number,
                value,
            );
            if (promote !== false) {
                await call.invoke(secret, "select", { id: owner.id, version: created.number });
            }

            return created;
        },
        disable: async (call) => {
            SecretVersion.requireKept(call.target!);

            return call.revise({ disabledAt: call.now });
        },
        enable: async (call) => {
            SecretVersion.requireKept(call.target!);

            return call.revise({ disabledAt: null });
        },
        destroy: (call) => destroy(call),
        purge: (call) => destroy(call),
    });

    return { vault: vaults, secret: recoverable.within(secrets, recovery), version: versions };
}
