import { Caller, type Creation, Manager, principal } from "@destack/access";
import { type RowImage, Transaction } from "@destack/db";
import { type Call, type CallOf, type NextOf, recoverable, type ResultOf } from "@destack/object";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Binding, installation } from "@destack/stack/object";
import type { ResourceNeed, ServedBindable } from "@destack/stack/server";
import { SECRET_KIND, VAULT_PROVIDER } from "../declare/kind.ts";
import { SecretDescription } from "../declare/secret.ts";
import type { VaultKeystore } from "../key/index.ts";
import {
    SECRET_RECOVERY,
    secret,
    SecretVersion,
    secretVersion,
    vault,
    vaultKey,
} from "../object/index.ts";
import {
    type SecretAlgorithm,
    type SecretGeneration,
    SecretReading,
    type SecretValue,
} from "../secret/secret.ts";
import { vaultService } from "../service/index.ts";

/** The shortest recovery window a machine may give deleted secrets: one day. */
const MINIMUM_RECOVERY: Duration = { days: 1 };

/** The algorithm of generated ES256 keys: ECDSA over P-256 (RFC 7518 3.4). */
const ES256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** Serve a space's vaults, reading their secrets by name under a machine's keystore. */
export function serveVaults(keystore: VaultKeystore) {
    return vault.implement({
        methods: {
            create: (call, next) => startKey(call, next),
            apply: (call, next) => startKey(call, next),
            read: (call) => readNamed(call, keystore),
            put: putNamed,
            remove: removeNamed,
            finalize: async (call, next) => {
                // discard the records of its purged secrets in one request
                const purged = await call
                    .read(secret)
                    .rows({ parentId: call.target.id, purgedAt: { isNotNull: true } });
                if (purged.length > 0) {
                    await call.mutate(
                        call.caller,
                        purged.map(({ id }) => secret.call("discard", { id })),
                    );
                }

                return next();
            },
        },
    });
}

/** Put a value as the one live version of a vault's secret, created on its first put. */
async function putNamed(call: CallOf<typeof vault, "put">): Promise<ResultOf<typeof vault, "put">> {
    // find the secret, or create it bound to the consumer
    const { target, input } = call;
    const { consumer } = input;
    const id = input.secret;
    const [found] = await call.read(secret).rows({
        parentId: target.id,
        id,
        purgedAt: { isNull: true },
    });
    const named = secret.target({ scope: target.scope, id });
    const creation =
        found === undefined
            ? [
                  secret.call("create", { ...named, parentId: target.id, name: id }),
                  secret.call("bindConsumer", { ...named, consumer }),
              ]
            : [];

    // retire the intact versions before
    const before =
        found === undefined
            ? []
            : await call.read(secretVersion).rows({ parentId: id, destroyedAt: { isNull: true } });
    const retiring = before.flatMap((earlier) => [
        secretVersion.call("releaseConsumer", { ...secretVersion.target(earlier), consumer }),
        secretVersion.call("destroy", secretVersion.target(earlier)),
    ]);

    // store the value as the current version, bound to the consumer, in one request
    const version = secretVersion.target({ scope: target.scope, id: secretVersion.generateId() });
    await call.mutate(call.caller, [
        ...creation,
        secretVersion.call("create", { ...version, parentId: id, value: input.value }),
        secretVersion.call("bindConsumer", { ...version, consumer }),
        ...retiring,
    ]);

    return target;
}

/** Remove a vault's secret with its versions, releasing them from the consumer. */
async function removeNamed(
    call: CallOf<typeof vault, "remove">,
): Promise<ResultOf<typeof vault, "remove">> {
    // leave a secret the vault keeps no longer
    const { target, input } = call;
    const { consumer } = input;
    const [found] = await call.read(secret).rows({
        parentId: target.id,
        id: input.secret,
        purgedAt: { isNull: true },
    });
    if (found === undefined) {
        return target;
    }

    // release it and its versions from the consumer, then trash and purge it
    const versions = await call.read(secretVersion).rows({ parentId: found.id });
    const named = secret.target(found);
    await call.mutate(call.caller, [
        secret.call("releaseConsumer", { ...named, consumer }),
        ...versions.map((version) =>
            secretVersion.call("releaseConsumer", { ...secretVersion.target(version), consumer }),
        ),
        secret.call("delete", named),
        secret.call("purge", named),
    ]);

    return target;
}

/** Start a vault's key at its first version in the write creating or declaring the vault. */
async function startKey<
    Created extends { readonly scope: string; readonly id: Identifier<"vault"> },
>(call: Call, next: () => Promise<Created>): Promise<Created> {
    // write the vault and read whether it has a key version
    const created = await next();
    const started = await call.read(vaultKey).rows({ parentId: created.id });

    // generate its first one otherwise
    if (started.length === 0) {
        await call.mutate(call.caller, [
            vaultKey.call("rotate", { ...vaultKey.in(created.scope), parentId: created.id }),
        ]);
    }

    return created;
}

