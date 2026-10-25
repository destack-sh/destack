import { Caller } from "@destack/access";
import { present } from "@destack/schema";
import { Binding } from "@destack/stack/object";
import { secret, secretVersion, vault } from "../object/index.ts";
import { SecretVersion } from "./version.ts";
import { VaultError } from "../error/error.ts";

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

                // refuse a missing secret and one the caller may not read alike, leaking no names
                const reference =
                    named === undefined ? undefined : secret.reference(call.scope, named.id);
                const readable =
                    reference === undefined
                        ? undefined
                        : await call.check(secret.permission("read"), reference);
                if (named === undefined || readable?.isAllowed !== true) {
                    throw VaultError.of("SECRET_NOT_FOUND", `vault has no secret ${name}`);
                }

                return SecretVersion.read(named, version, call);
            },
            put: async (call) => {
                // find the secret, or create it under its name bound to its caller as the consumer
                const { target, input } = call;
                const { name } = input;
                const consumer = present(
                    Caller.principal(call.caller),
                    "the consumer putting a secret",
                );
                const id = input.secret;
                const [found] = await call.read(secret).findMany({
                    where: { id, purgedAt: { isNull: true } },
                });
                const named = secret.target({ scope: target.scope, id });

                // refuse a secret of another vault as missing, which reveals nothing of it
                if (found !== undefined && found.parentId !== target.id) {
                    throw VaultError.of("SECRET_NOT_FOUND", `vault has no secret ${id}`);
                }
                // refuse a secret of this vault under another name
                else if (found !== undefined && found.name !== name) {
                    throw VaultError.of(
                        "SECRET_MISMATCH",
                        `secret ${id} is ${found.name} in this vault, not ${name}`,
                    );
                }
                // require updating a secret it has
                else if (found !== undefined) {
                    await call.require(
                        secret.permission("update"),
                        secret.reference(target.scope, id),
                    );
                }
                const creation =
                    found === undefined
                        ? [
                              secret.call("create", { ...named, parentId: target.id, name }),
                              secret.call("bindConsumer", { ...named, consumer }),
                          ]
                        : [];

                // release and destroy the earlier intact versions
                const before =
                    found === undefined
                        ? []
                        : await call.read(secretVersion).findMany({
                              where: { parentId: id, state: { in: ["enabled", "disabled"] } },
                          });
                const retiring = before.flatMap((earlier) => [
                    secretVersion.call("releaseConsumer", {
                        ...secretVersion.target(earlier),
                        consumer,
                    }),
                    secretVersion.call("destroy", secretVersion.target(earlier)),
                ]);

                // store the value as a new version bound to the consumer
                const version = secretVersion.target({
                    scope: target.scope,
                    id: await call.generateId(secretVersion),
                });
                if (creation.length > 0) {
                    await call.mutate(call.caller, creation);
                }
                const [created] = await call.mutate(call.caller, [
                    secretVersion.call("create", { ...version, parentId: id, value: input.value }),
                ]);

                // point the current alias at it, then retire the earlier versions
                await call.mutate(call.caller, [
                    secretVersion.call("bindConsumer", { ...version, consumer }),
                    secret.call("update", { id, currentVersion: created.number }),
                    ...retiring,
                ]);

                return target;
            },
            remove: async (call) => {
                // pass a secret the vault keeps no longer, so a repeated removal succeeds
                const { target, input } = call;
                const consumer = present(
                    Caller.principal(call.caller),
                    "the consumer removing a secret",
                );
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
            delete: async (call, next) => {
                // delete a vault no binding targets, its secrets moving to the trash with it
                await Binding.requireUnbound(call, call.target);

                return next();
            },
            finalize: async (call, next) => {
                // purge each secret and discard its record once the vault's retention ends
                const secrets = await call
                    .read(secret)
                    .findMany({ where: { parentId: call.target.id } });
                const calls = secrets.flatMap((row) => [
                    ...(row.purgedAt === null ? [secret.call("purge", secret.target(row))] : []),
                    secret.call("discard", secret.target(row)),
                ]);
                if (calls.length > 0) {
                    await call.mutate(call.caller, calls);
                }

                return next();
            },
        },
    });
}
