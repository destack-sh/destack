import { TEST_DIALECTS } from "@destack/db/test";
import { principal } from "@destack/access";
import { Subject, Scope } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { afterAll, afterEach, beforeAll, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { DomainFixture, NOW } from "./domain.ts";

/** The global database of each dialect for the scenarios' own accounts. */
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

/** Claim a hostname, refuse its verification until its TXT record exists, then verify it. */
test.for(TEST_DIALECTS)(
    "claim and verify a hostname through its TXT record on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(databases.get(dialect)!.database);
        const { accountId, hostname } = fixture;
        const owner = fixture.client(fixture.ownerId);

        // claim the hostname for the account, unverified
        const claimed = await owner.domain.create({
            accountId,
            requestId: RequestId.create(),
            hostname,
        });
        const record = {
            id: claimed.id,
            scope: accountId,
            hostname,
            verifiedAt: null,
            revision: 1,
            tags: {},
            createdAt: NOW,
            createdBy: Subject.key(principal.user.reference(Scope.universe.id, fixture.ownerId)),
            updatedAt: NOW,
            updatedBy: Subject.key(principal.user.reference(Scope.universe.id, fixture.ownerId)),
        };
        expect(claimed).toEqual(record);

        // refuse verification before the challenge record exists
        const challenge = `_destack.${hostname}`;
        await expect(
            owner.domain.verify({ accountId, id: claimed.id, requestId: RequestId.create() }),
        ).rejects.toEqual(
            new ServiceError("PRECONDITION_FAILED", {
                message: `publish a TXT record ${challenge} with destack-verify=${claimed.id} first`,
                defined: true,
            }),
        );

        // verify once the record exists, reading only the challenge name each time
        fixture.publish(claimed);
        const verified = { ...record, verifiedAt: NOW, revision: 2 };
        expect(
            await owner.domain.verify({ accountId, id: claimed.id, requestId: RequestId.create() }),
        ).toEqual(verified);
        expect(await owner.domain.get({ accountId, id: claimed.id })).toEqual(verified);
        expect(fixture.lookups).toEqual([challenge, challenge]);
    },
);

/** Refuse hostnames of the platform's own domains and IP literals as claims. */
test.for(TEST_DIALECTS)(
    "refuse reserved platform domains and IP literals on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(databases.get(dialect)!.database);
        const { accountId } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const claim = (hostname: string) =>
            owner.domain.create({ accountId, requestId: RequestId.create(), hostname });

        // refuse a reserved domain, a subdomain of one, and IPv4 and IPv6 literals
        const reserved = new ServiceError("BAD_REQUEST", {
            message: "invalid input: hostname: platform domains are reserved",
        });
        const malformed = new ServiceError("BAD_REQUEST", {
            message:
                "invalid input: hostname: hostnames are lowercase ASCII labels, punycode for others",
        });
        await expect(claim("destack.app")).rejects.toEqual(reserved);
        await expect(claim("notes.acme.destack.space")).rejects.toEqual(reserved);
        await expect(claim("192.168.0.1")).rejects.toEqual(malformed);
        await expect(claim("[::1]")).rejects.toEqual(malformed);
        expect(await owner.domain.list({ accountId })).toEqual({ items: [], cursor: null });
    },
);

/** Let several accounts claim one hostname, and let only the first to prove it verify it. */
test.for(TEST_DIALECTS)(
    "let a second account claim a hostname and refuse its verification once another verified it on %s",
    async (dialect) => {
        await using fixture = await DomainFixture.open(databases.get(dialect)!.database);
        const { accountId, otherId, hostname } = fixture;
        const owner = fixture.client(fixture.ownerId);
        const rival = fixture.client(fixture.rivalId);

        // claim the hostname in both accounts
        const first = await owner.domain.create({
            accountId,
            requestId: RequestId.create(),
            hostname,
        });
        const second = await rival.domain.create({
            accountId: otherId,
            requestId: RequestId.create(),
            hostname,
        });
        expect(second).toEqual({
            id: second.id,
            scope: otherId,
            hostname,
            verifiedAt: null,
            revision: 1,
            tags: {},
            createdAt: NOW,
            createdBy: Subject.key(principal.user.reference(Scope.universe.id, fixture.rivalId)),
            updatedAt: NOW,
            updatedBy: Subject.key(principal.user.reference(Scope.universe.id, fixture.rivalId)),
        });

        // verify the first claim, then refuse the second although its record exists too
        fixture.publish(first);
        fixture.publish(second);
        await owner.domain.verify({ accountId, id: first.id, requestId: RequestId.create() });
        await expect(
            rival.domain.verify({
                accountId: otherId,
                id: second.id,
                requestId: RequestId.create(),
            }),
        ).rejects.toEqual(
            new ServiceError("CONFLICT", {
                message: `another account verified ${fixture.hostname}`,
                defined: true,
            }),
        );
        expect(await rival.domain.get({ accountId: otherId, id: second.id })).toEqual(second);
    },
);

/** Refuse every domain method to a user outside the account. */
test.for(TEST_DIALECTS)("refuse domain methods to a non-member on %s", async (dialect) => {
    await using fixture = await DomainFixture.open(databases.get(dialect)!.database);
    const { accountId, hostname } = fixture;
    const owner = fixture.client(fixture.ownerId);
    const stranger = fixture.client(fixture.strangerId);
    const claimed = await owner.domain.create({
        accountId,
        requestId: RequestId.create(),
        hostname,
    });
    fixture.publish(claimed);

    // refuse reading, claiming, verifying and deleting the account's domains
    const id = claimed.id;
    const hidden = new ServiceError("NOT_FOUND", {
        message: `no scope ${accountId}`,
        defined: true,
    });
    await expect(stranger.domain.list({ accountId })).rejects.toEqual(hidden);
    await expect(stranger.domain.get({ accountId, id })).rejects.toEqual(hidden);
    await expect(
        stranger.domain.create({ accountId, requestId: RequestId.create(), hostname }),
    ).rejects.toEqual(hidden);
    await expect(
        stranger.domain.verify({ accountId, id, requestId: RequestId.create() }),
    ).rejects.toEqual(hidden);
    await expect(
        stranger.domain.delete({ accountId, id, requestId: RequestId.create() }),
    ).rejects.toEqual(hidden);

    // keep the claim unverified, and ask the resolver nothing
    expect(await owner.domain.get({ accountId, id })).toEqual(claimed);
    expect(fixture.lookups).toEqual([]);
});
