import { and, eq, isNotNull, isNull } from "@destack/db";
import { type CallOf, type NextOf, recoverable, type ResultOf } from "@destack/object";
import { Duration, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Binding, capture, Deployment, deployment } from "@destack/space/object";
import type { Bindable } from "@destack/space/server";
import * as base from "../object/index.ts";
import { SecretVersion, secretVersion } from "../object/index.ts";
import { SecretPromotion, SecretSelection, SecretValue, VersionWrite } from "../secret/index.ts";
import { SECRET_KIND } from "@destack/resource";
import { SecretDescription } from "../declare/secret.ts";
import type { Keyring } from "@destack/host/keychain";
import { VaultKey } from "../encryption/index.ts";

/** How long deleted secrets stay restorable by default: the 30 days of AWS Secrets Manager's recovery window. */
const RECOVERY: Duration = { days: 30 };

/** The shortest recovery window a host may give deleted secrets: one day. */
const MINIMUM_RECOVERY: Duration = { days: 1 };

/** Vaults as their space serves them, finalized with the records of their purged secrets. */
export const vault = base.vault.handle({
    finalize: async (call, next) => {
        // discard the records of its purged secrets
        const purged = await call.database
            .select({ id: base.secret.table.id })
            .from(base.secret.table)
            .where(
                and(
                    eq(base.secret.table.parentId, call.target.id),
                    isNotNull(base.secret.table.purgedAt),
                ),
            );
        for (const { id } of purged) {
            await call.invoke(secret).discard({ id });
        }

        return next();
    },
});

/** Secrets a stack declares, deleted to the trash once no longer declared. */
export const secret = base.secret.declare({
    after: [vault],
    resolve: async (_name, declared, stack) => {
        // wait for the vault to be provisioned before writing into it
        const vaultId = schema
            .identifier("vault")
            .parse(await stack.require(vault, declared.vault));
        const [provisioned] = await stack.database
            .select({ id: vault.table.id })
            .from(vault.table)
            .where(and(eq(vault.table.id, vaultId), isNotNull(vault.table.reference)));
        if (!provisioned) {
            stack.wait(`vault ${declared.vault} is not provisioned`);
        }

        return { parentId: vaultId, name: declared.name };
    },
    values: (_name, resolved) => resolved,
});

/** Secrets that deployments capture at their current version or at a binding's fixed one, generated at install where their declaration says how. */
export const secretBindable: Bindable<typeof secret> = {
    object: secret,
    used: [secret, secretVersion],
    declaration: SECRET_KIND,
    own: async (call, _target, need, bindings) => {
        // leave a secret without generation to the stack binding it
        const { generated } = SecretDescription.parse({ ...need.spec, name: need.name });
        if (generated === undefined) {
            return undefined;
        }

        // require the installation's binding of the package's vault keeping it
        const kept = bindings.find(
            (entry) => entry.packageId === need.package.id && entry.name === generated.vault,
        );
        if (kept === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `secret ${need.name} is kept in vault ${generated.vault}, which the installation binds nowhere`,
            });
        }

        // create the secret in the vault, its first version waiting for the vault's key
        const created = await call.invoke(secret).own({ parentId: kept.target, name: need.name });

        return created.id;
    },
    ready: async (call, bound, need) => {
        // pass a secret with a version and one a stack binds
        const [target] = await call.database
            .select({
                parentId: secret.table.parentId,
                currentVersion: secret.table.currentVersion,
            })
            .from(secret.table)
            .where(eq(secret.table.id, schema.identifier("secret").parse(bound.target)));
        const { generated } = SecretDescription.parse({ ...need.spec, name: need.name });
        if (target === undefined || target.currentVersion !== null || generated === undefined) {
            return true;
        }

        // wait for the vault's key
        const [kept] = await call.database
            .select({ reference: vault.table.reference })
            .from(vault.table)
            .where(eq(vault.table.id, target.parentId));
        if (kept?.reference === null || kept === undefined) {
            return false;
        }

        // write the generated first version
        const value = await SecretValue.generate(generated.algorithm);
        await call.invoke(secretVersion).store({ parentId: bound.target, value });

        return true;
    },
    version: (target, pin) => {
        // require a version to run with: the fixed one, else the current one
        const version = pin ?? target.currentVersion;
        if (version === null) {
            throw new ServiceError("CONFLICT", {
                message: `secret ${target.id} has no current version`,
            });
        }

        return version;
    },
    uses: async (captured, database) => {
        // read the captured version, refusing a capture whose version is gone
        const secretId = schema.identifier("secret").parse(captured.target);
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

/** Serve secrets and versions, restorable for a recovery window after deletion. */
export function serveSecrets(keyring: Keyring, location: string, recovery: Duration = RECOVERY) {
    // require a recovery window of a day at least
    if (Duration.milliseconds(recovery) < Duration.milliseconds(MINIMUM_RECOVERY)) {
        throw new TypeError("vault recovery window is shorter than a day");
    }

    // serve the secret methods, destroying the values of a purged secret
    const secrets = secret.handle({
        disable: (call) => call.update({ disabledAt: call.now }),
        enable: (call) => call.update({ disabledAt: null }),
        promote: (call) => select(call),
        select,
        read: (call) => read(call, keyring, location),
        delete: remove,
        discard,
        purge,
    });

    // write, read and manage versions with their sealed values
    const versions = secretVersion.handle({
        create: (call, next) => create(call, next, keyring, location),
        store: (call, next) => create(call, next, keyring, location),
        disable,
        enable,
        destroy,
        purge: (call) => destroy(call),
    });

    return { secret: recoverable.within(secrets, recovery), version: versions };
}

/** Open an enabled secret's selected version, its current one by default, for readers of the secret or of the exact version. */
async function read(
    call: CallOf<typeof secret, "read">,
    keyring: Keyring,
    location: string,
): Promise<ResultOf<typeof secret, "read">> {
    // require an enabled secret with the selected version, its current one by default
    const { target } = call;
    const { version } = SecretSelection.strip().parse(call.input);
    const number = version ?? target.currentVersion;
    if (target.disabledAt !== null) {
        throw new ServiceError("FORBIDDEN", { message: "secret is unavailable" });
    } else if (number === null) {
        throw new ServiceError("NOT_FOUND", { message: "secret has no current version" });
    }

    // require reading the secret or the exact version
    const authorization = call.requireAuthorization();
    const isSecretReader = (
        await authorization.check(call.object.permission("read"), call.reference())
    ).isAllowed;
    const selected = await SecretVersion.find(call.database, target.id, number);
    const isVersionReader =
        version !== undefined &&
        selected !== undefined &&
        (
            await authorization.check(
                secretVersion.permission("read"),
                secretVersion.reference(call.scope, selected.id),
            )
        ).isAllowed;

    // refuse a caller reading neither
    if (!isSecretReader && !isVersionReader) {
        throw new ServiceError("FORBIDDEN", { message: "permission denied: read" });
    }
    // answer a missing version only to readers of the secret
    else if (selected === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "secret version not found" });
    }
    SecretVersion.requireReadable(selected, call.now);

    return {
        version: selected.number,
        value: await SecretVersion.open(
            await VaultKey.load(call.database, keyring, location, target.parentId),
            target,
            selected,
        ),
    };
}

