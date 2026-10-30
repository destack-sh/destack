import type { Environment } from "@destack/environment";
import { testSimulation } from "@destack/object/test";
import { Snapshot } from "@destack/db";
import { IdentityOperation, KeyPair } from "@destack/identity";
import { RequestId } from "@destack/sync";
import { type Digest, present } from "@destack/schema";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { Identity } from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { RECOVERY_MILLISECONDS } from "../src/server/index.ts";
import { TestAccountStore, OWNER_EMAIL } from "../src/test/index.ts";
import type { TestCaller } from "@destack/object/test";

/** Serve identities through the account package, a space placed on a machine that starts them. */
async function openRegistry(world: Environment) {
    // place a space on a machine a person enrolls, and enroll another machine serving nothing of it
    const accounts = await TestAccountStore.open();
    const { clock, environment } = accounts;
    onTestFinished(() => accounts[Symbol.asyncDispose]());
    const owner = await accounts.signIn(OWNER_EMAIL);
    const { spaceId: space, machine: serving } = await accounts.createSpace(owner);
    const stranger = accounts.as(await accounts.enroll(owner.account, owner));

    return {
        environment: world,
        clock,
        simulation: accounts.simulation,
        space,
        stranger,
        /** Start the space's identity as the serving machine, or as another caller. */
        start: (signed: string, as: TestCaller = serving) =>
            as.client(accountPackage).identity.create({
                id: space,
                requestId: RequestId.create(environment),
                operation: signed,
            }),
        /** Append an operation to the space's identity as the serving machine. */
        extend: (signed: string) =>
            serving.client(accountPackage).operation.create({
                parentId: space,
                requestId: RequestId.create(environment),
                operation: signed,
            }),
        /** Read the space's current identity. */
        read: async () =>
            Identity.current(await Identity.log(Snapshot.live(accounts.database), space)),
        /** Read the space's log as each operation and whether it was nullified. */
        log: async () =>
            (await Identity.log(Snapshot.live(accounts.database), space)).map((entry) => [
                entry.operation,
                entry.isNullified,
            ]),
    };
}

