import { type GrantClaim, GrantPermission, principal, type StepUp } from "@destack/access";
import type { Environment } from "@destack/environment";
import { ListPage } from "@destack/object";
import type { PackageId } from "@destack/package";
import { type Fetch, PACKAGE_PATH, PackageMount, type PackageHandle } from "@destack/object";
import { type Bookmark, type ClientOptions, createClient } from "@destack/object/client";
import { RequestId, ObjectReference, Scope, type Subject } from "@destack/sync";
import { canonicalize, type Identifier } from "@destack/schema";
import { SpaceAddress } from "../address/index.ts";
import { Account } from "../object/account.ts";
import objectPackage from "@destack/object/package";
import accountPackage from "../package.ts";
import { Issuer, type TokenBinding } from "./issuer.ts";
import type { PartitionId, TokenCache } from "@destack/access/token";
import { AccountError } from "../error/error.ts";
import { ACCESS_TOKEN_TYPE } from "../oauth/exchange.ts";

/** Where a person's account tokens come from, and the cache of the tokens exchanged for them. */
export interface PersonCredential {
    /** The cache of the person's access tokens, by package and binding. */
    readonly tokens: TokenCache;
    /** Issue the person's account access token anew. */
    readonly issue: () => Promise<string>;
}

/** A person acting through their account tokens at the issuer that signed them in. */
export class Person {
    /** The person's user identifier. */
    readonly subjectId: Identifier<"user">;
    /** The issuer that signed the person in. */
    readonly issuer: Issuer;
    /** Where the person's account tokens come from. */
    readonly #credential: PersonCredential;
    /** The fetch calling the issuer and the machines. */
    readonly #fetch: Fetch;
    /** The environment the calls run in, whose clock the tokens are reused by. */
    readonly #environment: Environment;
    /** The positions the person's calls saw, which later calls wait for. */
    readonly #bookmark: Bookmark;

    /** Act as a person at an issuer through their account tokens. */
    constructor(
        subjectId: Identifier<"user">,
        issuer: Issuer,
        credential: PersonCredential,
        fetch: Fetch,
        environment: Environment,
        bookmark: Bookmark,
    ) {
        // keep the person, their issuer, credential, fetch, environment and bookmark
        this.subjectId = subjectId;
        this.issuer = issuer;
        this.#credential = credential;
        this.#fetch = fetch;
        this.#environment = environment;
        this.#bookmark = bookmark;
    }

    /** The person. */
    get subject(): Subject {
        return principal.user.reference(Scope.universe.id, this.subjectId);
    }

