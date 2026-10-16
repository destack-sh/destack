import { principal, type StepUp } from "@destack/access";
import type { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { type Fetch, PACKAGE_PATH, PackageMount, type PackageHandle } from "@destack/runtime";
import { type ClientOptions, createClient } from "@destack/runtime/client";
import { TokenCache } from "@destack/runtime/authentication";
import { CallError } from "@destack/runtime/error";
import { Scope, type Subject } from "@destack/sync";
import { Account } from "../object/account.ts";
import { client } from "../object/client.ts";
import accountPackage from "../package.ts";
import { Issuer, type TokenBinding } from "./issuer.ts";
import type { Login } from "./login.ts";
import type { SessionKeychain } from "./session-keychain.ts";
import type { Clock } from "@destack/environment";

/** A person at an issuer, with one sign-in on this computer. */
export const Person = schema.object({
    /** The issuer. */
    issuer: schema.string(),
    /** The user. */
    subject: schema.string(),
});
/** A person at an issuer, with one sign-in on this computer. */
export type Person = schema.Infer<typeof Person>;

/** A person signed in on a computer through a client's refresh token, which the keychain stores. */
export class PersonClient {
    /** The signed-in person. */
    readonly subject: Subject;
    /** The issuer that signed the person in. */
    readonly issuer: Issuer;
    /** The client the person signed in through. */
    readonly clientId: Identifier<"client">;
    /** The keychain storing the client's refresh token, which each refresh replaces. */
    readonly #sessions: SessionKeychain;
    /** The fetch calling the issuer and the machines. */
    readonly #fetch: Fetch;
    /** The access tokens refreshed and exchanged for the person, by package and binding. */
    readonly #tokens = new TokenCache();
    /** The clock the tokens are reused by. */
    readonly #clock: Clock;

    /** Act as a person through a client's refresh token a keychain stores. */
    constructor(
        subject: Subject,
        signIn: { readonly issuer: string; readonly clientId: Identifier<"client"> },
        sessions: SessionKeychain,
        fetch: Fetch,
        clock: Clock,
    ) {
        // store the person, their client, the keychain, the fetch and the clock
        this.subject = subject;
        this.issuer = new Issuer(signIn.issuer, fetch);
        this.clientId = signIn.clientId;
        this.#sessions = sessions;
        this.#fetch = fetch;
        this.#clock = clock;
    }

    /** Act as the person of a login through the refresh token the keychain stores for them. */
    static async open(
        login: Login,
        sessions: SessionKeychain,
        fetch: Fetch,
        clock: Clock,
    ): Promise<PersonClient> {
        // require a person still signed in
        if (login.status !== "authenticated") {
            throw new CallError("UNAUTHORIZED", { message: "the person is not signed in" });
        }

        return new PersonClient(
            principal.user.reference(Scope.universe.id, login.subjectId),
            { issuer: login.issuer, clientId: client.identifier(login.registrationId) },
            sessions,
            fetch,
            clock,
        );
    }

    /** Connect to the account package at the person's issuer. */
    accounts() {
        return createClient(accountPackage, this.universe(accountPackage.id));
    }

    /** Reach a package the universe serves at the person's issuer, such as settings. */
    universe(audience: PackageId): ClientOptions {
        return {
            url: PackageMount.url(new URL(this.issuer.url).origin, audience),
            fetch: (request) => this.send(request, audience),
        };
    }

    /** Find the person's own account, which they signed up with. */
    async account(): Promise<Identifier<"account">> {
        // list the person's accounts of kind personal
        const { items } = await this.accounts().account.list({
            scope: schema.identifier("user").parse(this.subject.id),
            where: { kind: "personal" },
        });
        const [personal] = items;
        if (personal === undefined) {
            throw new CallError("NOT_FOUND", { message: "the person has no personal account" });
        }

        return personal.id;
    }

    /** Find the home space of the person's own account. */
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

    /** Reach the package of an installation of a space through its origin's proxy. */
    installation(origin: string, audience: PackageId, spaceId: Identifier<"space">): ClientOptions {
        return {
            url: `${origin}${PACKAGE_PATH}`,
            fetch: (request) => this.send(request, audience, { spaceId }),
        };
    }

    /** Read the person's token for a package, bound to a space or a grant, answering a step-up. */
    token(audience: PackageId, bound: TokenBinding = {}, stepUp?: StepUp): Promise<string> {
        // take the account token itself toward the account package outside spaces
        const { spaceId, grant } = bound;
        if (audience === accountPackage.id && spaceId === undefined && grant === undefined) {
            return this.#accountToken(stepUp);
        }

        // exchange it for the package's token
        return this.#tokens.token(
            JSON.stringify([audience, spaceId ?? null, grant ?? null]),
            this.#clock.now(),
            async () => this.issuer.exchange(await this.#accountToken(stepUp), audience, bound),
            stepUp,
        );
    }

    /** Send a request with the person's access token for a package, bound to a space or a grant. */
    async send(request: Request, audience: PackageId, bound: TokenBinding = {}) {
        // replace the request's credential with the token
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${await this.token(audience, bound)}`);

        return this.#fetch(new Request(request, { headers }));
    }

    /** Read the person's access token for the account package, refreshing it for a step-up. */
    #accountToken(stepUp?: StepUp): Promise<string> {
        return this.#tokens.token(
            accountPackage.id,
            this.#clock.now(),
            async () => {
                // read the refresh token the keychain stores, refusing a signed-out person
                const person = { issuer: this.issuer.url, subject: this.subject.id };
                const refreshToken = await this.#sessions.load(person);
                if (refreshToken === undefined) {
                    throw new CallError("UNAUTHORIZED", { message: "the person signed out" });
                }

                // refresh, storing the refresh token the issuer rotates to
                const tokens = await this.issuer.refresh(this.clientId, refreshToken);
                await this.#sessions.replace(person, tokens.refreshToken);

                return tokens;
            },
            stepUp,
        );
    }
}
