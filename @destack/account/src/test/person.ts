import type { Environment } from "@destack/environment";
import { principal } from "@destack/access";
import {
    type Client,
    PackageMount,
    type PackageHandle,
    type ProcedureRouter,
} from "@destack/object";
import type { PackageId } from "@destack/package";
import { type ClientOptions, type ClientTarget, createClient } from "@destack/object/client";
import { Identifier, present, schema, type UuidEnvironment } from "@destack/schema";
import { Scope, type Subject } from "@destack/sync";
import type { Issuer, TokenBinding, Tokens } from "../native/index.ts";
import { Account } from "../object/index.ts";
import type { ResidencyCode } from "../object/index.ts";
import type { TestMail } from "./account.ts";
import { ACCESS_TOKEN_TYPE, type SubjectToken } from "../oauth/exchange.ts";

/** A token exchanged for a package (RFC 8693), and when it expires. */
interface TestToken {
    /** The access token. */
    readonly accessToken: string;
    /** When it expires, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** Where people sign in as native clients do: the issuer, its mail and the residency. */
export interface TestSignIn {
    /** The issuer people sign up and in with. */
    readonly issuer: Issuer;
    /** The mail the issuer's codes arrive in. */
    readonly mail: TestMail;
    /** The origin the packages answer at. */
    readonly origin: string;
    /** Send a request to the packages, as a native client's network does. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** The world the person's calls run in, whose clock tokens expire by. */
    readonly environment: Environment;
    /** The residency new people sign up in. */
    readonly residencyId: ResidencyCode;
}

/** A person's sign-in on a client, as a template stores it to restore the person. */
export const PersonSignIn = schema.object({
    /** The person's user. */
    id: schema.identifier("user"),
    /** The email the person signs in with. */
    email: schema.string(),
    /** The handle of the person's personal account. */
    handle: schema.string(),
    /** The client the person signed in on. */
    clientId: schema.identifier("client"),
    /** The tokens the client holds. */
    tokens: schema.object({
        /** The access token for the account package. */
        accessToken: schema.string(),
        /** When the access token expires, in UTC epoch milliseconds. */
        expiresAt: schema.number(),
        /** The refresh token of the signed-in person. */
        refreshToken: schema.string(),
    }),
});
/** A person's sign-in on a client, as a template stores it to restore the person. */
export type PersonSignIn = schema.Infer<typeof PersonSignIn>;

/** A person signed in on a client of their own, calling packages with a token for each. */
export class TestPerson {
    /** The person's user. */
    readonly id: Identifier<"user">;
    /** The person as a principal. */
    readonly subject: Subject;
    /** The email the person signs in with. */
    readonly email: string;
    /** The person's personal account. */
    readonly account: Identifier<"account">;
    /** The person's home space. */
    readonly home: Identifier<"space">;
    /** The handle of the person's personal account. */
    readonly handle: string;
    /** The client the person signed in on. */
    readonly clientId: Identifier<"client">;
    /** The tokens of the person's client, refreshed once the access token expires. */
    #client: Promise<Tokens>;
    /** Where the person signed in. */
    readonly #signIn: TestSignIn;
    /** The tokens exchanged for packages, by package and binding, stored until they expire. */
    readonly #tokens = new Map<string, Promise<TestToken>>();

    /** Describe a signed-in person. */
    constructor(signIn: TestSignIn, person: PersonSignIn) {
        // store where the person signed in, their identities and their token
        this.#signIn = signIn;
        this.id = person.id;
        this.subject = principal.user.reference(Scope.universe.id, person.id);
        this.email = person.email;
        this.account = Account.personal(person.id);
        this.home = Account.home(this.account);
        this.handle = person.handle;
        this.clientId = person.clientId;
        this.#client = Promise.resolve(person.tokens);
    }