/** Serve the vaults' key versions, each generated under the machine's keyring as it is created. */
export function serveKeys(keystore: VaultKeystore) {
    return vaultKey.implement({
        methods: {
            rotate: async (call, next) => {
                // generate the version's material under its identifier
                const id = call.id ?? vaultKey.generateId();
                const ciphertext = await keystore.generate(call.input.parentId, id);

                return next(call.with({ id, input: { ...call.input, ciphertext } }));
            },
        },
    });
}

/** The secrets a stack declares, deleted to the trash once no longer declared. */
export const servedSecret = secret.implement({
    declaration: {
        after: [vault],
        resolve: async (_name, declared, stack) => {
            // wait for the vault to be provisioned before writing into it
            const vaultId = schema
                .identifier("vault")
                .parse(await stack.require(vault, declared.vault));
            const [provisioned] = await stack.transaction.rows(vault.table, {
                id: vaultId,
                reference: { isNotNull: true },
            });
            if (!provisioned) {
                stack.wait(`vault ${declared.vault} is not provisioned`);
            }

            return { parentId: vaultId, name: declared.name };
        },
        values: (_name, resolved) => resolved,
    },
});

/** Secrets that deployments capture at their current version or at a binding's fixed one. */
export const secretBindable: ServedBindable<typeof servedSecret> = {
    object: servedSecret,
    used: [servedSecret, secretVersion],
    declaration: SECRET_KIND,
    provider: VAULT_PROVIDER,
    reference: vaultService.package.name,
    provision: async (call, _target, need, bindings) => {
        // leave a secret without generation to the stack binding it
        const generated = generation(need);
        if (generated === undefined) {
            return undefined;
        }

        // require the installation's binding of the package's vault keeping it
        const holding = bindings.find(
            (entry) => entry.packageId === need.package.id && entry.name === generated.vault,
        );
        if (holding === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `secret ${need.name} lives in vault ${generated.vault}, which the installation binds nowhere`,
            });
        }

        // create the secret under the identifier its binding takes
        const id = secret.generateId();
        await call.mutate(call.service, [
            secret.call("provision", { id, parentId: holding.target, name: need.name }),
        ]);

        return id;
    },
    ready: async (call, bound, need) => {
        // read the bound secret and how its declaration generates it
        const id = schema.identifier("secret").parse(bound.target);
        const target = await call.view().row(secret.table, { id });
        const generated = generation(need);

        // refuse a binding whose secret is gone
        if (target === null) {
            throw new ServiceError("NOT_FOUND", {
                message: `secret ${id} of binding ${bound.name} is gone`,
            });
        }
        // pass a secret with a version, and one a stack binds
        else if (target.currentVersion !== null || generated === undefined) {
            return true;
        }

        // wait for the vault to be provisioned
        const owner = await call.view().row(vault.table, { id: target.parentId });
        if (present(owner, `vault ${target.parentId}`).reference === null) {
            return false;
        }

        // have the vault generate the first version and wait for its copy
        await call.mutate(call.service, [
            secret.call("generate", { id, algorithm: generated.algorithm }),
        ]);

        return false;
    },
    version: (target, pin) => pin ?? target.currentVersion,
    uses: async (captured, snapshot) => {
        // use the secret alone before its first version
        const secretId = schema.identifier("secret").parse(captured.target);
        if (captured.version === null) {
            return [secret.reference(captured.scope, secretId)];
        }

        // require the captured version
        const [version] = await snapshot.rows(secretVersion.table, {
            parentId: secretId,
            number: captured.version,
        });
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

/** Serve the secrets and versions of a machine's vaults. */
export function serveSecrets(keystore: VaultKeystore, recovery: Duration = SECRET_RECOVERY) {
    // require a recovery window of a day at least
    if (Duration.milliseconds(recovery) < Duration.milliseconds(MINIMUM_RECOVERY)) {
        throw new TypeError("vault recovery window is shorter than a day");
    }

    // serve the secret methods
    const secrets = servedSecret.implement({
        methods: {
            create: { creation: custody },
            disable: (call) => call.update({ disabledAt: call.now }),
            enable: (call) => call.update({ disabledAt: null }),
            promote,
            select: (call) => call.update({ currentVersion: call.input.version }),
            generate,
            read: (call) => open(call, call.target, call.input.version, keystore),
            delete: remove,
            discard,
            purge,
            apply: applyDeclared,
        },
    });

    // serve the version methods
    const versions = secretVersion.implement({
        methods: {
            create: (call, next) => create(call, next, keystore),
            disable,
            enable,
            destroy,
            erase: (call) => call.update(erasure(call.now)),
        },
    });

    return [recoverable.within(secrets, recovery), versions] as const;
}

/** Read a version of a vault's secret of a name, refusing a caller who opens no such secret. */
async function readNamed(
    call: CallOf<typeof vault, "read">,
    keystore: VaultKeystore,
): Promise<ResultOf<typeof vault, "read">> {
    // find the secret of the name in the vault, outside the trash
    const { name, version } = call.input;
    const [named] = await call.read(secret).rows({
        parentId: call.target.id,
        name,
        deletionRequestedAt: { isNull: true },
    });
    if (named === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `vault has no secret ${name}` });
    }

    // require opening it
    const reference = secret.reference(call.scope, named.id);
    const opening = await call.requireAuthorization().check(secret.permission("open"), reference);
    if (!opening.isAllowed) {
        throw new ServiceError("FORBIDDEN", { message: "permission denied: open" });
    }

    return open(call, named, version, keystore);
}

