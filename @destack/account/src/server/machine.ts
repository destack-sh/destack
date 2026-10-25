import { principal } from "@destack/access";
import { present, schema } from "@destack/schema";
import { Entitlement, entitlement } from "../object/entitlement.ts";
import type { MethodContext, MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { Scope } from "@destack/sync";
import { Revocation } from "../credential/revocation.ts";
import { Proof, type PublicKey } from "@destack/identity";
import type { Select } from "@destack/db";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { account } from "../object/account.ts";
import { DOMAINS, MachineAddress } from "../address/address.ts";
import { key, Key } from "../object/key.ts";
import { machine, Machine } from "../object/machine.ts";
import { residency } from "../object/residency.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { StoreRelationship } from "./relationship.ts";

/** Implement machines. */
export function implementMachine(): typeof machine {
    return machine.implement({
        methods: {
            enroll,
            declare,
            revoke,
            publish,
            relay,
            enable: (call) => transition(call, "enabled"),
            drain: (call) => transition(call, "draining"),
            disable: (call) => transition(call, "disabled"),
            rename,
        },
    });
}

/** Enroll a machine under the first free variant of its name, with the key in its proof. */
async function enroll(
    context: MethodContextOf<typeof machine, "enroll">,
    next: NextOf<typeof machine, "enroll">,
): Promise<ResultOf<typeof machine, "enroll">> {
    // require the right to host the residency a cloud machine would serve
    const { input } = context;
    if (input.kind === "cloud") {
        const served = residency.reference(Scope.universe.id, input.residencyId);
        await context.require(residency.permission("operate"), served);
    }

    // require the machine's proof by the key it registers
    if (context.id === undefined) {
        throw AccountError.of(
            "PROOF_MISMATCH",
            "a machine must enroll under the identifier in its proof",
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
    // require an account's dedicated machine to be one its plan unlocks
    if (context.scope !== Scope.universe.id) {
        const accountId = schema.identifier("account").parse(context.scope);
        const entitled = await context.read(entitlement).findMany({ where: { scope: accountId } });
        const allowed = Entitlement.decide(entitled, { tenancy: "dedicated" });
        if (allowed.status !== "true") {
            throw AccountError.of(
                "DEDICATED_TENANCY_UNENTITLED",
                `${accountId} has no dedicated tenancy: ${allowed.message}`,
            );
        }
    }

    // create the machine with its key
    const row = await next();
    await provision(context, row, context.input.publicKey);

    return row;
}

/** Publish where a standing machine answers, replacing what it published before. */
async function publish(
    context: MethodContextOf<typeof machine, "publish">,
): Promise<ResultOf<typeof machine, "publish">> {
    // require a standing cloud machine, as a personal one answers only through a relay
    Revocation.requireUnrevoked(context.target, "machine");
    if (context.target.kind === "personal") {
        throw AccountError.of(
            "MACHINE_PERSONAL",
            `machine ${context.target.id} is personal: it answers only through a relay`,
        );
    }

    // replace the published endpoint, or publish the first one
    await Endpoint.upsert(context, context.target.id, { url: context.input.url, relay: null });

    return context.target;
}

/** Publish the tunnel a relay accepted from a standing personal machine, at its derived name. */
async function relay(
    context: MethodContextOf<typeof machine, "relay">,
): Promise<ResultOf<typeof machine, "relay">> {
    // require a standing personal machine, as a cloud one answers at its URL
    Revocation.requireUnrevoked(context.target, "machine");
    if (context.target.kind === "cloud") {
        throw AccountError.of(
            "MACHINE_CLOUD",
            `machine ${context.target.id} is a cloud machine: it answers at its URL`,
        );
    }

    // address the machine below its account's handle
    const owner = present(
        await context
            .read(account)
            .findFirst({ where: { id: account.identifier(context.target.scope) } }),
        `the account of machine ${context.target.id}`,
    );
    const name = MachineAddress.format(
        { machine: context.target.name, handle: owner.handle },
        DOMAINS.machine,
    );

    // replace the published endpoint, or publish the first one
    await Endpoint.upsert(context, context.target.id, {
        url: `https://${name}`,
        relay: context.input,
    });

    return context.target;
}

/** Withdraw a machine with its keys. */
async function revoke(
    context: MethodContextOf<typeof machine, "revoke">,
): Promise<ResultOf<typeof machine, "revoke">> {
    // end the machine's keys and its place among its residency's servers
    Revocation.requireUnrevoked(context.target, "machine");
    await invokeKeys(context, context.target.id, "revoke");
    await leaveResidency(context, Machine.of(context.target));

    // withdraw the machine with its endpoint
    const published = await context.read(endpoint).findFirst({ where: { id: context.target.id } });
    if (published !== undefined) {
        await context.mutate(context.caller, [endpoint.call("delete", endpoint.target(published))]);
    }

    return context.update({ revokedAt: context.now });
}

/** Move a standing machine to a status, suspending its keys while it is disabled. */
async function transition(
    context: MethodContextOf<typeof machine, "enable" | "drain" | "disable">,
    status: Machine["status"],
): Promise<ResultOf<typeof machine, "enable" | "drain" | "disable">> {
    // refuse a withdrawn machine
    Revocation.requireUnrevoked(context.target, "machine");

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

/** Rename a standing machine. */
async function rename(
    context: MethodContextOf<typeof machine, "rename">,
): Promise<ResultOf<typeof machine, "rename">> {
    Revocation.requireUnrevoked(context.target, "machine");

    return context.update({ name: context.input.name });
}

/** Provision a new machine's first key, and relate a cloud machine to its residency. */
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

    // relate a cloud machine to its residency
    const row = Machine.of(created);
    if (row.kind === "cloud") {
        await StoreRelationship.touch(context, residencyServer(row));
    }
}

/** Revoke a cloud machine's place among its residency's servers. */
async function leaveResidency(context: MethodContext, row: Machine): Promise<void> {
    // revoke a cloud machine's relationship to its residency
    if (row.kind === "cloud") {
        await StoreRelationship.delete(context, residencyServer(row));
    }
}

/** Read a cloud machine's relationship to the residency it serves. */
function residencyServer(row: Extract<Machine, { kind: "cloud" }>) {
    return {
        object: residency.reference(Scope.universe.id, row.residencyId),
        relation: "machine",
        subject: principal.machine.reference(Scope.universe.id, row.id),
    };
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