/** Delete a secret no binding targets. */
async function remove(
    call: CallOf<typeof secret, "delete">,
    next: NextOf<typeof secret, "delete">,
): Promise<ResultOf<typeof secret, "delete">> {
    await Binding.requireUnbound(call.database, call.target);

    return next();
}

/** Discard a secret after its versions. */
async function discard(
    call: CallOf<typeof secret, "discard">,
    next: NextOf<typeof secret, "discard">,
): Promise<ResultOf<typeof secret, "discard">> {
    // discard the versions first
    const target = await call.update({ currentVersion: null });
    const versions = await call.database
        .select({ id: secretVersion.table.id })
        .from(secretVersion.table)
        .where(eq(secretVersion.table.parentId, call.target.id));
    for (const { id } of versions) {
        await call.invoke(secretVersion).delete({ id });
    }

    return next(call.with({ target }));
}

/** Purge an unbound secret at any time while it is in the trash, destroying the value of every kept version. */
async function purge(
    call: CallOf<typeof secret, "purge">,
    next: NextOf<typeof secret, "purge">,
): Promise<ResultOf<typeof secret, "purge">> {
    // purge only an unbound secret
    const { target } = call;
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
        await call.invoke(secretVersion).purge({ id });
    }

    return next();
}

/** Number and seal a version of a secret outside the trash, and select it as current unless told not to. */
async function create(
    call: CallOf<typeof secretVersion, "create" | "store">,
    next: NextOf<typeof secretVersion, "create" | "store">,
    keyring: Keyring,
    location: string,
): Promise<ResultOf<typeof secretVersion, "create" | "store">> {
    // require a secret outside the trash and an expiry in the future
    const { value, promote } = VersionWrite.strip().parse(call.input);
    const [owner] = await call.database
        .select()
        .from(secret.table)
        .where(eq(secret.table.id, schema.identifier("secret").parse(call.input["parentId"])));
    if (owner === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "secret not found" });
    }
    const expiresAt = call.input["expiresAt"];
    if (owner.deletionRequestedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "secret is in the trash" });
    } else if (typeof expiresAt === "number" && expiresAt <= call.now) {
        throw new ServiceError("BAD_REQUEST", { message: "secret expiry must be in the future" });
    }

    // number the version and store its sealed value
    const created = await next();
    await SecretVersion.seal(
        call.database,
        await VaultKey.load(call.database, keyring, location, owner.parentId),
        owner,
        created.number,
        value,
    );

    // select it as current unless told not to
    if (promote !== false) {
        await call.invoke(secret).select({ id: owner.id, version: created.number });
    }

    return created;
}

/** Disable a kept version. */
async function disable(
    call: CallOf<typeof secretVersion, "disable">,
): Promise<ResultOf<typeof secretVersion, "disable">> {
    SecretVersion.requireKept(call.target);

    return call.update({ disabledAt: call.now });
}

/** Enable a kept version. */
async function enable(
    call: CallOf<typeof secretVersion, "enable">,
): Promise<ResultOf<typeof secretVersion, "enable">> {
    SecretVersion.requireKept(call.target);

    return call.update({ disabledAt: null });
}

/** Select a readable version as its secret's current one. */
async function select(
    call: CallOf<typeof secret, "select">,
): Promise<ResultOf<typeof secret, "select">> {
    // require a readable version
    const { version } = SecretPromotion.strip().parse(call.input);
    const selected = await SecretVersion.find(call.database, call.target.id, version);
    if (selected === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "secret version not found" });
    }
    SecretVersion.requireReadable(selected, call.now);

    return call.update({ currentVersion: version });
}

/** Erase a kept version's ciphertext and keep its record, refusing one a live deployment captured. */
async function destroy(
    call: CallOf<typeof secretVersion, "destroy">,
): Promise<ResultOf<typeof secretVersion, "destroy">> {
    // require a kept version no live deployment runs with
    const { target } = call;
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

    // erase its ciphertext with the version
    return call.update({ destroyedAt: call.now, envelope: null });
}
