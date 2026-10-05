import { eq } from "@destack/db";
import { principal } from "@destack/access";
import { account } from "@destack/account/object";
import { Scope } from "@destack/sync";
import { aligned, Identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { afterAll, beforeAll, expect, refusal, test } from "@destack/test";
import { host, Host, hostKey, HostKey } from "@destack/account/object";
import { AccountFixture, ids } from "../src/test/index.ts";
import { HostIdentity } from "../src/identity/index.ts";
import { MemoryKeychain } from "../src/keychain/index.ts";

/** A year and a day, past every key's lifetime, in milliseconds. */
const PAST_LIFETIME = 366 * 24 * 60 * 60 * 1000;

/** Report a call's outcome as its failure code and message, or as accepted. */
async function outcome(call: Promise<unknown>): Promise<string> {
    const refused = await refusal(call);

    return refused === "done" ? "accepted" : refused.join(": ");
}

/** The account service every scenario enrolls its host in. */
let accounts: AccountFixture;

beforeAll(async () => {
    accounts = await AccountFixture.open();
});

afterAll(async () => {
    await accounts[Symbol.asyncDispose]();
});

test("find a standing host by its name within its account, and none once revoked", async () => {
    const identity = await accounts.enroll(ids.account);
    const hostId = identity.hostId;
    const { name } = await accounts.host(identity).host.get({ accountId: ids.account, id: hostId });

    // find the host by its name in its account only
    expect([
        await Host.find(accounts.database, ids.account, name),
        await Host.find(accounts.database, ids.other, name),
        await Host.find(accounts.database, ids.account, "missing"),
    ]).toEqual([hostId, undefined, undefined]);

    // find nothing once the owner revoked the host
    await accounts.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    expect(await Host.find(accounts.database, ids.account, name)).toBeUndefined();
});

test("rotate a host's key, revoking its other keys, and match the keys authenticating at a time", async () => {
    const identity = await accounts.enroll(ids.account);
    const hostId = identity.hostId;
    const keys = () =>
        accounts.database
            .select({ id: hostKey.table.id, revokedAt: hostKey.table.revokedAt })
            .from(hostKey.table)
            .where(eq(hostKey.table.parentId, hostId))
            .orderBy(hostKey.table.createdAt);
    const active = async (now: number) =>
        (
            await HostKey.select(accounts.database, {
                AND: [{ parentId: hostId }, HostKey.authenticates(now)],
            })
        ).map((key) => key.id);

    // rotate the key, which revokes the first one
    await identity.rotate(accounts.host(identity), ids.account);
    const rotated = await keys();
    const first = aligned(rotated, 0);
    const second = aligned(rotated, 1);
    expect([first.revokedAt !== null, second.revokedAt !== null]).toEqual([true, false]);

    // select only the second key, and none past its lifetime or once the host is revoked
    const now = Date.now();
    const selected = [await active(now), await active(now + PAST_LIFETIME)];
    await accounts.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    expect([...selected, await active(now)]).toEqual([[second.id], [], []]);
});

test("enroll hosts proposing one name under its first free numbered variants", async () => {
    // enroll three hosts proposing the same name in one account
    const preferred = `laptop-${crypto.randomUUID().slice(0, 8)}`;
    const names: string[] = [];
    for (let index = 0; index < 3; index++) {
        const identity = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
        await identity.enroll(accounts.user(ids.owner), {
            accountId: ids.account,
            requestId: RequestId.create(),
            name: preferred,
            kind: "cloud",
        });
        const hostId = identity.hostId;
        const enrolled = await accounts
            .host(identity)
            .host.get({ accountId: ids.account, id: hostId });
        names.push(enrolled.name);
    }

    // keep the first, number the others
    expect(names).toEqual([preferred, `${preferred}-2`, `${preferred}-3`]);
});

test("enroll a host under the name of a revoked one", async () => {
    // revoke a host, then enroll another under its name
    const revoked = await accounts.enroll(ids.account);
    const hostId = revoked.hostId;
    const { name } = await accounts.host(revoked).host.get({ accountId: ids.account, id: hostId });
    await accounts.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
    });
    const identity = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
    const enrolled = await identity.enroll(accounts.user(ids.owner), {
        accountId: ids.account,
        requestId: RequestId.create(),
        name,
        kind: "cloud",
    });

    // take the free name instead of a numbered variant
    expect(enrolled.name).toBe(name);
});

test("let a tenant read a host and its keys, and refuse renaming it", async () => {
    // make the stranger an editor of the rival account, and the rival's members tenants of the host
    const identity = await accounts.enroll(ids.account);
    const hostId = identity.hostId;
    const rival = account.reference(ids.owner, ids.other);
    await AccountFixture.relate(
        accounts.database,
        ids.other,
        rival,
        "editor",
        principal.user.reference(Scope.universe.id, ids.stranger),
    );
    await AccountFixture.relate(
        accounts.database,
        ids.account,
        host.reference(ids.account, hostId),
        "tenant",
        {
            ...rival,
            relation: "member",
        },
    );

    // read the host and its keys as the tenant, and refuse its rename
    const tenant = accounts.user(ids.stranger);
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
    const identity = await accounts.enroll(ids.account);
    const hostId = identity.hostId;

    // report the version and runtimes as the host
    const before = Date.now();
    await accounts.host(identity).host.report({
        accountId: ids.account,
        id: hostId,
        requestId: RequestId.create(),
        version: "2026.9.1",
        runtimes: ["bun", "workerd"],
    });
    const after = Date.now();
    const seen = await accounts.host(identity).host.get({ accountId: ids.account, id: hostId });

    // keep what the host reported, seen within the call
    expect({
        version: seen.version,
        runtimes: seen.runtimes,
        isSeenWithin:
            seen.lastSeenAt !== null && seen.lastSeenAt >= before && seen.lastSeenAt <= after,
    }).toEqual({ version: "2026.9.1", runtimes: ["bun", "workerd"], isSeenWithin: true });
});
