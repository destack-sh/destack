import accountPackage from "../src/package.ts";
import objectPackage from "@destack/object/package";
import { AccessContext, Caller } from "@destack/access";
import { eq, Snapshot } from "@destack/db";
import { present } from "@destack/schema";
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
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const enrolled = await fixture.enroll(accountId, owner);
    const rotated = new MachineIdentity(
        enrolled.machineId,
        new MemoryKeychain(),
        fixture.store.environment,
    );
    const { id: rotatedId } = await fixture
        .as(enrolled)
        .client(accountPackage)
        .key.create({
            scope: accountId,
            requestId: RequestId.create(fixture.store.environment),
            parent: { ...machine.typeReference, id: enrolled.machineId },
            ...(await rotated.generate()),
        });

    // revoke the first key, keyed by identifier as both share their creation time
    const { database } = fixture.storage;
    const keys = await database
        .select({ id: key.table.id, revokedAt: key.table.revokedAt })
        .from(key.table)
        .where(eq(key.table.parentId, enrolled.machineId))
        .orderBy(key.table.createdAt, key.table.id);
    const first = present(
        keys.find((row) => row.id !== rotatedId),
        "the enrolled key",
    );
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
        requestId: RequestId.create(fixture.store.environment),
    });
    const revocations = Object.fromEntries(keys.map((row) => [row.id, row.revokedAt]));
    expect({ revocations, selected: [...selected, await active(now)] }).toEqual({
        revocations: { [first.id]: now, [rotatedId]: null },
        selected: [[rotatedId], [], []],
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
        requestId: RequestId.create(fixture.store.environment),
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
        requestId: RequestId.create(fixture.store.environment),
        object: target,
        relation: "tenant",
        subject: tenants,
    });
    const change = (person: TestPerson, name: "drain" | "enable") =>
        refusal(
            person.client(accountPackage).machine[name]({
                scope: accountId,
                id: enrolled.machineId,
                requestId: RequestId.create(fixture.store.environment),
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
            requestId: RequestId.create(fixture.store.environment),
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

test("record a machine's contact with what it reports, replacing its host, and refuse a report without one", async () => {
    // report the version, runtimes, host, system, start and update state as the machine at a clock standing still
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const enrolled = await fixture.enroll(accountId, owner);
    const machines = fixture.as(enrolled).client(accountPackage);
    const before = await machines.machine.get({ scope: accountId, id: enrolled.machineId });
    await machines.machine.report({
        scope: accountId,
        id: enrolled.machineId,
        requestId: RequestId.create(fixture.store.environment),
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
        osVersion: "25.1.0",
        startedAt: 1_000,
        update: { installed: "2026.9.1", staged: "2026.9.2", status: "staged" },
        host: { platform: "Linux", architecture: "x64" },
    });
    const seen = await machines.machine.get({ scope: accountId, id: enrolled.machineId });
    const hostless = await refusal(
        machines.machine.report({
            scope: accountId,
            id: enrolled.machineId,
            requestId: RequestId.create(fixture.store.environment),
            version: "2026.9.1",
            runtimes: [],
            osVersion: null,
            startedAt: null,
            update: null,
            host: null,
        }),
    );

    // store what the machine reported, seen at the store's time, and refuse a personal machine reporting no host
    expect(hostless).toEqual([
        "BAD_REQUEST",
        "HOST_REQUIRED",
        "a personal machine reports its host",
    ]);
    expect(seen).toEqual({
        ...before,
        ...ObjectReference.columns("updatedBy", fixture.as(enrolled).subject),
        revision: before.revision + 1,
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
        osVersion: "25.1.0",
        startedAt: 1_000,
        update: { installed: "2026.9.1", staged: "2026.9.2", status: "staged" },
        platform: "Linux",
        architecture: "x64",
        lastSeenAt: fixture.clock.now(),
    });
});

test("refuse a machine sharing, unmasking or relaying its own row, which no role of its own covers", async () => {
    // enroll a machine of the owner's account, and a tenant account to share it with
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const tenant = await fixture.signIn("partner@example.com");
    const { id: accountId } = await fixture.createAccount(owner);
    const rival = await fixture.createAccount(tenant);
    const enrolled = await fixture.enroll(accountId, owner);
    const self = fixture.as(enrolled);
    const target = machine.reference(accountId, enrolled.machineId);

    // share it, relay it and unmask its telemetry as the machine itself
    const shared = await refusal(
        self.client(objectPackage).relationship.create({
            scope: accountId,
            requestId: RequestId.create(fixture.store.environment),
            object: target,
            relation: "tenant",
            subject: { ...account.reference(tenant.id, rival.id), relation: "member" },
        }),
    );
    const relayed = await refusal(
        self.client(accountPackage).machine.relay({
            scope: accountId,
            id: enrolled.machineId,
            url: "https://eu.relay.test",
            lapsesAt: fixture.store.clock.now() + 60_000,
            requestId: RequestId.create(fixture.store.environment),
        }),
    );
    const { authorizer } = fixture.store;
    const snapshot = Snapshot.live(fixture.database);
    const access = await authorizer.resolve(
        snapshot,
        accountId,
        AccessContext.of(Caller.of(self.subject), fixture.store.clock.now()),
    );
    const unmasked = await authorizer.check(snapshot, machine.permission("unmask"), target, access);
    expect({ shared, relayed, unmasked }).toEqual({
        shared: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"],
        relayed: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: relay"],
        unmasked: { isAllowed: false },
    });
});

test("refuse a cloud machine reporting a host, and its residency's relay publishing a tunnel for it", async () => {
    // declare a dedicated cloud machine of the owner's account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const cloud = await fixture.declareCloudMachine(accountId, "eu");
    const target = { scope: accountId, id: cloud.machineId };

    // report a host as the cloud machine, and relay its tunnel as the residency's relay
    const reported = await refusal(
        fixture
            .as(cloud)
            .client(accountPackage)
            .machine.report({
                ...target,
                requestId: RequestId.create(fixture.store.environment),
                version: "2026.9.1",
                runtimes: ["workerd"],
                osVersion: null,
                startedAt: null,
                update: null,
                host: { platform: "Linux", architecture: "x64" },
            }),
    );
    const relaying = fixture.as(await fixture.relay("eu"));
    const relayed = await refusal(
        relaying.client(accountPackage).machine.relay({
            ...target,
            url: "https://eu.relay.test",
            lapsesAt: fixture.clock.now() + 60_000,
            requestId: RequestId.create(fixture.store.environment),
        }),
    );

    expect([reported, relayed]).toEqual([
        ["BAD_REQUEST", "CLOUD_HOST_REPORTED", "a cloud machine reports no host"],
        [
            "FORBIDDEN",
            "MACHINE_CLOUD",
            `machine ${cloud.machineId} is a cloud machine: it answers at its URL`,
        ],
    ]);
});