/** Open an enabled secret's selected version, its current one by default. */
async function open(
    call: Call,
    target: RowImage<typeof secret.table>,
    version: number | undefined,
    keystore: VaultKeystore,
): Promise<SecretReading> {
    // require an enabled secret with a selected version
    const number = version ?? target.currentVersion;
    if (target.disabledAt !== null) {
        throw new ServiceError("FORBIDDEN", { message: "secret is unavailable" });
    } else if (number === null) {
        throw new ServiceError("NOT_FOUND", { message: "secret has no current version" });
    }

    // require reading the secret or the exact version
    const authorization = call.requireAuthorization();
    const isSecretReader = (
        await authorization.check(
            secret.permission("read"),
            secret.reference(call.scope, target.id),
        )
    ).isAllowed;
    const selected = await SecretVersion.find(call.requireTransaction(), target.id, number);
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

    // open the value under the key version that sealed it
    const { keyId, ciphertext } = SecretVersion.requireReadable(selected, call.now);
    const key = await keystore.open(call.requireTransaction(), keyId);
    const value = await SecretVersion.open(key, target, selected.id, ciphertext);

    return { version: selected.number, value };
}

/** Write a generated first version of a secret without one, leaving a secret that has one. */
async function generate(
    call: CallOf<typeof secret, "generate">,
): Promise<ResultOf<typeof secret, "generate">> {
    // leave a secret that has a version
    const { target } = call;
    if (target.currentVersion !== null) {
        return target;
    }

    // store the generated value as its first version, current
    const value = await generateValue(call.input.algorithm);
    await call.mutate(call.caller, [secretVersion.call("create", { parentId: target.id, value })]);

    return readSecret(call, target.id);
}

/** Apply a secret a stack declares. */
async function applyDeclared(
    call: CallOf<typeof secret, "apply">,
    next: NextOf<typeof secret, "apply">,
): Promise<ResultOf<typeof secret, "apply">> {
    // require the managing installation in the secret's space
    const manager = Manager.schema.parse(call.input["manager"]);
    const [managing] = await call.read(installation).rows({
        scope: schema.identifier("space").parse(call.scope),
        id: manager.installationId,
    });
    if (managing === undefined) {
        throw new ServiceError("NOT_FOUND", {
            message: `no installation ${manager.installationId} manages secrets in ${call.scope}`,
        });
    }

    return next();
}

/** Delete a secret no binding targets. */
async function remove(
    call: CallOf<typeof secret, "delete">,
    next: NextOf<typeof secret, "delete">,
): Promise<ResultOf<typeof secret, "delete">> {
    await Binding.requireUnbound(call.requireTransaction(), call.target);

    return next();
}

/** Discard a secret after its versions. */
async function discard(
    call: CallOf<typeof secret, "discard">,
    next: NextOf<typeof secret, "discard">,
): Promise<ResultOf<typeof secret, "discard">> {
    // discard the versions first, in one request
    const target = await call.update({ currentVersion: null });
    const versions = await call.read(secretVersion).rows({ parentId: call.target.id });
    if (versions.length > 0) {
        await call.mutate(
            call.caller,
            versions.map(({ id }) => secretVersion.call("delete", { id })),
        );
    }

    return next(call.with({ target }));
}

/** Purge an unbound secret at any time while it is in the trash. */
async function purge(
    call: CallOf<typeof secret, "purge">,
    next: NextOf<typeof secret, "purge">,
): Promise<ResultOf<typeof secret, "purge">> {
    // purge only an unbound secret
    const { target } = call;
    await Binding.requireUnbound(call.requireTransaction(), target);

    // destroy the value of every intact version no live deployment runs with, in one request
    const intact = await call.requireTransaction().rows(secretVersion.table, {
        parentId: target.id,
        destroyedAt: { isNull: true },
    });
    for (const version of intact) {
        await requireReleased(call.requireTransaction(), version);
    }
    if (intact.length > 0) {
        await call.mutate(
            call.caller,
            intact.map((version) => secretVersion.call("erase", secretVersion.target(version))),
        );
    }

    return next();
}

