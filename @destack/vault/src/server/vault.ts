import { CallError } from "@destack/runtime/error";
import { secret, SecretVersion, secretVersion, vault } from "../object/index.ts";

/** Implement vaults: each keeps its secrets by name. */
export function implementVault(): typeof vault {
    return vault.implement({
        methods: {
            read: async (call) => {
                // find the secret of the name in the vault, outside the trash
                const { name, version } = call.input;
                const [named] = await call.read(secret).findMany({
                    where: {
                        parentId: call.target.id,
                        name,
                        deletedAt: { isNull: true },
                    },
                });

                // refuse a missing secret and one the caller may not open alike, leaking no names
                const reference =
                    named === undefined ? undefined : secret.reference(call.scope, named.id);
                const opening =
                    reference === undefined
                        ? undefined
                        : await call.check(secret.permission("read"), reference);
                if (named === undefined || opening?.isAllowed !== true) {
                    throw new CallError("NOT_FOUND", { message: `vault has no secret ${name}` });
                }

                return SecretVersion.read(named, version, call);
            },
            put: async (call) => {
                // find the secret, or create it under its name bound to the consumer
                const { target, input } = call;
                const { consumer, name } = input;
                const id = input.secret;
                const [found] = await call.read(secret).findMany({
                    where: {
                        parentId: target.id,
                        id,
                        purgedAt: { isNull: true },
                    },
                });
                const named = secret.target({ scope: target.scope, id });
                const creation =
                    found === undefined
                        ? [
                              secret.call("create", { ...named, parentId: target.id, name }),
                              secret.call("bindConsumer", { ...named, consumer }),
                          ]
                        : [];

                // retire the intact versions before
                const before =
                    found === undefined
                        ? []
                        : await call
                              .read(secretVersion)
                              .findMany({ where: { parentId: id, destroyedAt: { isNull: true } } });
                const retiring = before.flatMap((earlier) => [
                    secretVersion.call("releaseConsumer", {
                        ...secretVersion.target(earlier),
                        consumer,
                    }),
                    secretVersion.call("destroy", secretVersion.target(earlier)),
                ]);

                // store the value as the current version, bound to the consumer, in one request
                const version = secretVersion.target({
                    scope: target.scope,
                    id: await call.generateId(secretVersion),
                });
                await call.mutate(call.caller, [
                    ...creation,
                    secretVersion.call("create", { ...version, parentId: id, value: input.value }),
                    secretVersion.call("bindConsumer", { ...version, consumer }),
                    ...retiring,
                ]);

                return target;
            },
            remove: async (call) => {
                // pass a secret the vault keeps no longer, so a repeated removal succeeds
                const { target, input } = call;
                const { consumer } = input;
                const [found] = await call.read(secret).findMany({
                    where: {
                        parentId: target.id,
                        id: input.secret,
                        purgedAt: { isNull: true },
                    },
                });
                if (found === undefined) {
                    return target;
                }

                // release it and its versions from the consumer, then trash and purge it
                const versions = await call
                    .read(secretVersion)
                    .findMany({ where: { parentId: found.id } });
                const named = secret.target(found);
                await call.mutate(call.caller, [
                    secret.call("releaseConsumer", { ...named, consumer }),
                    ...versions.map((version) =>
                        secretVersion.call("releaseConsumer", {
                            ...secretVersion.target(version),
                            consumer,
                        }),
                    ),
                    secret.call("delete", named),
                    secret.call("purge", named),
                ]);

                return target;
            },
            finalize: async (call, next) => {
                // discard the records of its purged secrets in one request
                const purged = await call.read(secret).findMany({
                    where: { parentId: call.target.id, purgedAt: { isNotNull: true } },
                });
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