    /** Connect to the account package at the person's issuer. */
    accounts() {
        return createClient(accountPackage, this.universe(accountPackage.id), this.#environment);
    }

    /** Connect to the object package at the person's issuer, which serves their grants. */
    objects() {
        return createClient(objectPackage, this.universe(objectPackage.id), this.#environment);
    }

    /** Connect to a package the universe serves at the person's issuer, such as settings. */
    universe(audience: PackageId): ClientOptions {
        return {
            url: PackageMount.url(new URL(this.issuer.url).origin, audience),
            fetch: (request) => this.send(request, audience),
            bookmark: this.#bookmark,
        };
    }

    /** Find the person's personal account, which they signed up with. */
    async account(): Promise<Identifier<"account">> {
        // list the person's accounts of kind personal
        const { items } = await this.accounts().account.list({
            scope: this.subjectId,
            where: { kind: "personal" },
        });
        const [personal] = items;
        if (personal === undefined) {
            throw AccountError.of("PERSONAL_ACCOUNT_MISSING", "the person has no personal account");
        }

        return personal.id;
    }

    /** List the addresses of the spaces of every account the person belongs to. */
    async spaces(): Promise<SpaceAddress[]> {
        // list the person's accounts, then each account's spaces
        const accounts = this.accounts();
        const owned = await Array.fromAsync(
            ListPage.all(accounts.account.list, { scope: this.subjectId }),
        );
        const spaces = await Promise.all(
            owned.map(async (account) => {
                const listed = await Array.fromAsync(
                    ListPage.all(accounts.space.list, { accountId: account.id }),
                );

                return listed.map((entry) =>
                    SpaceAddress.parse(
                        SpaceAddress.of({ space: entry.name, handle: account.handle }),
                    ),
                );
            }),
        );

        return spaces.flat().toSorted();
    }

    /** Find the home space of the person's personal account. */
    async home(): Promise<Identifier<"space">> {
        return Account.home(await this.account());
    }

    /** Find the space of the residency the person's personal account stores its data in. */
    async residency(): Promise<Identifier<"space">> {
        return this.accounts().residency.locate({ account: await this.account() });
    }

    /** Connect to a package a machine mounts in a partition, with the person's bound token. */
    package<Handle extends PackageHandle>(
        handle: Handle,
        endpoint: string,
        partition: PartitionId,
    ) {
        return createClient<Handle["router"]>(
            handle,
            {
                url: PackageMount.url(endpoint, handle.id),
                fetch: (request) => this.send(request, handle.id, { partition }),
                bookmark: this.#bookmark,
            },
            this.#environment,
        );
    }

    /** Connect to the package of an installation of a space through its origin's proxy. */
    installation(origin: string, audience: PackageId, spaceId: Identifier<"space">): ClientOptions {
        return {
            url: `${origin}${PACKAGE_PATH}`,
            fetch: (request) => this.send(request, audience, { partition: spaceId }),
            bookmark: this.#bookmark,
        };
    }

    /** Read the person's token for a package under a binding, answering a step-up. */
    async token(audience: PackageId, bound: TokenBinding = {}, stepUp?: StepUp): Promise<string> {
        // take the account token itself toward the account package outside spaces
        const isUnbound = Object.keys(bound).length === 0;
        if (audience === accountPackage.id && isUnbound) {
            return this.accountToken(stepUp);
        }

        // exchange it for the package's token, cached per package and binding
        return this.#credential.tokens.token(
            canonicalize([audience, bound]),
            this.#environment.clock.now(),
            async () => {
                const token = await this.accountToken(stepUp);
                const exchanged = await this.issuer.exchange(
                    { token, type: ACCESS_TOKEN_TYPE },
                    audience,
                    bound,
                );

                return exchanged.accessToken;
            },
            stepUp,
        );
    }

    /** Read the person's unexpiring grant to a delegate, absent without one. */
    async granted(
        delegate: Subject,
    ): Promise<GrantClaim | undefined> {
        // read the person's one grant to the delegate, an expiring one asking again
        const { items } = await this.objects().grant.list({
            scope: this.subject.id,
            where: ObjectReference.columns("delegate", delegate),
        });
        const [found] = items;

        return found === undefined || found.expiresAt !== null
            ? undefined
            : { id: found.id, scope: this.subject.id, permissions: found.permissions };
    }

    /** Grant a delegate some permissions, growing the person's grant to it to include them. */
    async grant(delegate: Subject, permissions: readonly GrantPermission[]): Promise<GrantClaim> {
        // reuse the person's grant when it includes them
        const found = await this.granted(delegate);
        if (found !== undefined && GrantPermission.includes(found.permissions, permissions)) {
            return found;
        }

        // grant them together with what the person granted already
        const merged = GrantPermission.merge(found?.permissions ?? [], permissions);
        const created = await this.objects().grant.create({
            scope: this.subject.id,
            requestId: RequestId.create(this.#environment),
            delegate,
            permissions: merged,
        });

        return { id: created.id, scope: this.subject.id, permissions: merged };
    }

    /** Send a request with the person's access token for a package under a binding. */
    async send(request: Request, audience: PackageId, bound: TokenBinding = {}) {
        // replace the request's credential with the token
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${await this.token(audience, bound)}`);

        return this.#fetch(new Request(request, { headers }));
    }

    /** Read the person's account token, renewed for a step-up. */
    accountToken(stepUp?: StepUp): Promise<string> {
        return this.#credential.tokens.token(
            accountPackage.id,
            this.#environment.clock.now(),
            () => this.#credential.issue(),
            stepUp,
        );
    }
}
