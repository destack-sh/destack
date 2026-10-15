import { Caller, Manager, principal } from "@destack/access";
import { present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Binding, installation } from "@destack/stack/object";
import type { ServedBindable } from "@destack/stack/server";
import { SECRET_KIND, VAULT_PROVIDER } from "../declare/kind.ts";
import { SecretDescription } from "../declare/secret.ts";
import { secret, SecretVersion, secretVersion, vault } from "../object/index.ts";
import { SecretGeneration } from "../secret/secret.ts";
import vaultPackage from "../package.ts";

/** The secrets a stack declares, deleted to the trash once no longer declared. */
const declaredSecret = secret.implement({
    declaration: {
        after: [vault],
        resolve: async (_name, declared, stack) => {
            // wait for the vault to be provisioned before writing into it
            const vaultId = schema
                .identifier("vault")
                .parse(await stack.require(vault, declared.vault));
            const [provisioned] = await stack.transaction.read(vault.table).findMany({
                where: {
                    id: vaultId,
                    reference: { isNotNull: true },
                },
            });
            if (!provisioned) {
                stack.wait(`vault ${declared.vault} is not provisioned`);
            }

            return { parentId: vaultId, name: declared.name };
        },
        values: (_name, resolved) => resolved,
    },
});

/** Implement secrets, restorable from the trash within their declared recovery window. */
export function implementSecret(): typeof secret {
    return declaredSecret.implement({
        methods: {
            create: {
                // relate the installation writing the secret as its custodian, acting itself or as a delegate
                creation: (context) => {
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

                // store the generated value as its first version, current
                const value = await SecretGeneration.generate(call.input.algorithm);
                await call.mutate(call.caller, [
                    secretVersion.call("create", { parentId: target.id, value }),
                ]);

                return SecretVersion.owner(call, target.id);
            },
            promote: async (call) => {
                // require a readable version
                const { version } = call.input;
                const selected = await SecretVersion.find(call, call.target.id, version);
                if (selected === undefined) {
                    throw new CallError("NOT_FOUND", { message: "secret version not found" });
                }
                SecretVersion.requireReadable(selected, call.now);

                return call.update({ currentVersion: version });
            },
            delete: async (call, next) => {
                // delete a secret no binding targets
                await Binding.requireUnbound(call, call.target);

                return next();
            },
            discard: async (call, next) => {
                // discard the versions first, in one request
                const target = await call.update({ currentVersion: null });
                const versions = await call
                    .read(secretVersion)
                    .findMany({ where: { parentId: call.target.id } });
                if (versions.length > 0) {
                    await call.mutate(
                        call.caller,
                        versions.map(({ id }) => secretVersion.call("delete", { id })),
                    );
                }

                return next(call.with({ target }));
            },
            purge: async (call, next) => {
                // purge only an unbound secret
                const { target } = call;
                await Binding.requireUnbound(call, target);

                // erase the value of every intact version no live deployment runs with, in one request
                const intact = await call.reveal(secretVersion).findMany({
                    where: {
                        parentId: target.id,
                        destroyedAt: { isNull: true },
                    },
                });
                for (const version of intact) {
                    await SecretVersion.requireReleased(call, version);
                }
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
            apply: async (call, next) => {
                // require the managing installation in the secret's space
                const manager = Manager.schema.parse(call.input["manager"]);
                const [managing] = await call.read(installation).findMany({
                    where: {
                        scope: schema.identifier("space").parse(call.scope),
                        id: manager.installationId,
                    },
                });
                if (managing === undefined) {
                    throw new CallError("NOT_FOUND", {
                        message: `no installation ${manager.installationId} manages secrets in ${call.scope}`,
                    });
                }

                return next();
            },
        },
    });
}

/** Implement secret versions, each keeping its value sealed at rest by the space's database. */
export function implementSecretVersion(): typeof secretVersion {
    return secretVersion.implement({
        methods: {
            create: async (call, next) => {
                // require a secret outside the trash and an expiry in the future
                const { value, stage, expiresAt, parentId } = call.input;
                const owner = await SecretVersion.owner(call, parentId);
                // refuse a secret in the trash
                if (owner.deletedAt !== null) {
                    throw new CallError("CONFLICT", { message: "secret is in the trash" });
                }
                // refuse an expiry in the past
                else if (expiresAt !== undefined && expiresAt !== null && expiresAt <= call.now) {
                    throw new CallError("BAD_REQUEST", {
                        message: "secret expiry must be in the future",
                    });
                }

                // create the version with its value bound to it, within the size limit
                SecretVersion.requireSize(value);
                const id = call.id ?? (await call.generateId(secretVersion));
                const bound = { secret: owner.id, version: id, value };
                const created = await next(
                    call.with({ id, input: { ...call.input, content: bound } }),
                );

                // promote it to current unless it stays pending
                if (stage !== "pending") {
                    await call.mutate(call.caller, [
                        secret.call("promote", { id: owner.id, version: created.number }),
                    ]);
                }

                return created;
            },
            disable: async (call) => {
                SecretVersion.requireIntact(call.target);

                return call.update({ disabledAt: call.now });
            },
            enable: async (call) => {
                SecretVersion.requireIntact(call.target);

                return call.update({ disabledAt: null });
            },
            destroy: async (call) => {
                // require an intact version no live deployment runs with
                await SecretVersion.requireReleased(call, call.target);

                // erase its value and mark the version destroyed
                return call.update(SecretVersion.erasure(call.now));
            },
            erase: (call) => call.update(SecretVersion.erasure(call.now)),
        },
    });
}

/** Secrets that deployments capture at their current version or at a binding's fixed one. */
export const secretBindable: ServedBindable<typeof declaredSecret> = {
    object: declaredSecret,
    used: [declaredSecret, secretVersion],
    declaration: SECRET_KIND,
    provider: VAULT_PROVIDER,
    reference: vaultPackage.name,
    provision: async (call, _target, need, bindings) => {
        // leave a secret without generation to the stack binding it
        const generated = SecretDescription.generation(need);
        if (generated === undefined) {
            return undefined;
        }

        // require the installation's binding of the package's vault keeping it
        const holding = bindings.find(
            (entry) => entry.packageId === need.package.id && entry.name === generated.vault,
        );
        if (holding === undefined) {
            throw new CallError("PRECONDITION_FAILED", {
                message: `secret ${need.name} lives in vault ${generated.vault}, which the installation binds nowhere`,
            });
        }

        // create the secret under the identifier its binding takes, stable across retries
        const id = await call.generateId(secret);
        await call.mutate(call.self, [
            secret.call("provision", { id, parentId: holding.target, name: need.name }),
        ]);

        return id;
    },
    ready: async (call, bound, need) => {
        // read the bound secret and how its declaration generates it
        const id = schema.identifier("secret").parse(bound.target);
        const target = await call.read(secret.table).findFirst({ where: { id } });
        const generated = SecretDescription.generation(need);

        // refuse a binding whose secret is gone
        if (target === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `secret ${id} of binding ${bound.name} is gone`,
            });
        }
        // pass a secret with a version, and one a stack binds
        else if (target.currentVersion !== null || generated === undefined) {
            return true;
        }

        // wait for the vault to be provisioned
        const owner = await call.read(vault.table).findFirst({ where: { id: target.parentId } });
        if (present(owner, `vault ${target.parentId}`).reference === null) {
            return false;
        }

        // have the vault generate the first version, and wait for it
        await call.mutate(call.self, [
            secret.call("generate", { id, algorithm: generated.algorithm }),
        ]);

        return false;
    },
    version: (target, pin) => pin ?? target.currentVersion,
    uses: async (captured, snapshot) => {
        // use only the secret before its first version
        const secretId = schema.identifier("secret").parse(captured.target);
        if (captured.version === null) {
            return [secret.reference(captured.scope, secretId)];
        }

        // require the captured version
        const [version] = await snapshot.read(secretVersion.table).findMany({
            where: {
                parentId: secretId,
                number: captured.version,
            },
        });
        if (version === undefined) {
            throw new CallError("INTERNAL_SERVER_ERROR", {
                message: `secret ${secretId} has no version ${captured.version} its capture keeps`,
            });
        }

        return [
            secret.reference(captured.scope, secretId),
            secretVersion.reference(captured.scope, version.id),
        ];
    },
};
