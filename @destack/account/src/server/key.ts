import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { Revocation } from "../credential/revocation.ts";
import { Proof, PublicKey } from "@destack/identity";
import { client } from "../object/client.ts";
import { key, Key } from "../object/key.ts";
import { machine } from "../object/machine.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Implement keys. */
export function implementKey(): typeof key {
    return key.implement({ methods: { create } });
}

/** Rotate a standing client or machine key. */
async function create(
    context: MethodContextOf<typeof key, "create">,
    next: NextOf<typeof key, "create">,
): Promise<ResultOf<typeof key, "create">> {
    // find the standing holder in the key's scope
    const parent = context.requireParent();
    const holder = await holderOf(context, parent);

    // verify and spend the proof
    const publicKey = PublicKey.parse(context.input.publicKey);
    const proof = Proof.read(context.input.proof);
    await proof.verify(publicKey, parent.id, context.now, context.environment.crypto);
    await Key.spend(context, proof);
    await holder.report();

    // revoke the holder's other keys in one mutation and store the proven key
    const others = await Key.unrevoked(context, holder.type, parent.id);
    await context.mutate(
        context.caller,
        others.map((row) => key.call("revoke", key.target(row))),
    );

    return next(
        context.with({
            input: {
                ...context.input,
                ...(await Key.columns(publicKey, context.now, context.environment.crypto)),
            },
        }),
    );
}

/** Find the standing client or machine a key belongs to, with the report of its proven contact. */
async function holderOf(context: MethodContext, parent: ObjectReference) {
    // find a client of the user
    if (client.is(parent)) {
        const id = schema.identifier("client").parse(parent.id);
        const row = await context.read(client).findFirst({ where: { id } });
        Revocation.requireUnrevoked(
            requireFound(row?.scope === context.scope ? row : undefined, parent),
            "client",
        );

        return {
            type: client,
            report: () => context.mutate(context.self, [client.call("report", { id })]),
        };
    }
    // find a machine of the account
    else if (machine.is(parent)) {
        const id = schema.identifier("machine").parse(parent.id);
        const row = await context.read(machine).findFirst({ where: { id } });
        const found = Revocation.requireUnrevoked(
            requireFound(row?.scope === context.scope ? row : undefined, parent),
            "machine",
        );

        // report the contact, keeping the last report's contents
        const host =
            found.platform === null || found.architecture === null
                ? null
                : { platform: found.platform, architecture: found.architecture };
        const kept = {
            id,
            version: found.version,
            runtimes: found.runtimes,
            osVersion: found.osVersion,
            startedAt: found.startedAt,
            update: found.update,
            host,
        };

        return {
            type: machine,
            report: () => context.mutate(context.self, [machine.call("report", kept)]),
        };
    }
    // refuse any other parent
    else {
        throw AccountError.of("KEYS_MISSING", `a ${parent.type} has no keys`);
    }
}

/** Require a key's holder to exist. */
function requireFound<Row>(row: Row | undefined, parent: { readonly type: string }): Row {
    if (row === undefined) {
        throw AccountError.of("PARENT_NOT_FOUND", `${parent.type} not found`);
    }

    return row;
}
