import {
    type GrantClaim,
    type GrantPermission,
    principal,
    type StepUp,
    through,
} from "@destack/access";
import { type Select } from "@destack/db";
import type { Clock } from "@destack/environment";
import { defineObject, device, field, ListPage } from "@destack/object";
import type { PackageId } from "@destack/package";
import { type Fetch, PACKAGE_PATH, PackageMount, type PackageHandle } from "@destack/object";
import { type ClientOptions, createClient } from "@destack/object/client";
import { RequestId, ObjectReference, Scope, type Subject } from "@destack/sync";
import { canonicalize, type Identifier, schema } from "@destack/schema";
import { Account } from "../object/account.ts";
import { client } from "../object/client.ts";
import objectPackage from "@destack/object/package";
import accountPackage from "../package.ts";
import { Issuer, type TokenBinding } from "./issuer.ts";
import type { SessionStore } from "./session.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { ACCESS_TOKEN_TYPE } from "../oauth/exchange.ts";

/** A person signed in on a computer's native device, the keychain storing their refresh token. */
export const login = defineObject({
    name: "login",
    plural: "logins",
    scope: device,
    storage: "local",
    fields: {
        /** The universe that issued the session. */
        issuer: field.string(schema.url()).immutable(),
        /** The signed-in user. */
        subject: field.reference(principal.user).immutable(),
        /** The client the sign-in registered in the user's account, whose key proves it locally. */
        client: field.reference(principal.client),
        /** The user's display name at sign-in. */
        name: field.string(),
        /** Whether the sign-in is valid, or the person must sign in again. */
        status: field.enum(["authenticated", "reauthentication"]),
    },
    // let the people signed in on the device read and write them
    permissions: { read: through("device", "use"), write: through("device", "use") },
    constraints: {
        subject: { kind: "unique", on: ["scope", "issuer", "subjectId"] },
    },
});
/** A login as devices store it. */
export type LoginRow = Select<typeof login.table>;

/** A person signed in on a computer, acting through the session the keychain stores for them. */
export class Login {
    /** The login's row. */
    readonly row: LoginRow;
    /** The issuer that signed the person in. */
    readonly issuer: Issuer;
    /** The keychain storing the client's refresh token, which each refresh replaces. */
    readonly #sessions: SessionStore;
    /** The fetch calling the issuer and the machines. */
    readonly #fetch: Fetch;
    /** The clock the tokens are reused by. */
    readonly #clock: Clock;

    /** Act as the person of a login through the refresh token a keychain stores for them. */
    constructor(row: LoginRow, sessions: SessionStore, fetch: Fetch, clock: Clock) {
        // require a person still signed in
        if (row.status !== "authenticated") {
            throw AccountError.of("NOT_SIGNED_IN", "the person is not signed in");
        }

        // store the login, its issuer, the keychain, the fetch and the clock
        this.row = row;
        this.issuer = new Issuer(row.issuer, fetch);
        this.#sessions = sessions;
        this.#fetch = fetch;
        this.#clock = clock;
    }

    /** The signed-in person. */
    get subject(): Subject {
        return principal.user.reference(Scope.universe.id, this.row.subjectId);
    }

    /** Connect to the account package at the person's issuer. */
    accounts() {
        return createClient(accountPackage, this.universe(accountPackage.id));
    }

    /** Connect to the object package at the person's issuer, which serves their grants. */
    objects() {
        return createClient(objectPackage, this.universe(objectPackage.id));
    }

    /** Connect to a package the universe serves at the person's issuer, such as settings. */
    universe(audience: PackageId): ClientOptions {
        return {
            url: PackageMount.url(new URL(this.issuer.url).origin, audience),
            fetch: (request) => this.send(request, audience),
        };
    }

    /** Find the person's personal account, which they signed up with. */
    async account(): Promise<Identifier<"account">> {
        // list the person's accounts of kind personal
        const { items } = await this.accounts().account.list({
            scope: this.row.subjectId,
            where: { kind: "personal" },
        });
        const [personal] = items;
        if (personal === undefined) {
            throw AccountError.of("NO_PERSONAL_ACCOUNT", "the person has no personal account");
        }

        return personal.id;
    }

