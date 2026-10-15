import { principal, Relationship } from "@destack/access";
import { present } from "@destack/schema";
import type { Call, CallOf, NextOf, ResultOf } from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { Scope } from "@destack/sync";
import { Revocation } from "../credential/revocation.ts";
import { Proof, type PublicKey } from "@destack/identity";
import type { Select } from "@destack/db";
import { endpoint } from "../object/endpoint.ts";
import { key, Key } from "../object/key.ts";
import { machine, Machine } from "../object/machine.ts";
import { relationship } from "../object/relationship.ts";
import { residency } from "../object/residency.ts";

/** Implement machines. */
export function implementMachine(): typeof machine {
    return machine.implement({
        methods: {
            enroll,
            keep,
            revoke,
            publish,
            enable: (call) => transition(call, "enabled"),
            drain: (call) => transition(call, "draining"),
            disable: (call) => transition(call, "disabled"),
            rename,
        },
    });
}

/** Enroll a machine under the first free variant of its name, with the key in its proof. */
async function enroll(
    call: CallOf<typeof machine, "enroll">,
    next: NextOf<typeof machine, "enroll">,
): Promise<ResultOf<typeof machine, "enroll">> {
    // require the right to host the residency a cloud machine would serve
    const { input } = call;
    if (input.kind === "cloud") {
        const served = residency.reference(Scope.universe.id, input.residencyId);
        await call.require(residency.permission("operate"), served);
    }

    // require the machine's proof by the key it registers
    if (call.id === undefined) {
        throw new CallError("BAD_REQUEST", {
            message: "a machine enrolls under the identifier its proof carries",
        });
    }
    const proof = Proof.read(input.proof);
    await proof.verify(input.publicKey, call.id, call.now);
    await Key.spend(call, proof);

    // create the machine with its first key under the first free variant of its name
    const name = await Machine.uniqueName(call, call.scope, input.name);
    const row = await next(call.with({ input: { ...input, name, lastSeenAt: call.now } }));
    await provision(call, row, input.publicKey);

    return row;
}

/** Keep a machine the universe declares with its key, as the universe trusting the declaration. */
async function keep(
    call: CallOf<typeof machine, "keep">,
    next: NextOf<typeof machine, "keep">,
): Promise<ResultOf<typeof machine, "keep">> {
    // create the machine with its key
    const row = await next();
    await provision(call, row, call.input.publicKey);

    return row;
}

/** Publish where a standing machine answers, replacing what it published before. */
async function publish(
    call: CallOf<typeof machine, "publish">,
): Promise<ResultOf<typeof machine, "publish">> {
    // require a standing machine
    Revocation.requireUnrevoked(call.target, "machine");

    // replace the published endpoint, or publish the first one
    const target = endpoint.target({ scope: call.scope, id: call.target.id });
    const published = await call.read(endpoint).findFirst({ where: { id: call.target.id } });
    const { url, relay } = call.input;
    await call.mutate(call.caller, [
        published === undefined
            ? endpoint.call("create", { ...target, parentId: call.target.id, url, relay })
            : endpoint.call("update", { ...target, url, relay }),
    ]);

    return call.target;
}

/** Withdraw a machine with its keys. */
async function revoke(
    call: CallOf<typeof machine, "revoke">,
): Promise<ResultOf<typeof machine, "revoke">> {
    // end the machine's keys and its place among its residency's servers
    Revocation.requireUnrevoked(call.target, "machine");
    await invokeKeys(call, call.target.id, "revoke");
    await leaveResidency(call, Machine.of(call.target));

    // withdraw the machine with its endpoint
    const published = await call.read(endpoint).findFirst({ where: { id: call.target.id } });
    if (published !== undefined) {
        await call.mutate(call.caller, [endpoint.call("delete", endpoint.target(published))]);
    }

    return call.update({ revokedAt: call.now });
}

/** Move a standing machine to a status, suspending its keys while it is disabled. */
async function transition(
    call: CallOf<typeof machine, "enable" | "drain" | "disable">,
    status: Machine["status"],
): Promise<ResultOf<typeof machine, "enable" | "drain" | "disable">> {
    // refuse a withdrawn machine
    Revocation.requireUnrevoked(call.target, "machine");

    // suspend the keys on disabling
    if (status === "disabled" && call.target.status !== "disabled") {
        await invokeKeys(call, call.target.id, "suspend");
    }
    // resume them on leaving disabled
    else if (status !== "disabled" && call.target.status === "disabled") {
        await invokeKeys(call, call.target.id, "resume");
    }

    return call.update({ status });
}

/** Rename a standing machine. */
async function rename(
    call: CallOf<typeof machine, "rename">,
): Promise<ResultOf<typeof machine, "rename">> {
    Revocation.requireUnrevoked(call.target, "machine");

    return call.update({ name: call.input.name });
}

/** Provision a new machine's first key, and relate a cloud machine to its residency. */
async function provision(
    call: Call,
    created: Select<typeof machine.table>,
    publicKey: PublicKey,
): Promise<void> {
    // enroll the key as the universe
    await call.mutate(call.self, [
        key.call("enroll", {
            parent: { ...machine.typeReference, id: created.id },
            ...(await Key.columns(publicKey, call.now)),
        }),
    ]);

    // relate a cloud machine to its residency
    const row = Machine.of(created);
    if (row.kind === "cloud") {
        await call.mutate(call.self, [
            relationship.call("create", {
                ...relationship.in(Scope.universe.id),
                ...residencyServer(row),
            }),
        ]);
    }
}

/** Revoke a cloud machine's place among its residency's servers. */
async function leaveResidency(call: Call, row: Machine): Promise<void> {
    // find a cloud machine's relationship to its residency
    if (row.kind !== "cloud") {
        return;
    }
    const { object, relation, subject } = residencyServer(row);
    const [related] = await Relationship.readOn(call, object, { relation, subject });

    // revoke it as the store
    await call.mutate(call.self, [
        relationship.call("delete", {
            ...relationship.in(Scope.universe.id),
            object,
            relationshipId: present(related, `the server relationship of ${row.id}`).id,
        }),
    ]);
}

/** Read a cloud machine's relationship to the residency it serves. */
function residencyServer(row: Extract<Machine, { kind: "cloud" }>) {
    return {
        object: residency.reference(Scope.universe.id, row.residencyId),
        relation: "machine",
        subject: principal.machine.reference(Scope.universe.id, row.id),
    };
}

/** Invoke a method on each of a machine's unrevoked keys as the universe. */
async function invokeKeys(
    call: Call,
    machineId: Machine["id"],
    name: "revoke" | "suspend" | "resume",
): Promise<void> {
    const keys = await Key.unrevoked(call, machine, machineId);
    await call.mutate(
        call.self,
        keys.map((row) => key.call(name, key.target(row))),
    );
}
