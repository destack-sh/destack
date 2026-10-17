import { TEST_DIALECTS } from "@destack/db/test";
import { principal } from "@destack/access";
import { ObjectReference, Scope } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import { afterAll, afterEach, beforeAll, beforeEach, expect, test, vi } from "@destack/test";
import { found } from "@destack/schema";
import { outcome, TestEntitlement } from "../src/test/index.ts";
import { DomainFixture, NOW } from "./domain.ts";

/** The account database of each dialect for the accounts of the scenarios. */
let databases: Awaited<ReturnType<typeof DomainFixture.databases>>;

beforeAll(async () => {
    databases = await DomainFixture.databases();
});

afterAll(async () => {
    for (const database of databases.values()) {
        await database.close();
    }
});

beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"], now: NOW });
});

afterEach(() => {
    vi.useRealTimers();
});

/** Claim a domain, refuse its verification until its TXT record exists, then verify it once. */
test.for(TEST_DIALECTS)(
    "claim and verify a domain once through its TXT record on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(found(databases, dialect).database);
        const { accountId, name } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const verify = (id: string) =>
            owner.domain.verify({ scope: accountId, id, requestId: RequestId.create(Date.now()) });

        // claim the domain for the account on a plan with custom domains
        await TestEntitlement.set(fixture.database, accountId, "domain", { kind: "boolean" });
        const claimed = await owner.domain.create({
            scope: accountId,
            requestId: RequestId.create(Date.now()),
            name,
        });
        const creator = principal.user.reference(Scope.universe.id, fixture.ownerId);
        const record = {
            id: claimed.id,
            scope: accountId,
            name,
            verifiedAt: null,
            revision: 1,
            tags: {},
            createdAt: NOW,
            ...ObjectReference.columns("createdBy", creator),
            updatedAt: NOW,
            ...ObjectReference.columns("updatedBy", creator),
        };
        expect(claimed).toEqual(record);

        // refuse verification before the challenge record exists
        const challenge = `_destack.${name}`;
        expect(await outcome(verify(claimed.id))).toBe(
            `PRECONDITION_FAILED: publish a TXT record ${challenge} with destack-verify=${accountId} first`,
        );

        // verify once the record exists and refuse a second verification
        fixture.publish(claimed);
        const verified = { ...record, verifiedAt: NOW, revision: 2 };
        expect(await verify(claimed.id)).toEqual(verified);
        expect(await outcome(verify(claimed.id))).toBe(`CONFLICT: ${name} is verified`);
        expect(await owner.domain.get({ scope: accountId, id: claimed.id })).toEqual(verified);

        // read only the challenge name each time
        expect(fixture.lookups).toEqual([challenge, challenge]);
    },
);

/** Refuse IP literals and wildcards as domain claims. */
test.for(TEST_DIALECTS)(
    "refuse IP literals and wildcards as domain claims on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(found(databases, dialect).database);
        const { accountId } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const claim = (name: string) =>
            owner.domain.create({
                scope: accountId,
                requestId: RequestId.create(Date.now()),
                name,
            });

        // refuse IP literals and wildcards
        const malformed =
            "BAD_REQUEST: invalid input: name: hostnames are lowercase ASCII labels, punycode for others";
        expect(await outcome(claim("*.acme.com"))).toBe(malformed);
        expect(await outcome(claim("192.168.0.1"))).toBe(
            "BAD_REQUEST: invalid input: name: top-level labels have a letter",
        );
        expect(await outcome(claim("[::1]"))).toBe(
            "BAD_REQUEST: invalid input: name: hostnames are lowercase ASCII labels, punycode for others; name: top-level labels have a letter",
        );
        expect(await owner.domain.list({ scope: accountId })).toEqual({ items: [], cursor: null });
    },
);

/** Let several accounts claim one domain, and let only the first to prove it verify it. */
test.for(TEST_DIALECTS)(
    "let a second account claim a domain and refuse its verification once another verified it on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(found(databases, dialect).database);
        const { accountId, otherId, name } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const rival = fixture.client(fixture.rivalId);

        // claim the domain in both accounts on plans with custom domains
        await TestEntitlement.set(fixture.database, accountId, "domain", { kind: "boolean" });
        await TestEntitlement.set(fixture.database, otherId, "domain", { kind: "boolean" });
        const first = await owner.domain.create({
            scope: accountId,
            requestId: RequestId.create(Date.now()),
            name,
        });
        const second = await rival.domain.create({
            scope: otherId,
            requestId: RequestId.create(Date.now()),
            name,
        });

        // publish both records and verify the first
        fixture.publish(first);
        fixture.publish(second);
        await owner.domain.verify({
            scope: accountId,
            id: first.id,
            requestId: RequestId.create(Date.now()),
        });

        // refuse the second although its record exists too
        expect(
            await outcome(
                rival.domain.verify({
                    scope: otherId,
                    id: second.id,
                    requestId: RequestId.create(Date.now()),
                }),
            ),
        ).toBe(`CONFLICT: another owner verified ${name}`);
        expect(await rival.domain.get({ scope: otherId, id: second.id })).toEqual(second);
    },
);

