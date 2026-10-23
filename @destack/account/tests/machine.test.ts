import accountPackage from "../src/package.ts";
import objectPackage from "@destack/object/package";
import { eq, Snapshot } from "@destack/db";
import { nth } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { MemoryKeychain } from "@destack/identity";
import { expect, test, refusal } from "@destack/test";
import { MachineIdentity } from "../src/client/index.ts";
import { account, key, Key, machine } from "../src/object/index.ts";
import {
    TestAccountStore,
    type TestPerson,
    OWNER_EMAIL,
    STRANGER_EMAIL,
} from "../src/test/index.ts";

/** A year and a day, past every key's lifetime, in milliseconds. */
const PAST_LIFETIME = 366 * 24 * 60 * 60 * 1000;

/** Read the name an enrolled machine took in its account. */
async function nameOf(
    owner: TestPerson,
    accountId: string,
    enrolled: MachineIdentity,
): Promise<string> {
    return (
        await owner.client(accountPackage).machine.get({ scope: accountId, id: enrolled.machineId })
    ).name;
}

test("rotate a machine's key, revoking its other keys, and match the keys authenticating at a time", async () => {
    // enroll a machine and register a new key at a clock standing still
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const enrolled = await fixture.enroll(accountId, owner);
    const rotated = new MachineIdentity(
        enrolled.machineId,
        new MemoryKeychain(),
        fixture.store.clock,
    );
    await fixture
        .as(enrolled)
        .client(accountPackage)
        .key.create({
            scope: accountId,
            requestId: RequestId.create(fixture.store.clock.now()),
            parent: { ...machine.typeReference, id: enrolled.machineId },
            ...(await rotated.generate()),
        });

    // revoke the first key
    const { database } = fixture.storage;
    const keys = await database
        .select({ id: key.table.id, revokedAt: key.table.revokedAt })
        .from(key.table)
        .where(eq(key.table.parentId, enrolled.machineId))
        .orderBy(key.table.createdAt);
    const active = async (now: number) =>
        (
            await Snapshot.live(database)
                .read(key.table)
                .findMany({
                    where: {
                        AND: [{ parentId: enrolled.machineId }, Key.authenticates(now)],
                    },
                })
        ).map((row) => row.id);
    const now = fixture.store.clock.now();
    const selected = [await active(now), await active(now + PAST_LIFETIME)];
    await owner.client(accountPackage).machine.revoke({
        scope: accountId,
        id: enrolled.machineId,
        requestId: RequestId.create(fixture.store.clock.now()),
    });
    const [first, second] = [nth(keys, 0), nth(keys, 1)];
    expect({ keys, selected: [...selected, await active(now)] }).toEqual({
        keys: [
            { id: first.id, revokedAt: now },
            { id: second.id, revokedAt: null },
        ],
        selected: [[second.id], [], []],
    });
});

test("enroll machines proposing one name under its first free numbered variants, and under the name of a revoked one", async () => {
    // enroll three machines proposing the same name in one account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const preferred = "laptop";
    const names: string[] = [];
    const enroll = (name: string) => fixture.enroll(accountId, owner, { name });
    for (let index = 0; index < 3; index++) {
        names.push(await nameOf(owner, accountId, await enroll(preferred)));
    }

    // take the name of a revoked machine instead of a numbered variant
    const revoked = await enroll("desktop");
    await owner.client(accountPackage).machine.revoke({
        scope: accountId,
        id: revoked.machineId,
        requestId: RequestId.create(fixture.store.clock.now()),
    });
    const again = await enroll("desktop");
    expect([...names, await nameOf(owner, accountId, again)]).toEqual([
        preferred,
        `${preferred}-2`,
        `${preferred}-3`,
        "desktop",
    ]);
});

test("let a tenant read, drain and enable a machine and read its keys, and refuse renaming it and a stranger draining it", async () => {
    // make the members of a tenant's account tenants of the owner's machine
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const tenant = await fixture.signIn("partner@example.com");
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const rival = await fixture.createAccount(tenant);
    const enrolled = await fixture.enroll(accountId, owner);
    const target = machine.reference(accountId, enrolled.machineId);
    const tenants = { ...account.reference(tenant.id, rival.id), relation: "member" };
    await owner.client(objectPackage).relationship.create({
        scope: accountId,
        requestId: RequestId.create(fixture.store.clock.now()),
        object: target,
        relation: "tenant",
        subject: tenants,
    });
    const change = (person: TestPerson, name: "drain" | "enable") =>
        refusal(
            person.client(accountPackage).machine[name]({
                scope: accountId,
                id: enrolled.machineId,
                requestId: RequestId.create(fixture.store.clock.now()),
            }),
        );

    // read, drain and enable the machine as the tenant, and refuse renaming it
    const read = await refusal(
        tenant.client(accountPackage).machine.get({ scope: accountId, id: enrolled.machineId }),
    );
    const keys = await refusal(tenant.client(accountPackage).key.list({ scope: accountId }));
    const drained = await change(tenant, "drain");
    const status = (
        await owner.client(accountPackage).machine.get({ scope: accountId, id: enrolled.machineId })
    ).status;
    const enabled = await change(tenant, "enable");
    const renamed = await refusal(
        tenant.client(accountPackage).machine.rename({
            scope: accountId,
            id: enrolled.machineId,
            requestId: RequestId.create(fixture.store.clock.now()),
            name: "taken-over",
        }),
    );

    // refuse a stranger draining it
    expect([
        read,
        keys,
        drained,
        status,
        enabled,
        renamed,
        await change(stranger, "drain"),
    ]).toEqual([
        "done",
        "done",
        "done",
        "draining",
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: rename"],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`],
    ]);
});

test("record a machine's contact with the version and runtimes it reports", async () => {
    // report the version and runtimes as the machine at a clock standing still
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const enrolled = await fixture.enroll(accountId, owner);
    const machines = fixture.as(enrolled).client(accountPackage);
    const before = await machines.machine.get({ scope: accountId, id: enrolled.machineId });
    await machines.machine.report({
        scope: accountId,
        id: enrolled.machineId,
        requestId: RequestId.create(fixture.store.clock.now()),
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
    });
    const seen = await machines.machine.get({ scope: accountId, id: enrolled.machineId });

    // store what the machine reported, seen at the store's time
    expect(seen).toEqual({
        ...before,
        ...ObjectReference.columns("updatedBy", fixture.as(enrolled).subject),
        revision: before.revision + 1,
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
        lastSeenAt: fixture.clock.now(),
    });
});
