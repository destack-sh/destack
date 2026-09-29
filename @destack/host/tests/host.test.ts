import { eq } from "@destack/db";
import { principal } from "@destack/access";
import { account } from "@destack/account/object";
import { Scope } from "@destack/sync";
import { Condition } from "@destack/db/query";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { host, Host, hostKey, HostKey } from "../src/object/index.ts";
import { GlobalFixture, ids } from "../src/test/index.ts";
import { HostIdentity } from "../src/identity/index.ts";
import { MemoryKeychain } from "../src/keychain/index.ts";
import { v7 } from "uuid";

/** A year and a day, past every key's lifetime, in milliseconds. */
const PAST_LIFETIME = 366 * 24 * 60 * 60 * 1000;

/** The global tier every scenario enrolls its own host in. */
let global: GlobalFixture;

beforeAll(async () => {
    global = await GlobalFixture.open();
});

afterAll(async () => {
    await global[Symbol.asyncDispose]();
});

test("find a standing host by its name within its account, and none once revoked", async () => {
    const identity = await global.enroll(ids.account);
    const hostId = identifier("host").parse(identity.hostId);
    const { name } = await global.host(identity).host.get({ accountId: ids.account, id: hostId });

    // find the host by its name in its own account only
    expect([
        await Host.find(global.database, ids.account, name),
        await Host.find(global.database, ids.other, name),
        await Host.find(global.database, ids.account, "missing"),
    ]).toEqual([hostId, undefined, undefined]);

    // find nothing once the owner revoked the host
    await global.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    expect(await Host.find(global.database, ids.account, name)).toBeUndefined();
});

test("rotate a host's key, revoking its other keys, and match the keys authenticating at a time", async () => {
    const identity = await global.enroll(ids.account);
    const hostId = identifier("host").parse(identity.hostId);
    const keys = () =>
        global.database
            .select({ id: hostKey.table.id, revokedAt: hostKey.table.revokedAt })
            .from(hostKey.table)
            .where(eq(hostKey.table.parentId, hostId))
            .orderBy(hostKey.table.createdAt);
    const active = async (now: number) =>
        (
            await HostKey.select(
                global.database,
                Condition.all(Condition.eq("parentId", hostId), HostKey.authenticates(now)),
            )
        ).map((key) => key.id);

    // rotate the key, which revokes the first one
    await identity.rotate(global.host(identity), ids.account);
    const [first, second] = await keys();
    expect([first!.revokedAt !== null, second!.revokedAt !== null]).toEqual([true, false]);

    // select only the second key, and none past its lifetime or once the host is revoked
    const now = Date.now();
    const selected = [await active(now), await active(now + PAST_LIFETIME)];
    await global.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    expect([...selected, await active(now)]).toEqual([[second!.id], [], []]);
});

test("enroll hosts proposing one name under its first free numbered variants", async () => {
    // enroll three hosts proposing the same name in one account
    const preferred = `laptop-${crypto.randomUUID().slice(0, 8)}`;
    const names: string[] = [];
    for (let index = 0; index < 3; index++) {
        const identity = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
        await identity.enroll(global.user(ids.owner), {
            accountId: identifier("account").parse(ids.account),
            requestId: RequestId.create(),
            name: preferred,
            kind: "cloud",
        });
        const hostId = identifier("host").parse(identity.hostId);
        const enrolled = await global
            .host(identity)
            .host.get({ accountId: ids.account, id: hostId });
        names.push(enrolled.name);
    }

    // keep the first, number the others
    expect(names).toEqual([preferred, `${preferred}-2`, `${preferred}-3`]);
});

test("enroll a host under the name of a revoked one", async () => {
    // revoke a host, then enroll another under its name
    const revoked = await global.enroll(ids.account);
    const hostId = identifier("host").parse(revoked.hostId);
    const { name } = await global.host(revoked).host.get({ accountId: ids.account, id: hostId });
    await global.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    const identity = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
    const enrolled = await identity.enroll(global.user(ids.owner), {
        accountId: ids.account,
        requestId: RequestId.create(),
        name,
        kind: "cloud",
    });

    // take the free name instead of a numbered variant
    expect(enrolled.name).toBe(name);
});

test("let a tenant read a host and its keys, and refuse renaming it", async () => {
    // make the stranger a member of the rival account, and the rival a tenant of the host
    const identity = await global.enroll(ids.account);
    const hostId = identifier("host").parse(identity.hostId);
    const rival = account.reference(ids.owner, ids.other);
    await GlobalFixture.relate(
        global.database,
        ids.other,
        rival,
        "member",
        principal.user.reference(Scope.universe.id, ids.stranger),
    );
    await GlobalFixture.relate(
        global.database,
        ids.account,
        host.reference(ids.account, hostId),
        "tenant",
        {
            ...rival,
            relation: "member",
        },
    );

    // read the host and its keys as the tenant, and refuse its rename
    const tenant = global.user(ids.stranger);
    const outcome = (call: Promise<unknown>) =>
        call.then(
            () => "accepted",
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );
    expect([
        await outcome(tenant.host.get({ accountId: ids.account, id: hostId })),
        await outcome(tenant.hostKey.list({ accountId: ids.account })),
        await outcome(
            tenant.host.rename({
                accountId: ids.account,
                id: hostId,
                requestId: RequestId.create(),
                name: "taken-over",
            }),
        ),
    ]).toEqual(["accepted", "accepted", "FORBIDDEN: permission denied: rename"]);
});

test("record a host's contact with the version and runtimes it reports", async () => {
    const identity = await global.enroll(ids.account);
    const hostId = identifier("host").parse(identity.hostId);

    // report the version and runtimes as the host
    const before = Date.now();
    await global.host(identity).host.see({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
    });
    const after = Date.now();
    const seen = await global.host(identity).host.get({ accountId: ids.account, id: hostId });

    // keep what the host reported, seen within the call
    expect({
        version: seen.version,
        runtimes: seen.runtimes,
        isSeenWithin: seen.lastSeenAt! >= before && seen.lastSeenAt! <= after,
    }).toEqual({ version: "2026.9.1", runtimes: ["bun", "workerd"], isSeenWithin: true });
});