/** Serve hostnames below an account's verified domain, refusing its removal while one serves. */
test.for(TEST_DIALECTS)(
    "serve hostnames below a verified domain alone, and refuse to remove the domain while one serves on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(found(databases, dialect).database);
        const { accountId, otherId, name } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const rival = fixture.client(fixture.rivalId);
        const notes = `notes.${name}`;

        // refuse a hostname before its domain is verified
        await TestEntitlement.set(fixture.database, accountId, "domain", { kind: "boolean" });
        const claimed = await owner.domain.create({
            scope: accountId,
            requestId: RequestId.create(Date.now()),
            name,
        });
        expect(await outcome(serve(owner, accountId, notes))).toBe(
            `PRECONDITION_FAILED: verify a domain of ${notes} first`,
        );
        fixture.publish(claimed);
        await owner.domain.verify({
            scope: accountId,
            id: claimed.id,
            requestId: RequestId.create(Date.now()),
        });
        const served = await serve(owner, accountId, notes);
        await serve(owner, accountId, name);
        await serve(owner, accountId, `*.${name}`);

        // refuse the rival a hostname below the domain
        expect(await outcome(serve(rival, otherId, `blog.${name}`))).toBe(
            `PRECONDITION_FAILED: verify a domain of blog.${name} first`,
        );
        const remove = () =>
            owner.domain.delete({
                scope: accountId,
                id: claimed.id,
                requestId: RequestId.create(Date.now()),
            });
        const covered = [name, notes, `*.${name}`].toSorted().join(", ");
        expect(await outcome(remove())).toBe(
            `PRECONDITION_FAILED: hostnames serve below ${name}: ${covered}`,
        );
        expect(
            (await owner.hostname.list({ accountId })).items.map((entry) => entry.name).toSorted(),
        ).toEqual([name, notes, `*.${name}`].toSorted());
        expect(served.name).toBe(notes);
    },
);

/** Refuse every domain method to a user outside the account. */
test.for(TEST_DIALECTS)("refuse domain methods to a non-member on %s", async (dialect) => {
    await using fixture = await DomainFixture.open(found(databases, dialect).database);
    const { accountId, name } = fixture;
    const owner = fixture.client(fixture.ownerId);
    const stranger = fixture.client(fixture.strangerId);
    await TestEntitlement.set(fixture.database, accountId, "domain", { kind: "boolean" });
    const claimed = await owner.domain.create({
        scope: accountId,
        requestId: RequestId.create(Date.now()),
        name,
    });
    fixture.publish(claimed);

    // hide the account from a stranger's listing, reads, claims, verifications and deletions
    const id = claimed.id;
    const scope = accountId;
    const hidden = `NOT_FOUND: no scope ${accountId}`;
    expect(await outcome(stranger.domain.list({ scope }))).toBe(hidden);
    expect(await outcome(stranger.domain.get({ scope, id }))).toBe(hidden);
    expect(
        await outcome(
            stranger.domain.create({ scope, requestId: RequestId.create(Date.now()), name }),
        ),
    ).toBe(hidden);
    expect(
        await outcome(
            stranger.domain.verify({ scope, id, requestId: RequestId.create(Date.now()) }),
        ),
    ).toBe(hidden);
    expect(
        await outcome(
            stranger.domain.delete({ scope, id, requestId: RequestId.create(Date.now()) }),
        ),
    ).toBe(hidden);

    // leave the claim unverified and ask the resolver nothing
    expect(await owner.domain.get({ scope, id })).toEqual(claimed);
    expect(fixture.lookups).toEqual([]);
});

/** Serve a hostname in an account as a client of the account package. */
function serve(client: ReturnType<DomainFixture["client"]>, accountId: string, name: string) {
    return client.hostname.create({ accountId, requestId: RequestId.create(Date.now()), name });
}

/** Refuse a domain claim in an account whose plan allows no custom domains, then admit it. */
test.for(TEST_DIALECTS)(
    "refuse a domain claim the account's plan allows no custom domains on %s",
    async (dialect) => {
        // serve the domains under the account's entitlements
        const { database } = found(databases, dialect);
        await using fixture = await DomainFixture.open(database);
        const owner = fixture.client(fixture.ownerId);
        const claim = () =>
            owner.domain.create({
                scope: fixture.accountId,
                requestId: RequestId.create(Date.now()),
                name: fixture.name,
            });

        // refuse the claim, then admit it on a plan with custom domains
        expect(await outcome(claim())).toBe(
            `QUOTA_EXCEEDED: the plan of ${fixture.accountId} allows no custom domains`,
        );
        await TestEntitlement.set(database, fixture.accountId, "domain", { kind: "boolean" });
        expect((await claim()).name).toBe(fixture.name);
    },
);
