import { eq, Snapshot, Transaction } from "@destack/db";
import { aligned, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { MemoryKeychain } from "@destack/identity";
import { expect, test } from "@destack/test";
import { MachineIdentity } from "../src/client/index.ts";
import { account, key, Key, machine } from "../src/object/index.ts";
import { AccountFixture, type Person, outcome } from "../src/test/index.ts";

/** A year and a day, past every key's lifetime, in milliseconds. */
const PAST_LIFETIME = 366 * 24 * 60 * 60 * 1000;

test("rotate a machine's key, revoking its other keys, and match the keys authenticating at a time", async () => {
    // enroll a machine and register a new key
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const enrolled = await fixture.enroll(accountId, { client: owner.client });
    const rotated = new MachineIdentity(enrolled.machineId, new MemoryKeychain(), Clock.system);
    await fixture.machine(enrolled).key.create({
        scope: accountId,
        requestId: RequestId.create(Date.now()),
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
        ).map((held) => held.id);
    const now = Date.now();
    const selected = [await active(now), await active(now + PAST_LIFETIME)];
    await owner.client.machine.revoke({
        scope: accountId,
        id: enrolled.machineId,
        requestId: RequestId.create(Date.now()),
    });
    const second = aligned(keys, 1);
    expect({
        revoked: keys.map((entry) => entry.revokedAt !== null),
        selected: [...selected, await active(now)],
    }).toEqual({ revoked: [true, false], selected: [[second.id], [], []] });
});

test("enroll machines proposing one name under its first free numbered variants, and under the name of a revoked one", async () => {
    // enroll three machines proposing the same name in one account
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const preferred = "laptop";
    const names: string[] = [];
    const enroll = (name: string) => fixture.enroll(accountId, { client: owner.client, name });
    for (let index = 0; index < 3; index++) {
        names.push(await nameOf(owner, accountId, await enroll(preferred)));
    }

    // take the name of a revoked machine instead of a numbered variant
    const revoked = await enroll("desktop");
    await owner.client.machine.revoke({
        scope: accountId,
        id: revoked.machineId,
        requestId: RequestId.create(Date.now()),
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
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const tenant = await fixture.person("tenant@example.com");
    const stranger = await fixture.person("stranger@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const rival = await fixture.createSpace(tenant);
    const enrolled = await fixture.enroll(accountId, { client: owner.client });
    const target = machine.reference(accountId, enrolled.machineId);
    const tenants = { ...account.reference(tenant.id, rival.accountId), relation: "member" };
    await Transaction.run(fixture.storage.database, async (transaction) => {
        const caller = fixture.objects.self;
        const universe = await fixture.objects.authorize(transaction, accountId, caller);
        await universe.keepRelationships(
            {
                scope: accountId,
                objects: [machine.typeReference],
                relation: "tenant",
                subject: tenants,
            },
            [{ object: target, relation: "tenant", subject: tenants }],
        );
    });
    const change = (person: Person, name: "drain" | "enable") =>
        outcome(
            person.client.machine[name]({
                scope: accountId,
                id: enrolled.machineId,
                requestId: RequestId.create(Date.now()),
            }),
        );

    // read, drain and enable the machine as the tenant, and refuse renaming it
    const read = await outcome(
        tenant.client.machine.get({ scope: accountId, id: enrolled.machineId }),
    );
    const keys = await outcome(tenant.client.key.list({ scope: accountId }));
    const drained = await change(tenant, "drain");
    const status = (await owner.client.machine.get({ scope: accountId, id: enrolled.machineId }))
        .status;
    const enabled = await change(tenant, "enable");
    const renamed = await outcome(
        tenant.client.machine.rename({
            scope: accountId,
            id: enrolled.machineId,
            requestId: RequestId.create(Date.now()),
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
        "executed",
        "executed",
        "executed",
        "draining",
        "executed",
        "FORBIDDEN: permission denied: rename",
        `NOT_FOUND: no scope ${accountId}`,
    ]);
});

test("record a machine's contact with the version and runtimes it reports", async () => {
    // report the version and runtimes as the machine
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const enrolled = await fixture.enroll(accountId, { client: owner.client });
    const machines = fixture.machine(enrolled);
    const before = Date.now();
    await machines.machine.report({
        scope: accountId,
        id: enrolled.machineId,
        requestId: RequestId.create(Date.now()),
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
    });
    const after = Date.now();
    const seen = await machines.machine.get({ scope: accountId, id: enrolled.machineId });

    // keep what the machine reported
    expect({
        version: seen.version,
        runtimes: seen.runtimes,
        isSeenWithin:
            seen.lastSeenAt !== null && seen.lastSeenAt >= before && seen.lastSeenAt <= after,
    }).toEqual({ version: "2026.9.1", runtimes: ["bun", "workerd"], isSeenWithin: true });
});

/** Read the name an enrolled machine took in its account. */
async function nameOf(
    owner: Person,
    accountId: string,
    enrolled: MachineIdentity,
): Promise<string> {
    return (await owner.client.machine.get({ scope: accountId, id: enrolled.machineId })).name;
}
