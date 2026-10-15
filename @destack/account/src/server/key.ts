import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { ObjectReference } from "@destack/sync";
import { Revocation } from "../credential/revocation.ts";
import { Proof, PublicKey } from "@destack/identity";
import { client } from "../object/client.ts";
import { key, Key } from "../object/key.ts";
import { machine } from "../object/machine.ts";

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
    await proof.verify(publicKey, parent.id, context.now);
    await Key.spend(context, proof);
    await holder.report();

    // revoke the holder's other keys in one mutation and keep the proven key
    const others = await Key.unrevoked(context, holder.type, parent.id);
    await context.mutate(
        context.caller,
        others.map((row) => key.call("revoke", key.target(row))),
    );

    return next(
        context.with({
            input: { ...context.input, ...(await Key.columns(publicKey, context.now)) },
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
        const { version, runtimes } = Revocation.requireUnrevoked(
            requireFound(row?.scope === context.scope ? row : undefined, parent),
            "machine",
        );

        return {
            type: machine,
            report: () =>
                context.mutate(context.self, [machine.call("report", { id, version, runtimes })]),
        };
    }
    // refuse any other parent
    else {
        throw new CallError("BAD_REQUEST", { message: `a ${parent.type} has no keys` });
    }
}

/** Require a key's holder to exist. */
function requireFound<Row>(row: Row | undefined, parent: { readonly type: string }): Row {
    if (row === undefined) {
        throw new CallError("NOT_FOUND", { message: `${parent.type} not found` });
    }

    return row;
}
