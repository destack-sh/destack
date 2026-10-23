import { principal } from "@destack/access";
import type { Clock } from "@destack/environment";
import {
    type Client,
    PackageMount,
    type PackageHandle,
    type ProcedureRouter,
} from "@destack/object";
import type { PackageId } from "@destack/package";
import { type ClientOptions, type ClientTarget, createClient } from "@destack/object/client";
import { Identifier, present } from "@destack/schema";
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
    /** The clock tokens expire by. */
    readonly clock: Clock;
    /** The residency new people sign up in. */
    readonly residencyId: ResidencyCode;
}

/** A person signed in on a client of their own, reaching packages with a token for each. */
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
    /** The access token of the person's client, which it exchanges for packages' tokens. */
    readonly token: SubjectToken;
    /** The refresh token of the person's client. */
    readonly refreshToken: string;
    /** Where the person signed in. */
    readonly #signIn: TestSignIn;
    /** The tokens exchanged for packages, by package and binding, kept until they expire. */
    readonly #tokens = new Map<string, Promise<TestToken>>();

    /** Keep a signed-in person. */
    constructor(
        signIn: TestSignIn,
        person: {
            /** The person's user. */
            readonly id: Identifier<"user">;
            /** The email the person signs in with. */
            readonly email: string;
            /** The handle of the person's personal account. */
            readonly handle: string;
            /** The client the person signed in on. */
            readonly clientId: Identifier<"client">;
            /** The tokens the client holds. */
            readonly tokens: Tokens;
        },
    ) {
        // keep where the person signed in, their identities and their token
        this.#signIn = signIn;
        this.id = person.id;
        this.subject = principal.user.reference(Scope.universe.id, person.id);
        this.email = person.email;
        this.account = Account.personal(person.id);
        this.home = Account.home(this.account);
        this.handle = person.handle;
        this.clientId = person.clientId;
        this.token = { token: person.tokens.accessToken, type: ACCESS_TOKEN_TYPE };
        this.refreshToken = person.tokens.refreshToken;
    }

    /** Sign a person up or in with a mailed code, as a native client does, on a new client. */
    static async signIn(signIn: TestSignIn, email: string): Promise<TestPerson> {
        // mail the code and read it from the test mail
        await signIn.issuer.sendCode(email);
        const codes = (await signIn.mail.codes()).filter((mailed) => mailed.email === email);
        const code = present(codes.at(-1), `a code mailed to ${email}`).otp;

        // redeem it on a new client, signing a new person up in the residency
        const clientId = Identifier.create("client");
        const { tokens, userId } = await signIn.issuer.redeemCode(clientId, email, code, {
            name: email,
            handle: freshHandle(),
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
        return createClient(handle, this.endpoint(handle, bound));
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

    /** Exchange the person's token for a package's, reusing one until it expires. */
    exchange(audience: PackageId, bound: TokenBinding = {}): Promise<TestToken> {
        // reuse a live token for the package and binding
        const key = JSON.stringify([audience, bound]);
        const kept = this.#tokens.get(key);
        const exchanging = (async () => {
            const current = await kept;
            if (current !== undefined && current.expiresAt > this.#signIn.clock.now()) {
                return current;
            }

            return this.#signIn.issuer.exchange(this.token, audience, bound);
        })();

        // keep it, forgetting a refused exchange
        this.#tokens.set(key, exchanging);
        exchanging.catch(() => this.#tokens.delete(key));

        return exchanging;
    }
}

/** Make a free handle for a person or an account a test creates. */
export function freshHandle(): string {
    return `account-${Identifier.create("account").slice(-12)}`;
}
