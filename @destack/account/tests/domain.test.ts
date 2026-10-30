import type { ClientOf } from "@destack/object/client";
import { ObjectReference, RequestId } from "@destack/sync";
import { expect, test, refusal } from "@destack/test";
import accountPackage from "../src/package.ts";
import type { Environment } from "@destack/environment";
import { TestDomain } from "./domain.ts";

/** Serve a hostname in an account as a client of the account package. */
function serve(
    client: ClientOf<typeof accountPackage>,
    accountId: string,
    name: string,
    environment: Environment,
) {
    return client.hostname.create({
        accountId,
        requestId: RequestId.create(environment),
        name,
    });
}

/** Write the row of a domain the fixture's owner claimed now, unverified. */
function claimedDomain(fixture: TestDomain, id: string) {
    const creator = fixture.owner.subject;

    return {
        id,
        scope: fixture.accountId,
        name: fixture.name,
        verifiedAt: null,
        revision: 1,
        tags: {},
        createdAt: fixture.clock.now(),
        ...ObjectReference.columns("createdBy", creator),
        updatedAt: fixture.clock.now(),
        ...ObjectReference.columns("updatedBy", creator),
        managerInstallationId: null,
        managerPackageId: null,
        managerName: null,
        detachedAt: null,
    };
}

test("claim and verify a domain once through its TXT record", async () => {
    await using fixture = await TestDomain.open();
    const { accountId, name } = fixture;
    const owner = fixture.owner.client(accountPackage);
    const verify = (id: string) =>
        owner.domain.verify({
            scope: accountId,
            id,
            requestId: RequestId.create(fixture.environment),
        });

    // claim the domain for the account on a plan with custom domains
    await fixture.accounts.entitle(accountId, "domain", { kind: "boolean" });
    const claimed = await owner.domain.create({
        scope: accountId,
        requestId: RequestId.create(fixture.environment),
        name,
    });
    const record = claimedDomain(fixture, claimed.id);
    expect(claimed).toEqual(record);

    // refuse verification before the challenge record exists
    const challenge = `_destack.${name}`;
    expect(await refusal(verify(claimed.id))).toEqual([
        "PRECONDITION_FAILED",
        "CHALLENGE_UNPUBLISHED",
        `publish a TXT record ${challenge} with destack-verify=${accountId} first`,
    ]);

    // verify once the record exists and refuse a second verification
    fixture.dns.publish(claimed);
    const verified = { ...record, verifiedAt: fixture.clock.now(), revision: 2 };
    expect(await verify(claimed.id)).toEqual(verified);
    expect(await refusal(verify(claimed.id))).toEqual([
        "CONFLICT",
        "DOMAIN_VERIFIED",
        `${name} is already verified`,
    ]);
    expect(await owner.domain.get({ scope: accountId, id: claimed.id })).toEqual(verified);

    // read only the challenge name each time
    expect(fixture.dns.lookups).toEqual([challenge, challenge]);
});

test("refuse IP literals and wildcards as domain claims", async () => {
    await using fixture = await TestDomain.open();
    const { accountId } = fixture;
    const owner = fixture.owner.client(accountPackage);
    const claim = (name: string) =>
        owner.domain.create({
            scope: accountId,
            requestId: RequestId.create(fixture.environment),
            name,
        });

    // refuse IP literals and wildcards
    const malformed = [
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: name: hostnames are lowercase ASCII labels, punycode for others",
    ];
    expect(await refusal(claim("*.acme.com"))).toEqual(malformed);
    expect(await refusal(claim("192.168.0.1"))).toEqual([
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: name: top-level labels have a letter",
    ]);
    expect(await refusal(claim("[::1]"))).toEqual([
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: name: hostnames are lowercase ASCII labels, punycode for others; name: top-level labels have a letter",
    ]);
    expect(await owner.domain.list({ scope: accountId })).toEqual({ items: [], cursor: null });
});

test("let a second account claim a domain and refuse its verification once another verified it", async () => {
    await using fixture = await TestDomain.open();
    const { accountId, otherId, name } = fixture;
    const owner = fixture.owner.client(accountPackage);
    const rival = fixture.rival.client(accountPackage);

    // claim the domain in both accounts on plans with custom domains
    await fixture.accounts.entitle(accountId, "domain", { kind: "boolean" });
    await fixture.accounts.entitle(otherId, "domain", { kind: "boolean" });
    const first = await owner.domain.create({
        scope: accountId,
        requestId: RequestId.create(fixture.environment),
        name,
    });
    const second = await rival.domain.create({
        scope: otherId,
        requestId: RequestId.create(fixture.environment),
        name,
    });

    // publish both records and verify the first
    fixture.dns.publish(first);
    fixture.dns.publish(second);
    await owner.domain.verify({
        scope: accountId,
        id: first.id,
        requestId: RequestId.create(fixture.environment),
    });

    // refuse the second although its record exists too
    expect(
        await refusal(
            rival.domain.verify({
                scope: otherId,
                id: second.id,
                requestId: RequestId.create(fixture.environment),
            }),
        ),
    ).toEqual(["CONFLICT", "DOMAIN_TAKEN", `another owner verified ${name}`]);
    expect(await rival.domain.get({ scope: otherId, id: second.id })).toEqual(second);
});

