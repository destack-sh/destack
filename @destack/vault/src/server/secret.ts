import { SECRET_KIND } from "@destack/stack/declare";
import { AccessContext, Caller, principal } from "@destack/access";
import { Duration, nth, schema } from "@destack/schema";
import { Binding } from "@destack/stack/object";
import type { ObjectBindable } from "@destack/stack/object";
import { VAULT_PROVIDER } from "../declare/kind.ts";
import { SecretDescription } from "../declare/secret.ts";
import { ROTATION_OVERLAP, secret, secretVersion } from "../object/index.ts";
import { SecretVersion } from "./version.ts";
import { type Secret, SecretReading, type SecretValue } from "@destack/resource";
import { SecretGeneration } from "../secret/secret.ts";
import { VAULT_REFERENCE } from "../provider/provider.ts";
import { VaultError } from "../error/error.ts";

/** Implement secrets, restorable from the trash within their declared recovery window. */
export function implementSecret(): typeof secret {
    return secret.implement({
        methods: {
            create: {
                // relate the installation writing the secret as its custodian, acting itself or as a delegate
                relate: (context) => {
                    const writer = Caller.principal(context.caller);
                    const isInstallation =
                        writer !== undefined && principal.installation.is(writer);

                    return {
                        relationships: isInstallation
                            ? [{ relation: "custodian", subject: writer }]
                            : [],
                    };
                },
            },
            read: (call) => SecretVersion.read(call.target, call.input.version, call),
            generate: async (call) => {
                // leave a secret that has a version
                const { target } = call;
                if (target.currentVersion !== null) {
                    return target;
                }

                // store the generated value as its first version, and point the current alias at it
                const { algorithm } = call.input;
                const value = await SecretGeneration.generate(algorithm, call.environment.crypto);
                const [created] = await call.mutate(call.caller, [
                    secretVersion.call("create", { parentId: target.id, value }),
                ]);

                return call.update({ currentVersion: created.number, algorithm });
            },
            versions: (call) => SecretVersion.versions(call.target, call.input.version, call),
            rotate: async (call) => {
                // choose the new value
                const { target } = call;
                const { value, expiresIn = ROTATION_OVERLAP } = call.input;
                let next: SecretValue;
                // take the given value
                if (value !== undefined) {
                    next = value;
                }
                // generate one by the secret's algorithm
                else if (target.algorithm !== null) {
                    next = await SecretGeneration.generate(
                        target.algorithm,
                        call.environment.crypto,
                    );
                }
                // refuse a secret without one
                else {
                    throw VaultError.of(
                        "VALUE_REQUIRED",
                        "value is required: a secret without an algorithm rotates to a given value",
                    );
                }

                // retire the current version after the overlap, and make the new one current
                const expiresAt = call.now + Duration.milliseconds(expiresIn);
                const current =
                    target.currentVersion === null
                        ? undefined
                        : await SecretVersion.find(call, target.id, target.currentVersion);
                const written = await call.mutate(call.caller, [
                    ...(current === undefined
                        ? []
                        : [secretVersion.call("expire", { id: current.id, expiresAt })]),
                    secretVersion.call("create", { parentId: target.id, value: next }),
                ]);

                return call.update({ currentVersion: nth(written, -1).number });
            },
            update: async (call, next) => {
                // refuse unsetting the current alias, and require the version it moves to to be readable
                const { currentVersion } = call.input;
                if (currentVersion === null) {
                    throw VaultError.of(
                        "CURRENT_VERSION_REQUIRED",
                        "current version is required: a secret's current version cannot be unset",
                    );
                } else if (currentVersion !== undefined) {
                    const selected = await SecretVersion.find(call, call.target.id, currentVersion);
                    if (selected === undefined) {
                        throw VaultError.of("VERSION_NOT_FOUND", "secret version not found");
                    }
                    SecretVersion.requireReadable(selected, call.now);
                }

                return next();
            },
            delete: async (call, next) => {
                // delete a secret no binding targets
                await Binding.requireUnbound(call, call.target);

                return next();
            },
            purge: async (call, next) => {
                // refuse purging a secret a binding targets or a live deployment runs with
                const { target } = call;
                await Binding.requireUnbound(call, target);

                // erase every intact value in one request, reading only which versions they are
                const intact = await call.read(secretVersion).findMany({
                    where: {
                        parentId: target.id,
                        state: { in: ["enabled", "disabled"] },
                    },
                });
                if (intact.length > 0) {
                    await call.mutate(
                        call.caller,
                        intact.map((version) =>
                            secretVersion.call("erase", secretVersion.target(version)),
                        ),
                    );
                }

                return next();
            },
        },
    });
}