    /** Find the home space of the person's personal account. */
    async home(): Promise<Identifier<"space">> {
        return Account.home(await this.account());
    }

    /** Connect to a package a machine mounts in a space, with the person's token bound to it. */
    package<Handle extends PackageHandle>(
        handle: Handle,
        endpoint: string,
        spaceId: Identifier<"space">,
    ) {
        return createClient<Handle["router"]>(handle, {
            url: PackageMount.url(endpoint, handle.id),
            fetch: (request) => this.send(request, handle.id, { spaceId }),
        });
    }

    /** Connect to the package of an installation of a space through its origin's proxy. */
    installation(origin: string, audience: PackageId, spaceId: Identifier<"space">): ClientOptions {
        return {
            url: `${origin}${PACKAGE_PATH}`,
            fetch: (request) => this.send(request, audience, { spaceId }),
        };
    }

    /** Read the person's token for a package under a binding, answering a step-up. */
    token(audience: PackageId, bound: TokenBinding = {}, stepUp?: StepUp): Promise<string> {
        // take the account token itself toward the account package outside spaces
        const isUnbound = Object.keys(bound).length === 0;
        if (audience === accountPackage.id && isUnbound) {
            return this.#accountToken(stepUp);
        }

        // exchange it for the package's token, cached per package and binding
        return this.#sessions.accessTokens(this.row).token(
            canonicalize([audience, bound]),
            this.#clock.now(),
            async () => {
                const token = await this.#accountToken(stepUp);

                return this.issuer.exchange({ token, type: ACCESS_TOKEN_TYPE }, audience, bound);
            },
            stepUp,
        );
    }

    /** Find the person's unexpiring grant of exactly some permissions to a delegate. */
    async granted(
        delegate: Subject,
        permissions: readonly GrantPermission[],
    ): Promise<GrantClaim | undefined> {
        // list the person's grants to the delegate and take the unexpiring one of exactly these permissions
        const items = await Array.fromAsync(
            ListPage.all(this.objects().grant.list, {
                scope: this.subject.id,
                where: ObjectReference.columns("delegate", delegate),
            }),
        );
        const wanted = canonicalize(permissions);
        const existing = items.find(
            (grant) => grant.expiresAt === null && canonicalize(grant.permissions) === wanted,
        );

        return existing === undefined ? undefined : { id: existing.id, scope: this.subject.id };
    }

    /** Grant a delegate exactly some permissions, reusing the person's unexpiring grant of them. */
    async grant(delegate: Subject, permissions: readonly GrantPermission[]): Promise<GrantClaim> {
        // reuse the person's unexpiring grant of exactly these permissions
        const existing = await this.granted(delegate, permissions);
        if (existing !== undefined) {
            return existing;
        }

        // grant them otherwise
        const created = await this.objects().grant.create({
            scope: this.subject.id,
            requestId: RequestId.create(this.#clock.now()),
            delegate,
            permissions: [...permissions],
        });

        return { id: created.id, scope: this.subject.id };
    }

    /** Send a request with the person's access token for a package under a binding. */
    async send(request: Request, audience: PackageId, bound: TokenBinding = {}) {
        // replace the request's credential with the token
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${await this.token(audience, bound)}`);

        return this.#fetch(new Request(request, { headers }));
    }

    /** Read the person's access token for the account package, refreshing it for a step-up. */
    #accountToken(stepUp?: StepUp): Promise<string> {
        return this.#sessions.accessTokens(this.row).token(
            accountPackage.id,
            this.#clock.now(),
            async () => {
                // read the refresh token the keychain stores, refusing a signed-out person
                const refreshToken = await this.#sessions.load(this.row);
                if (refreshToken === undefined) {
                    throw AccountError.of("SIGNED_OUT", "the person signed out");
                }

                // refresh, storing the refresh token the issuer rotates to
                const tokens = await this.issuer.refresh(
                    client.identifier(this.row.clientId),
                    refreshToken,
                );
                await this.#sessions.replace(this.row, tokens.refreshToken);

                return tokens;
            },
            stepUp,
        );
    }
}