test("start a space's identity from its machine, chain operations by rotation keys, and let a higher-priority key nullify within the recovery window", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    const registry = await openRegistry(world);
    const { clock, space, start, extend, read, log } = registry;
    const signing = (await KeyPair.generate(world.crypto)).pair;
    const machineKey = (await KeyPair.generate(world.crypto)).pair;
    const owner = (await KeyPair.generate(world.crypto)).pair;
    const intruder = (await KeyPair.generate(world.crypto)).pair;

    // refuse a malformed operation
    expect(await refusal(start("not.an.operation"))).toEqual([
        "BAD_REQUEST",
        "OPERATION_INVALID",
        "the operation is not an identity operation",
    ]);

    // start the identity only from the serving machine
    const first = await IdentityOperation.sign(
        {
            subject: space,
            previous: null,
            signingKeys: [signing.publicKey],
            rotationKeys: [machineKey.publicKey],
        },
        machineKey.privateKey,
        world.crypto,
    );
    expect(await refusal(start(first, registry.stranger))).toEqual([
        "FORBIDDEN",
        "IDENTITY_START_FOREIGN",
        `only the machine serving ${space} can start its identity`,
    ]);
    await start(first);
    expect(await read()).toEqual({
        signingKeys: [signing.publicKey],
        rotationKeys: [machineKey.publicKey],
        digest: await IdentityOperation.digest(first, world.crypto),
    });

    // add the owner's key above the machine's, signed by a rotation key alone
    const previous = await IdentityOperation.digest(first, world.crypto);
    const fields = { subject: space, previous, signingKeys: [signing.publicKey] };
    const rotationKeys = [owner.publicKey, machineKey.publicKey];
    expect(
        await refusal(
            extend(
                await IdentityOperation.sign(
                    { ...fields, rotationKeys },
                    intruder.privateKey,
                    world.crypto,
                ),
            ),
        ),
    ).toEqual([
        "UNAUTHORIZED",
        "OPERATION_UNSIGNED",
        "the operation is not signed by a rotation key of the identity",
    ]);
    const second = await IdentityOperation.sign(
        { ...fields, rotationKeys },
        machineKey.privateKey,
        world.crypto,
    );
    await extend(second);

    // let the owner's key nullify the machine's stolen signing key, and not the reverse
    const unrecoverable = [
        "CONFLICT",
        "OPERATIONS_UNRECOVERABLE",
        `the operation cannot nullify the later operations of ${space}`,
    ];
    const after = {
        subject: space,
        previous: await IdentityOperation.digest(second, world.crypto),
        rotationKeys,
    };
    const stolen = await IdentityOperation.sign(
        { ...after, signingKeys: [intruder.publicKey] },
        machineKey.privateKey,
        world.crypto,
    );
    await extend(stolen);
    expect(present(await read(), "the stolen identity").signingKeys).toEqual([intruder.publicKey]);
    const recovered = await IdentityOperation.sign(
        { ...after, signingKeys: [signing.publicKey] },
        owner.privateKey,
        world.crypto,
    );
    await extend(recovered);
    expect(
        await refusal(
            extend(
                await IdentityOperation.sign(
                    { ...after, signingKeys: [intruder.publicKey] },
                    machineKey.privateKey,
                    world.crypto,
                ),
            ),
        ),
    ).toEqual(unrecoverable);

    // expect the whole log, the stolen operation nullified
    expect([await read(), await log()]).toEqual([
        {
            signingKeys: [signing.publicKey],
            rotationKeys,
            digest: await IdentityOperation.digest(recovered, world.crypto),
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
        previous: await IdentityOperation.digest(recovered, world.crypto),
        rotationKeys,
    };
    await extend(
        await IdentityOperation.sign(
            { ...settled, signingKeys: [intruder.publicKey] },
            machineKey.privateKey,
            world.crypto,
        ),
    );
    await registry.simulation.jump(clock.now() + RECOVERY_MILLISECONDS + 1000);
    expect(
        await refusal(
            extend(
                await IdentityOperation.sign(
                    { ...settled, signingKeys: [signing.publicKey] },
                    owner.privateKey,
                    world.crypto,
                ),
            ),
        ),
    ).toEqual(unrecoverable);
});

test("append the first of two operations racing past the same operation, and refuse the second as a later operation", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    // start a space's identity from its machine
    const registry = await openRegistry(world);
    const { space, start, extend, log } = registry;
    const [machineKey, left, right] = [
        (await KeyPair.generate(world.crypto)).pair,
        (await KeyPair.generate(world.crypto)).pair,
        (await KeyPair.generate(world.crypto)).pair,
    ];
    const sign = (previous: Digest | null, signing: typeof left) =>
        IdentityOperation.sign(
            {
                subject: space,
                previous,
                signingKeys: [signing.publicKey],
                rotationKeys: [machineKey.publicKey],
            },
            machineKey.privateKey,
            world.crypto,
        );
    const first = await sign(null, left);
    await start(first);

    // append two operations following the first at once
    const previous = await IdentityOperation.digest(first, world.crypto);
    const racing = [await sign(previous, left), await sign(previous, right)];
    const outcomes = await Promise.all(racing.map((operation) => refusal(extend(operation))));
    const refused = [
        "CONFLICT",
        "OPERATIONS_UNRECOVERABLE",
        `the operation cannot nullify the later operations of ${space}`,
    ];
    // land one of the racing operations, whichever wins, refusing the other
    const landed = outcomes.filter((outcome) => outcome === "done").length;
    const lost = outcomes.filter((outcome) => outcome !== "done");
    expect([landed, lost]).toEqual([1, [refused]]);
    expect((await log()).length).toBe(2);
});