    /** Sign a person up or in with a mailed code, as a native client does, on a new client. */
    static async signIn(signIn: TestSignIn, email: string): Promise<TestPerson> {
        // mail the code and read it from the test mail
        await signIn.issuer.sendCode(email);
        const codes = (await signIn.mail.codes()).filter((mailed) => mailed.email === email);
        const code = present(codes.at(-1), `a code mailed to ${email}`).otp;

        // redeem it on a new client, signing a new person up in the residency
        const clientId = Identifier.create("client", signIn.environment);
        const { tokens, userId } = await signIn.issuer.redeemCode(clientId, email, code, {
            name: email,
            handle: freshHandle(signIn.environment),
            residencyId: signIn.residencyId,
            timeZone: "UTC",
            locale: "en",
        });

        // read the handle of the person's account, a new one's or an existing one's
        const personal = await signIn.issuer
            .accounts(tokens.accessToken)
            .account.get({ scope: userId, id: Account.personal(userId) });

        return new TestPerson(signIn, {
            id: userId,
            email,
            handle: personal.handle,
            clientId,
            tokens,
        });
    }

    /** Reach a package as the person, through its typed client, bound to a space when given. */
    client<Definition extends ProcedureRouter>(
        handle: PackageHandle & ClientTarget<Definition>,
        bound: TokenBinding = {},
    ): Client<Definition> {
        return createClient(handle, this.endpoint(handle, bound), this.#signIn.environment);
    }

    /** Address a package as the person, with a token for it exchanged again once it expires. */
    endpoint(handle: PackageHandle, bound: TokenBinding = {}): ClientOptions {
        const { origin, fetch } = this.#signIn;

        return {
            url: PackageMount.url(origin, handle.id),
            fetch: async (request: Request) => {
                // send the person's token for the package
                const { accessToken } = await this.exchange(handle.id, bound);
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${accessToken}`);

                return fetch(new Request(request, { headers }));
            },
        };
    }

    /** Read the client's tokens, refreshed once the access token expires (RFC 6749). */
    tokens(): Promise<Tokens> {
        // refresh expired tokens once, as every caller waits on the same refresh
        const current = this.#client;
        const refreshing = (async () => {
            const tokens = await current;
            if (tokens.expiresAt > this.#signIn.environment.clock.now()) {
                return tokens;
            }

            return await this.#signIn.issuer.refresh(this.clientId, tokens.refreshToken);
        })();
        this.#client = refreshing;

        // forget a refused refresh, keeping the tokens before it for the next caller
        refreshing.catch(() => {
            if (this.#client === refreshing) {
                this.#client = current;
            }
        });

        return refreshing;
    }

    /** Read the person's sign-in with the client's current tokens, as a template stores it. */
    async record(): Promise<PersonSignIn> {
        const tokens = await this.tokens();

        return {
            id: this.id,
            email: this.email,
            handle: this.handle,
            clientId: this.clientId,
            tokens,
        };
    }

    /** Read the client's current access token as the subject token it exchanges. */
    async token(): Promise<SubjectToken> {
        const { accessToken } = await this.tokens();

        return { token: accessToken, type: ACCESS_TOKEN_TYPE };
    }

    /** Exchange the person's token for a package's, reusing one until it expires. */
    exchange(audience: PackageId, bound: TokenBinding = {}): Promise<TestToken> {
        // reuse a live token for the package and binding
        const key = JSON.stringify([audience, bound]);
        const kept = this.#tokens.get(key);
        const exchanging = (async () => {
            const current = await kept;
            if (current !== undefined && current.expiresAt > this.#signIn.environment.clock.now()) {
                return current;
            }

            return this.#signIn.issuer.exchange(await this.token(), audience, bound);
        })();

        // store it, forgetting a refused exchange
        this.#tokens.set(key, exchanging);
        exchanging.catch(() => this.#tokens.delete(key));

        return exchanging;
    }
}

/** Make a free handle for a person or an account a test creates. */
export function freshHandle(environment: UuidEnvironment): string {
    return `account-${Identifier.create("account", environment).slice(-12)}`;
}
