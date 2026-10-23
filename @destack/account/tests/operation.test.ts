import { Snapshot } from "@destack/db";
import { IdentityOperation, KeyPair } from "@destack/identity";
import { RequestId } from "@destack/sync";
import { present } from "@destack/schema";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { Identity } from "../src/object/index.ts";
import accountPackage from "../src/package.ts";
import { RECOVERY_MILLISECONDS } from "../src/server/index.ts";
import { TestAccountStore, OWNER_EMAIL } from "../src/test/index.ts";
import type { TestCaller } from "@destack/object/test";

/** Serve identities through the account package, a space placed on a machine that starts them. */
async function openRegistry() {
    // place a space on a machine a person enrolls, and enroll another machine serving nothing of it
    const accounts = await TestAccountStore.open();
    const { clock } = accounts;
    onTestFinished(() => accounts[Symbol.asyncDispose]());
    const owner = await accounts.signIn(OWNER_EMAIL);
    const { spaceId: space, machine: serving } = await accounts.createSpace(owner);
    const stranger = accounts.as(await accounts.enroll(owner.account, owner));

    return {
        clock,
        space,
        stranger,
        /** Start the space's identity as the serving machine, or as another caller. */
        start: (signed: string, as: TestCaller = serving) =>
            as.client(accountPackage).identity.create({
                id: space,
                requestId: RequestId.create(clock.now()),
                operation: signed,
            }),
        /** Append an operation to the space's identity as the serving machine. */
        extend: (signed: string) =>
            serving.client(accountPackage).operation.create({
                parentId: space,
                requestId: RequestId.create(clock.now()),
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
    const registry = await openRegistry();
    const { clock, space, start, extend, read, log } = registry;
    clock.pause();
    const signing = (await KeyPair.generate()).pair;
    const machineKey = (await KeyPair.generate()).pair;
    const owner = (await KeyPair.generate()).pair;
    const intruder = (await KeyPair.generate()).pair;

    // refuse a malformed operation
    expect(await refusal(start("not.an.operation"))).toEqual([
        "BAD_REQUEST",
        "INVALID_OPERATION",
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
    );
    expect(await refusal(start(first, registry.stranger))).toEqual([
        "FORBIDDEN",
        "FOREIGN_IDENTITY_START",
        `only the machine serving ${space} can start its identity`,
    ]);
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
        await refusal(
            extend(await IdentityOperation.sign({ ...fields, rotationKeys }, intruder.privateKey)),
        ),
    ).toEqual([
        "UNAUTHORIZED",
        "UNSIGNED_OPERATION",
        "the operation is not signed by a rotation key of the identity",
    ]);
    const second = await IdentityOperation.sign({ ...fields, rotationKeys }, machineKey.privateKey);
    await extend(second);

    // let the owner's key nullify the machine's stolen signing key, and not the reverse
    const unrecoverable = [
        "CONFLICT",
        "NULLIFY_REFUSED",
        `the operation cannot nullify the later operations of ${space}`,
    ];
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
    expect(present(await read(), "the stolen identity").signingKeys).toEqual([intruder.publicKey]);
    const recovered = await IdentityOperation.sign(
        { ...after, signingKeys: [signing.publicKey] },
        owner.privateKey,
    );
    await extend(recovered);
    expect(
        await refusal(
            extend(
                await IdentityOperation.sign(
                    { ...after, signingKeys: [intruder.publicKey] },
                    machineKey.privateKey,
                ),
            ),
        ),
    ).toEqual(unrecoverable);

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
        await refusal(
            extend(
                await IdentityOperation.sign(
                    { ...settled, signingKeys: [signing.publicKey] },
                    owner.privateKey,
                ),
            ),
        ),
    ).toEqual(unrecoverable);
});

test("append the first of two operations racing past the same operation, and refuse the second as a later operation", async () => {
    // start a space's identity from its machine
    const registry = await openRegistry();
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
    const outcomes = await Promise.all(racing.map((operation) => refusal(extend(operation))));
    const refused = [
        "CONFLICT",
        "NULLIFY_REFUSED",
        `the operation cannot nullify the later operations of ${space}`,
    ];
    expect([
        ["done", refused],
        [refused, "done"],
    ]).toContainEqual(outcomes);
    expect((await log()).length).toBe(2);
});