/** Seal a version of a secret outside the trash under its vault's newest key. */
async function create(
    call: CallOf<typeof secretVersion, "create">,
    next: NextOf<typeof secretVersion, "create">,
    keystore: VaultKeystore,
): Promise<ResultOf<typeof secretVersion, "create">> {
    // require a secret outside the trash and an expiry in the future
    const { value, stage, expiresAt, parentId } = call.input;
    const owner = await readSecret(call, parentId);
    if (owner.deletionRequestedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "secret is in the trash" });
    } else if (expiresAt !== undefined && expiresAt !== null && expiresAt <= call.now) {
        throw new ServiceError("BAD_REQUEST", { message: "secret expiry must be in the future" });
    }

    // seal the value under the vault's newest key and create the version with it
    const id = call.id ?? secretVersion.generateId();
    const key = await keystore.latest(call.requireTransaction(), owner.parentId);
    const ciphertext = await SecretVersion.seal(key, owner, id, value);
    const created = await next(
        call.with({ id, input: { ...call.input, keyId: key.id, ciphertext } }),
    );

    // select it as current unless it stays pending
    if (stage !== "pending") {
        await call.mutate(call.caller, [
            secret.call("select", { id: owner.id, version: created.number }),
        ]);
    }

    return created;
}

/** Disable an intact version. */
async function disable(
    call: CallOf<typeof secretVersion, "disable">,
): Promise<ResultOf<typeof secretVersion, "disable">> {
    SecretVersion.requireIntact(call.target);

    return call.update({ disabledAt: call.now });
}

/** Enable an intact version. */
async function enable(
    call: CallOf<typeof secretVersion, "enable">,
): Promise<ResultOf<typeof secretVersion, "enable">> {
    SecretVersion.requireIntact(call.target);

    return call.update({ disabledAt: null });
}

/** Promote a readable version to its secret's current one. */
async function promote(
    call: CallOf<typeof secret, "promote">,
): Promise<ResultOf<typeof secret, "promote">> {
    // require a readable version
    const { version } = call.input;
    const selected = await SecretVersion.find(call.requireTransaction(), call.target.id, version);
    if (selected === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "secret version not found" });
    }
    SecretVersion.requireReadable(selected, call.now);

    return call.update({ currentVersion: version });
}

/** Erase an intact version's ciphertext, leaving its record. */
async function destroy(
    call: CallOf<typeof secretVersion, "destroy">,
): Promise<ResultOf<typeof secretVersion, "destroy">> {
    // require an intact version no live deployment runs with
    await requireReleased(call.requireTransaction(), call.target);

    // erase its ciphertext and mark the version destroyed
    return call.update(erasure(call.now));
}

/** Refuse destroying a version already destroyed or one a live deployment runs with. */
async function requireReleased(transaction: Transaction, version: SecretVersion): Promise<void> {
    SecretVersion.requireIntact(version);
    await Binding.requireReleased(transaction, {
        type: secret.name,
        scope: version.scope,
        id: version.parentId,
        version: version.number,
    });
}

/** Read how a need's declaration generates its secret, absent for a secret a stack binds. */
function generation(need: ResourceNeed): SecretGeneration | undefined {
    return SecretDescription.parse({ ...need.spec, name: need.name }).generated;
}

/** The fields of a version whose value is destroyed. */
function erasure(now: number) {
    return { destroyedAt: now, keyId: null, ciphertext: null };
}

/** Generate a secret's first value with an algorithm. */
async function generateValue(algorithm: SecretAlgorithm): Promise<SecretValue> {
    const keys = await crypto.subtle.generateKey(ES256, true, ["sign", "verify"]);
    const jwk = await crypto.subtle.exportKey("jwk", keys.privateKey);

    return { encoding: "text", value: JSON.stringify({ ...jwk, alg: algorithm }) };
}

/** Read a secret by its identifier, refusing an unknown one. */
async function readSecret(
    call: Call,
    id: Identifier<"secret">,
): Promise<RowImage<typeof secret.table>> {
    const row = await call.read(secret).row({ id });
    if (row === null) {
        throw new ServiceError("NOT_FOUND", { message: "secret not found" });
    }

    return row;
}

/** Relate the installation writing a secret as its custodian, acting itself or as a delegate. */
function custody(call: Call): Creation {
    const writer = Caller.principal(call.requireAuthorization().context());

    return {
        relationships:
            writer !== undefined && principal.installation.is(writer)
                ? [{ relation: "custodian", subject: writer }]
                : [],
    };
}