test("serve hostnames below a verified domain alone, and refuse to remove the domain while one serves", async () => {
    await using fixture = await TestDomain.open();
    const { accountId, otherId, name } = fixture;
    const owner = fixture.owner.client(accountPackage);
    const rival = fixture.rival.client(accountPackage);
    const notes = `notes.${name}`;

    // refuse a hostname before its domain is verified
    await fixture.accounts.entitle(accountId, "domain", { kind: "boolean" });
    const claimed = await owner.domain.create({
        scope: accountId,
        requestId: RequestId.create(fixture.environment),
        name,
    });
    expect(await refusal(serve(owner, accountId, notes, fixture.environment))).toEqual([
        "PRECONDITION_FAILED",
        "DOMAIN_UNVERIFIED",
        `verify a domain of ${notes} first`,
    ]);
    fixture.dns.publish(claimed);
    await owner.domain.verify({
        scope: accountId,
        id: claimed.id,
        requestId: RequestId.create(fixture.environment),
    });
    await serve(owner, accountId, notes, fixture.environment);
    await serve(owner, accountId, name, fixture.environment);
    await serve(owner, accountId, `*.${name}`, fixture.environment);

    // refuse the rival a hostname below the domain
    expect(await refusal(serve(rival, otherId, `blog.${name}`, fixture.environment))).toEqual([
        "PRECONDITION_FAILED",
        "DOMAIN_UNVERIFIED",
        `verify a domain of blog.${name} first`,
    ]);
    const remove = () =>
        owner.domain.delete({
            scope: accountId,
            id: claimed.id,
            requestId: RequestId.create(fixture.environment),
        });
    const covered = [name, notes, `*.${name}`].toSorted().join(", ");
    expect(await refusal(remove())).toEqual([
        "PRECONDITION_FAILED",
        "HOSTNAMES_IN_USE",
        `hostnames below ${name} are in use: ${covered}`,
    ]);
    expect(
        (await owner.hostname.list({ accountId })).items.map((entry) => entry.name).toSorted(),
    ).toEqual([name, notes, `*.${name}`].toSorted());
});

test("hide an account's domains from a stranger to every domain method", async () => {
    await using fixture = await TestDomain.open();
    const { accountId, name } = fixture;
    const owner = fixture.owner.client(accountPackage);
    const stranger = fixture.stranger.client(accountPackage);
    await fixture.accounts.entitle(accountId, "domain", { kind: "boolean" });
    const claimed = await owner.domain.create({
        scope: accountId,
        requestId: RequestId.create(fixture.environment),
        name,
    });
    fixture.dns.publish(claimed);

    // hide the account from a stranger's listing, reads, claims, verifications and deletions
    const id = claimed.id;
    const scope = accountId;
    const hidden = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`];
    expect(await refusal(stranger.domain.list({ scope }))).toEqual(hidden);
    expect(await refusal(stranger.domain.get({ scope, id }))).toEqual(hidden);
    expect(
        await refusal(
            stranger.domain.create({
                scope,
                requestId: RequestId.create(fixture.environment),
                name,
            }),
        ),
    ).toEqual(hidden);
    expect(
        await refusal(
            stranger.domain.verify({ scope, id, requestId: RequestId.create(fixture.environment) }),
        ),
    ).toEqual(hidden);
    expect(
        await refusal(
            stranger.domain.delete({ scope, id, requestId: RequestId.create(fixture.environment) }),
        ),
    ).toEqual(hidden);

    // leave the claim unverified and ask the resolver nothing
    expect(await owner.domain.get({ scope, id })).toEqual(claimed);
    expect(fixture.dns.lookups).toEqual([]);
});

test("refuse a domain claim until the account's plan allows custom domains", async () => {
    // serve the domains under the account's entitlements
    await using fixture = await TestDomain.open();
    const owner = fixture.owner.client(accountPackage);
    const claim = () =>
        owner.domain.create({
            scope: fixture.accountId,
            requestId: RequestId.create(fixture.environment),
            name: fixture.name,
        });

    // refuse the claim, then accept it on a plan with custom domains
    expect(await refusal(claim())).toEqual([
        "PAYMENT_REQUIRED",
        "CUSTOM_DOMAINS_UNENTITLED",
        `the plan of ${fixture.accountId} does not allow custom domains`,
    ]);
    await fixture.accounts.entitle(fixture.accountId, "domain", { kind: "boolean" });
    const claimed = await claim();
    expect(claimed).toEqual(claimedDomain(fixture, claimed.id));
});
