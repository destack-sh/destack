import { AccessContext, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { Snapshot } from "@destack/db";
import { IdentityOperation, KeyPair } from "@destack/identity";
import { ObjectStore } from "@destack/object/server";
import { testEvents, serversOf } from "@destack/object/test";
import { Package } from "@destack/package";
import { Identifier } from "@destack/schema";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { identity, Identity, machine, operation, placement } from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import {
    RECOVERY_MILLISECONDS,
    implementIdentity,
    implementOperation,
} from "../src/server/index.ts";
import { AccountFixture, outcome, TestDirectory } from "../src/test/index.ts";
import type { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

test("start a space's identity from its machine, chain operations by rotation keys, and let a higher-priority key nullify within the recovery window", async () => {
    const clock = new VirtualClock(Date.now());
    const registry = await openRegistry(clock);
    const { space, start, extend, read, log } = registry;
    const signing = (await KeyPair.generate()).pair;
    const machineKey = (await KeyPair.generate()).pair;
    const owner = (await KeyPair.generate()).pair;
    const intruder = (await KeyPair.generate()).pair;

    // refuse a malformed operation
    expect(await outcome(start("not.an.operation"))).toBe(
        "INVALID_OPERATION: the operation is no identity operation",
    );

    // start the identity only from the serving machine
    const first = await IdentityOperation.sign(
        {
            subject: space,
            previous: null,
            signingKeys: [signing.publicKey],
            rotationKeys: [machineKey.publicKey],
        },
        machineKey.privateKey,
    );
    expect(await outcome(start(first, registry.stranger))).toBe(
        `FORBIDDEN: only the machine serving ${space} starts its identity`,
    );
    await start(first);
    expect(await read()).toEqual({
        signingKeys: [signing.publicKey],
        rotationKeys: [machineKey.publicKey],
        digest: await IdentityOperation.digest(first),
    });

    // add the owner's key above the machine's, signed by a rotation key alone
    const previous = await IdentityOperation.digest(first);
    const fields = { subject: space, previous, signingKeys: [signing.publicKey] };
    const rotationKeys = [owner.publicKey, machineKey.publicKey];
    expect(
        await outcome(
            extend(await IdentityOperation.sign({ ...fields, rotationKeys }, intruder.privateKey)),
        ),
    ).toBe("UNAUTHORIZED: no rotation key of the identity signed the operation");
    const second = await IdentityOperation.sign({ ...fields, rotationKeys }, machineKey.privateKey);
    await extend(second);

    // let the owner's key nullify the machine's stolen signing key, and not the reverse
    const unrecoverable = `CONFLICT: the operation cannot nullify the later operations of ${space}`;
    const after = {
        subject: space,
        previous: await IdentityOperation.digest(second),
        rotationKeys,
    };
    const stolen = await IdentityOperation.sign(
        { ...after, signingKeys: [intruder.publicKey] },
        machineKey.privateKey,
    );
    await extend(stolen);
    expect((await read())?.signingKeys).toEqual([intruder.publicKey]);
    const recovered = await IdentityOperation.sign(
        { ...after, signingKeys: [signing.publicKey] },
        owner.privateKey,
    );
    await extend(recovered);
    expect(
        await outcome(
            extend(
                await IdentityOperation.sign(
                    { ...after, signingKeys: [intruder.publicKey] },
                    machineKey.privateKey,
                ),
            ),
        ),
    ).toBe(unrecoverable);

    // expect the whole log, the stolen operation nullified
    expect([await read(), await log()]).toEqual([
        {
            signingKeys: [signing.publicKey],
            rotationKeys,
            digest: await IdentityOperation.digest(recovered),
        },
        [
            [first, false],
            [second, false],
            [stolen, true],
            [recovered, false],
        ],
    ]);

    // refuse the owner's key nullifying the machine's next operation once the recovery window passed
    const settled = {
        subject: space,
        previous: await IdentityOperation.digest(recovered),
        rotationKeys,
    };
    await extend(
        await IdentityOperation.sign(
            { ...settled, signingKeys: [intruder.publicKey] },
            machineKey.privateKey,
        ),
    );
    clock.advance(RECOVERY_MILLISECONDS + 1000);
    expect(
        await outcome(
            extend(
                await IdentityOperation.sign(
                    { ...settled, signingKeys: [signing.publicKey] },
                    owner.privateKey,
                ),
            ),
        ),
    ).toBe(unrecoverable);
});

test("append the first of two operations racing past the same operation, and refuse the second as a later operation", async () => {
    // start a space's identity from its machine
    const registry = await openRegistry(new VirtualClock(Date.now()));
    const { space, start, extend, log } = registry;
    const [machineKey, left, right] = [
        (await KeyPair.generate()).pair,
        (await KeyPair.generate()).pair,
        (await KeyPair.generate()).pair,
    ];
    const sign = (previous: string | null, signing: typeof left) =>
        IdentityOperation.sign(
            {
                subject: space,
                previous,
                signingKeys: [signing.publicKey],
                rotationKeys: [machineKey.publicKey],
            },
            machineKey.privateKey,
        );
    const first = await sign(null, left);
    await start(first);

    // append two operations following the first at once
    const previous = await IdentityOperation.digest(first);
    const racing = [await sign(previous, left), await sign(previous, right)];
    const outcomes = await Promise.all(racing.map((each) => outcome(extend(each))));
    expect([outcomes.toSorted(), (await log()).length]).toEqual([
        [`CONFLICT: the operation cannot nullify the later operations of ${space}`, "executed"],
        2,
    ]);
});

/** Serve identities over an account database, a space placed on a machine that starts them. */
async function openRegistry(clock: Clock) {
    // place a space on a machine, which may start and extend identities, as a stranger may extend
    const storage = await AccountFixture.storage();
    onTestFinished(() => storage.close());
    const { database } = storage;
    const space = Identifier.create("space");
    const serving = principal.machine.reference(Scope.universe.id, Identifier.create("machine"));
    const stranger = principal.machine.reference(Scope.universe.id, Identifier.create("machine"));
    await TestDirectory.place(database, {
        id: space,
        scope: Identifier.create("account"),
        machine: machine.identifier(serving.id),
        epoch: 1,
    });

    // let every machine start and extend identities, as the universe's machine role does
    const access = new AccessFixture(database);
    for (const subject of [serving, stranger]) {
        await TestDirectory.publish(
            database,
            machine.identifier(subject.id),
            Scope.universe.id,
            null,
        );
    }
    await access.replicateRelationship(
        Scope.universe.id,
        Scope.universe,
        "machine",
        principal.machine.reference("*", "*"),
        clock.now(),
    );
    await access.replicateRole(
        Scope.universe,
        {
            name: "machine",
            description: "Starts and extends identities",
            permissions: [
                identity.permission("read"),
                identity.permission("start"),
                identity.permission("extend"),
            ],
        },
        { ...Scope.universe, relation: "machine" },
        clock.now(),
    );

    // serve the identities and their operations as the universe
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: Scope.universe,
        servers: serversOf([implementIdentity(), implementOperation()]),
        policies: [placement, machine],
        database,
        callKey: testCallKey,
        events: await testEvents(database, [], clock),
        origin: { package: Package.of(accountPackage) },
    });
    const call = (as: Subject, calls: Parameters<ObjectStore["mutate"]>[1]["calls"]) =>
        store.mutate(AccessContext.of(Caller.of(as), Date.now()), {
            id: RequestId.create(clock.now()),
            calls,
        });

    return {
        space,
        stranger,
        /** Start the space's identity as the serving machine, or as another caller. */
        start: (signed: string, as: Subject = serving) =>
            call(as, [
                identity.call("create", {
                    ...identity.target({ scope: Scope.universe.id, id: space }),
                    operation: signed,
                }),
            ]),
        /** Append an operation to the space's identity as the serving machine. */
        extend: (signed: string) =>
            call(serving, [
                operation.call("create", {
                    ...operation.in(Scope.universe.id),
                    parentId: space,
                    operation: signed,
                }),
            ]),
        /** Read the space's current identity. */
        read: async () => Identity.current(await Identity.log(Snapshot.live(database), space)),
        /** Read the space's log as each operation and whether it was nullified. */
        log: async () =>
            (await Identity.log(Snapshot.live(database), space)).map((entry) => [
                entry.operation,
                entry.isNullified,
            ]),
    };
}