/** Implement secret versions, each keeping its value encrypted at rest by the space's database. */
export function implementSecretVersion(): typeof secretVersion {
    return secretVersion.implement({
        methods: {
            create: async (call, next) => {
                // refuse an expiry in the past
                const { value, expiresAt, parentId } = call.input;
                if (expiresAt !== undefined && expiresAt <= call.now) {
                    throw VaultError.of(
                        "EXPIRY_IN_PAST_INVALID",
                        "secret expiry is invalid: it must be in the future",
                    );
                }

                // create the version with its value bound to it, within the size limit
                SecretVersion.requireSize(value);
                const id = call.id ?? (await call.generateId(secretVersion));
                const bound = { secret: parentId, version: id, value };

                return next(call.with({ id, input: { ...call.input, content: bound } }));
            },
            expire: async (call) => {
                // refuse an expiry in the past, and a destroyed version
                SecretVersion.requireIntact(call.target);
                const { expiresAt } = call.input;
                if (expiresAt <= call.now) {
                    throw VaultError.of(
                        "EXPIRY_IN_PAST_INVALID",
                        "secret expiry is invalid: it must be in the future",
                    );
                }

                return call.update({ expiresAt });
            },
            disable: async (call) => {
                SecretVersion.requireIntact(call.target);

                return call.update({ state: "disabled" });
            },
            enable: async (call) => {
                SecretVersion.requireIntact(call.target);

                return call.update({ state: "enabled" });
            },
            destroy: async (call) => {
                // require an intact version no live deployment runs with
                await SecretVersion.requireReleased(call, call.target);

                // erase its value and mark the version destroyed
                return call.update(SecretVersion.erasure);
            },
            erase: (call) => call.update(SecretVersion.erasure),
        },
    });
}

/** Secrets that deployments capture at their current version or at a binding's fixed one. */
export const secretBindable: ObjectBindable<typeof secret, Secret> = {
    object: secret,
    used: [secret, secretVersion],
    kind: SECRET_KIND,
    provider: VAULT_PROVIDER,
    reference: VAULT_REFERENCE,
    declare: (need, bindings) => {
        // leave a secret without generation to the stack binding it
        const generated = SecretDescription.generation(need);
        if (generated === undefined) {
            return undefined;
        }

        // declare it in the vault the installation's stack binds, else in the vault it declares itself
        const holding = bindings.find(
            (entry) => entry.packageId === need.package.id && entry.name === generated.vault,
        );

        return {
            name: need.name,
            parent: holding === undefined ? generated.vault : { id: holding.target },
        };
    },
    ready: async (call, bound, need) => {
        // read the bound secret and how its declaration generates it
        const id = schema.identifier("secret").parse(bound.target);
        const target = await call.read(secret.table).findFirst({ where: { id } });
        const generated = SecretDescription.generation(need);

        // require the bound secret, which deletion refuses while bound
        if (target === undefined) {
            throw new TypeError(`secret '${id}' of binding '${bound.name}' is gone while bound`);
        }

        // pass a secret with a version, and one a stack binds
        return target.currentVersion !== null || generated === undefined;
    },
    prepare: async (call, declarer, bound, need) => {
        // have the provisioned vault generate a generated secret's first version, as the declarer
        const id = schema.identifier("secret").parse(bound.target);
        const generated = SecretDescription.generation(need);
        if (generated !== undefined) {
            await call.mutate(declarer, [
                secret.call("generate", { id, algorithm: generated.algorithm }),
            ]);
        }
    },
    connect: async (serving, row, bound) => {
        // read the captured version, alone or with the retiring ones
        const query = async (method: "read" | "versions") => {
            // read as the installation, once the store serves
            const store = await serving;
            const id = secret.identifier(bound.target);
            const reader = principal.installation.reference(row.scope, row.installationId);

            return store.query(
                AccessContext.of(Caller.of(reader), store.clock.now()),
                secret.call(method, {
                    ...secret.target({ scope: row.scope, id }),
                    ...(bound.version === null ? {} : { version: bound.version }),
                }),
            );
        };

        return {
            read: async () => SecretReading.parse(await query("read")),
            versions: async () =>
                schema.tuple([SecretReading], SecretReading).parse(await query("versions")),
        };
    },
    version: (target, pin) => pin ?? target.currentVersion,
    uses: async (captured, snapshot) => {
        // use only the secret before its first version
        const secretId = schema.identifier("secret").parse(captured.target);
        if (captured.version === null) {
            return [secret.reference(captured.scope, secretId)];
        }

        // require the captured version, and use the earlier ones retiring with an expiry beside it
        const number = captured.version;
        const versions = await snapshot.read(secretVersion.table).findMany({
            where: { parentId: secretId, number: { lte: number } },
        });
        const version = versions.find((entry) => entry.number === number);
        if (version === undefined) {
            throw new TypeError(
                `secret '${secretId}' has no version '${number}', which its capture references`,
            );
        }
        const retiring = versions.filter(
            (entry) =>
                entry.number < number && entry.state === "enabled" && entry.expiresAt !== null,
        );

        return [
            secret.reference(captured.scope, secretId),
            ...[version, ...retiring].map((entry) =>
                secretVersion.reference(captured.scope, entry.id),
            ),
        ];
    },
};
