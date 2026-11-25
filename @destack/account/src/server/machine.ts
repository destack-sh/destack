import { principal } from "@destack/access";
import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { Scope } from "@destack/sync";
import { Proof, type PublicKey } from "@destack/identity";
import type { Select } from "@destack/db";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import type { UniverseOrigin } from "../address/address.ts";
import { key, Key } from "../object/key.ts";
import { machine, Machine } from "../object/machine.ts";
import { region } from "../object/region.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The relationship letting every machine verify a machine's keys. */
const PEERS = {
    relationships: [{ relation: "peers", subject: principal.machine.reference("*", "*") }],
};

/** Implement machines. */
export function implementMachine(universe: UniverseOrigin): typeof machine {
    return machine.implement({
        methods: {
            // let every machine verify a new machine's keys, as its peers in moves
            enroll: { handle: enroll, relate: () => PEERS },
            declare: { handle: declare, relate: () => PEERS },
            delete: remove,
            relay: (call) => relay(call, universe),
            enable: (call) => transition(call, "enabled"),
            drain: (call) => transition(call, "draining"),
            disable: (call) => transition(call, "disabled"),
            rate: (call) => call.update({ rates: call.input.rates }),
            rename,
        },
    });
}

/** Enroll a machine under the first free variant of its name, with the key in its proof. */
async function enroll(
    context: MethodContextOf<typeof machine, "enroll">,
    next: NextOf<typeof machine, "enroll">,
): Promise<ResultOf<typeof machine, "enroll">> {
    // require the right to operate the region a managed machine would serve
    const { input } = context;
    if (input.kind === "managed") {
        const served = region.reference(Scope.universe.id, input.regionId);
        await context.require(region.permission("operate"), served);
    }

    // require the machine's proof by the key it registers
    if (context.id === undefined) {
        throw AccountError.of(
            "PROOF_INVALID",
            "proof is invalid: a machine must enroll under the identifier in its proof",
        );
    }
    const proof = Proof.read(input.proof);
    await proof.verify(input.publicKey, context.id, context.now, context.environment.crypto);
    await Key.spend(context, proof);

    // create the machine with its first key under the first free variant of its name
    const name = await Machine.uniqueName(context, context.scope, input.name);
    const row = await next(context.with({ input: { ...input, name, lastSeenAt: context.now } }));
    await provision(context, row, input.publicKey);

    return row;
}

/** Store a machine the universe declares with its key, as the store trusting the declaration. */
async function declare(
    context: MethodContextOf<typeof machine, "declare">,
    next: NextOf<typeof machine, "declare">,
): Promise<ResultOf<typeof machine, "declare">> {
    // create the machine with its key and its endpoint at its origin
    const row = await next();
    await provision(context, row, context.input.publicKey);
    await Endpoint.upsert(context, row.id, {
        url: context.input.origin,
        relay: null,
        lapsesAt: null,
    });

    return row;
}

/** Record a self-hosted machine's tunnel at its origin. */
async function relay(
    context: MethodContextOf<typeof machine, "relay">,
    universe: UniverseOrigin,
): Promise<ResultOf<typeof machine, "relay">> {
    // require a self-hosted machine, as a managed one answers at its URL
    if (context.target.kind === "managed") {
        throw AccountError.of(
            "MACHINE_MANAGED_DENIED",
            `machine '${context.target.id}' is forbidden: it is a managed machine, so it answers at its URL`,
        );
    }

    // replace the machine's endpoint, or record its first one
    await Endpoint.upsert(context, context.target.id, {
        url: universe.machine(context.target.id),
        relay: context.input.url,
        lapsesAt: context.input.lapsesAt,
    });

    return context.target;
}

/** Delete a machine, revoking its keys and its endpoint. */
async function remove(
    context: MethodContextOf<typeof machine, "delete">,
    next: NextOf<typeof machine, "delete">,
): Promise<ResultOf<typeof machine, "delete">> {
    // end the machine's keys
    await invokeKeys(context, context.target.id, "revoke");

    // delete the machine with its endpoint
    const published = await context
        .read(endpoint)
        .findFirst({ where: { parentId: context.target.id } });
    if (published !== undefined) {
        await context.mutate(context.caller, [endpoint.call("delete", endpoint.target(published))]);
    }

    return next();
}

/** Move a machine to a status, suspending its keys while it is disabled. */
async function transition(
    context: MethodContextOf<typeof machine, "enable" | "drain" | "disable">,
    status: Machine["status"],
): Promise<ResultOf<typeof machine, "enable" | "drain" | "disable">> {
    // suspend the keys on disabling
    if (status === "disabled" && context.target.status !== "disabled") {
        await invokeKeys(context, context.target.id, "suspend");
    }
    // resume them on leaving disabled
    else if (status !== "disabled" && context.target.status === "disabled") {
        await invokeKeys(context, context.target.id, "resume");
    }

    return context.update({ status });
}

/** Rename a machine. */
async function rename(
    context: MethodContextOf<typeof machine, "rename">,
): Promise<ResultOf<typeof machine, "rename">> {
    return context.update({ name: context.input.name });
}

/** Provision a new machine's first key. */
async function provision(
    context: MethodContext,
    created: Select<typeof machine.table>,
    publicKey: PublicKey,
): Promise<void> {
    // enroll the key as the store
    await context.mutate(context.self, [
        key.call("enroll", {
            parent: { ...machine.typeReference, id: created.id },
            ...(await Key.columns(publicKey, context.now, context.environment.crypto)),
        }),
    ]);
}

/** Invoke a method on each of a machine's unrevoked keys as the store. */
async function invokeKeys(
    context: MethodContext,
    machineId: Machine["id"],
    name: "revoke" | "suspend" | "resume",
): Promise<void> {
    const keys = await Key.unrevoked(context, machine, machineId);
    await context.mutate(
        context.self,
        keys.map((row) => key.call(name, key.target(row))),
    );
}
