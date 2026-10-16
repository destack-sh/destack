import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { StepUp } from "@destack/access";
import type { Select } from "@destack/db";
import type { ObjectClient } from "@destack/object/client";
import { Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { until } from "@destack/runtime/timer";
import { client, type ClientRegistration } from "../object/client.ts";
import type { DeviceAuthorization, DevicePoll, Issuer, SignedIn } from "./issuer.ts";
import type { Residency } from "../object/residency.ts";
import { type Login, login } from "./login.ts";
import type { ClientKey } from "./client-key.ts";
import type { SessionKeychain } from "./session-keychain.ts";
import type { SignUp } from "../better-auth/sign-up.ts";
import type { Clock } from "@destack/environment";

/** The loopback path a browser sign-in returns to (RFC 8252 7.3). */
const REDIRECT_PATH = "/callback";

/** The loopback address the redirect listens on (RFC 8252 7.3). */
const LOOPBACK = "127.0.0.1";

/** How long a device flow waits between polls, in milliseconds: five seconds (RFC 8628 3.2). */
const POLL_MILLISECONDS = 5000;

/** How much longer each slow down makes a device flow wait: five seconds (RFC 8628 3.5). */
const SLOW_DOWN_MILLISECONDS = 5000;

/** The local logins of a native client, which its sign-ins write. */
export type LoginClient = ObjectClient<{ readonly login: typeof login }>;

/** What a native client signs people in with. */
export interface NativeSignInOptions {
    /** The issuer people sign in with. */
    readonly issuer: Issuer;
    /** The keychain's refresh tokens of the signed-in people. */
    readonly sessions: SessionKeychain;
    /** The keys of the clients sign-ins register, proving them to this computer's machine. */
    readonly keys: ClientKey;
    /** The native client's local logins. */
    readonly logins: LoginClient;
    /** The clock sign-ins are timed by. */
    readonly clock: Clock;
    /** What the client a sign-in registers in the person's account reports about itself. */
    readonly client: Omit<ClientRegistration, "id"> & {
        /** The processor architecture, such as arm64. */
        readonly architecture: string;
        /** The version of the distribution registering the client. */
        readonly version: string;
    };
}

/** A device sign-in in progress. */
export interface DeviceSignIn {
    /** The page the person opens. */
    readonly url: string;
    /** The code the person enters there. */
    readonly userCode: string;
    /** Poll the issuer once, and return the delay before the next poll or the granted login. */
    readonly poll: () => Promise<number | Login>;
}

/** An email sign-in in progress: a code mailed to the person. */
export interface EmailSignIn {
    /** The email the code was mailed to. */
    readonly email: string;
    /** Sign the person in with the mailed code. */
    readonly verify: (code: string) => Promise<Login>;
}

/** Sign people in on a native client, each sign-in registering a new client for the person. */
export class NativeSignIn {
    /** What the client signs people in with. */
    readonly #options: NativeSignInOptions;

    /** Sign people in with an issuer, the keychain's refresh tokens and the client's logins. */
    constructor(options: NativeSignInOptions) {
        this.#options = options;
    }

    /** Sign a person in through the browser with PKCE and a loopback redirect. */
    async browser(
        open: (url: string) => Promise<void>,
        stepUp?: StepUp,
        signal?: AbortSignal,
    ): Promise<Login> {
        // listen for the redirect on a free loopback port
        const { issuer, clock } = this.#options;
        await using redirect = await LoopbackRedirect.listen();

        // send the person to the issuer as a new client, and fail once the browser cannot open
        const clientId = Identifier.create("client");
        const state = crypto.randomUUID();
        const authorization = await issuer.authorize(
            clientId,
            redirect.url,
            state,
            clock.now(),
            stepUp,
        );
        const browserFailure = new AbortController();
        const opening = open(authorization.url);
        opening.catch((error: unknown) => browserFailure.abort(error));

        // wait for the browser to return with this sign-in's state
        const stop = AbortSignal.any([
            AbortSignal.timeout(Math.max(0, authorization.expiresAt - clock.now())),
            browserFailure.signal,
            ...(signal === undefined ? [] : [signal]),
        ]);
        const callback = await redirect.receive(state, stop);

        // fail a refused sign-in or one another issuer answers
        const code = callback.url.searchParams.get("code");
        if (code === null || callback.url.searchParams.get("iss") !== issuer.url) {
            callback.answer(400, "Sign-in failed. Start it again from Destack.");
            await opening;
            throw new CallError("UNAUTHORIZED", { message: "the sign-in did not complete" });
        }

        // redeem the code for the client's tokens and sign the person in
        try {
            const signedIn = await issuer.redeem(
                clientId,
                code,
                authorization.verifier,
                redirect.url,
            );
            const result = await this.#complete(clientId, signedIn);
            callback.answer(200, "Signed in. You can close this window.");
            await opening;

            return result;
        } catch (error) {
            callback.answer(400, "Sign-in failed. Start it again from Destack.");
            throw error;
        }
    }

    /** Sign a person in with a device code. */
    async device(
        show: (started: Pick<DeviceSignIn, "url" | "userCode">) => Promise<void>,
        signal?: AbortSignal,
    ): Promise<Login> {
        // show the code
        const deviceSignIn = await this.startDevice();
        await show(deviceSignIn);

        // poll at the issuer's pace until it answers
        for (;;) {
            // return the login once granted
            const result = await deviceSignIn.poll();
            if (typeof result !== "number") {
                return result;
            }

            // wait the delay the poll returned
            await until(
                AbortSignal.any([
                    AbortSignal.timeout(result),
                    ...(signal === undefined ? [] : [signal]),
                ]),
            );
            signal?.throwIfAborted();
        }
    }

    /** Start a device sign-in of a new client, polled once at a time. */
    async startDevice(): Promise<DeviceSignIn> {
        // ask the issuer for the codes
        const clientId = Identifier.create("client");
        const authorization = await this.#options.issuer.device(
            clientId,
            this.#options.clock.now(),
        );

        // poll at the issuer's pace, slowing down when asked
        let interval = POLL_MILLISECONDS;
        const poll = async (): Promise<number | Login> => {
            // sign the person in once granted
            const result = await this.#poll(clientId, authorization);
            if (result.status === "granted") {
                return this.#complete(clientId, result.signedIn);
            }
            // wait while the person decides
            else if (result.status === "pending") {
                return interval;
            }
            // wait longer after each slow down
            else if (result.status === "slow") {
                interval += SLOW_DOWN_MILLISECONDS;

                return interval;
            }
            // fail a refused or expired sign-in
            else {
                throw new CallError("UNAUTHORIZED", { message: "the sign-in did not complete" });
            }
        };

        return { url: authorization.url, userCode: authorization.userCode, poll };
    }

    /** Sign a person in with a code mailed to their email, or sign a new person up. */
    async email(
        email: string,
        ask: (started: EmailSignIn) => Promise<string>,
        signUp?: schema.Input<typeof SignUp>,
    ): Promise<Login> {
        const started = await this.startEmail(email, signUp);

        return started.verify(await ask(started));
    }

    /** Start an email sign-in of a new client: mail the person a code. */
    async startEmail(email: string, signUp?: schema.Input<typeof SignUp>): Promise<EmailSignIn> {
        // mail the code
        const { issuer } = this.#options;
        await issuer.sendCode(email);

        // redeem the code as a new client and sign the person in
        const clientId = Identifier.create("client");
        const verify = async (code: string) =>
            this.#complete(clientId, await issuer.redeemCode(clientId, email, code, signUp));

        return { email, verify };
    }

    /** List the residencies a new person's account may keep data in. */
    residencies(): Promise<Pick<Residency, "id" | "name">[]> {
        return this.#options.issuer.residencies();
    }

    /** Sign a person out: revoke and forget their refresh token, and remove their login. */
    async signOut(current: Login): Promise<void> {
        // revoke the refresh token at its issuer and forget it
        const { sessions, logins, issuer } = this.#options;
        const person = { issuer: current.issuer, subject: current.subjectId };
        const token = await sessions.load(person);
        if (token !== undefined) {
            await issuer.revoke(client.identifier(current.registrationId), token);
            await sessions.remove(person, token);
        }

        // remove the login
        await logins.mutate(login).delete({ id: current.id }).confirmed;
    }

    /** Poll a device code once, and refuse an expired code. */
    #poll(clientId: Identifier<"client">, authorization: DeviceAuthorization): Promise<DevicePoll> {
        // refuse an expired code
        if (authorization.expiresAt <= this.#options.clock.now()) {
            return Promise.resolve({ status: "refused" });
        }

        return this.#options.issuer.poll(clientId, authorization.deviceCode);
    }

    /** Sign in the person a new client's tokens act for, replacing their earlier client here. */
    async #complete(clientId: Identifier<"client">, signedIn: SignedIn): Promise<Login> {
        // read the ID token's person, and their name as they read it
        const { issuer, sessions, logins } = this.#options;
        const { tokens, userId: sub } = signedIn;
        const accounts = issuer.accounts(tokens.accessToken);
        const { name } = await accounts.user.get({ id: sub });
        const person = { issuer: issuer.url, subject: sub };

        // keep the refresh token, or adopt a racing sign-in's and revoke this one
        const kept = await sessions.keep(person, tokens.refreshToken);
        if (kept !== tokens.refreshToken) {
            await issuer.revoke(clientId, tokens.refreshToken);
        }

        // report the client's details, and withdraw the client an earlier sign-in here registered
        await this.#describe(accounts, sub, clientId);
        const existing = await logins.query.login.findFirst({
            where: { issuer: person.issuer, subjectId: person.subject },
        });
        if (existing !== undefined && existing.registrationId !== clientId) {
            const withdrawn = client.identifier(existing.registrationId);
            await this.#withdraw(accounts, sub, withdrawn);
            await this.#options.keys.forget(withdrawn);
        }

        // keep the key of the registered client, which proves it to this computer's machine
        await this.#options.keys.generate(clientId);

        return this.#upsert(existing, {
            issuer: person.issuer,
            subjectId: person.subject,
            name,
            status: "authenticated",
            registrationId: clientId,
        });
    }

    /** Report the client's details the issuer registered it without. */
    async #describe(
        accounts: ReturnType<Issuer["accounts"]>,
        userId: Identifier<"user">,
        clientId: Identifier<"client">,
    ): Promise<void> {
        const { client: described, clock } = this.#options;
        await accounts.client.update({
            userId,
            id: clientId,
            requestId: RequestId.create(clock.now()),
            name: described.name,
            ...(described.platform === undefined ? {} : { platform: described.platform }),
            architecture: described.architecture,
            version: described.version,
        });
    }

    /** Withdraw a client an earlier sign-in registered, unless the person withdrew it already. */
    async #withdraw(
        accounts: ReturnType<Issuer["accounts"]>,
        userId: Identifier<"user">,
        clientId: Identifier<"client">,
    ): Promise<void> {
        const registered = await accounts.client.get({ userId, id: clientId });
        if (registered.revokedAt === null) {
            await accounts.client.revoke({
                userId,
                id: clientId,
                requestId: RequestId.create(this.#options.clock.now()),
            });
        }
    }

    /** Create the person's login, or update the one an earlier sign-in wrote. */
    async #upsert(
        existing: Login | undefined,
        fields: Pick<
            Select<typeof login.table>,
            "issuer" | "subjectId" | "name" | "status" | "registrationId"
        >,
    ): Promise<Login> {
        // update it or create the first
        const { logins } = this.#options;
        const { issuer, subjectId, ...changed } = fields;
        const mutation =
            existing === undefined
                ? logins.mutate(login).create({ issuer, subjectId, ...changed })
                : logins.mutate(login).update({ id: existing.id, ...changed });
        await mutation.confirmed;

        // read the login as clients read it
        const { id } = await mutation.predicted;
        const row = await logins.query.login.findFirst({ where: { id } });
        if (row === undefined) {
            throw new TypeError(`the sign-in wrote no login ${id}`);
        }

        return row;
    }
}

/** A browser returning to the loopback redirect, answered once its sign-in settles. */
interface Callback {
    /** The redirect's URL with its query. */
    readonly url: URL;
    /** Answer the browser with a short page. */
    readonly answer: (status: number, text: string) => void;
}

/** A loopback listener receiving one browser sign-in's redirect (RFC 8252 7.3). */
class LoopbackRedirect implements AsyncDisposable {
    /** The redirect URL the issuer returns the browser to. */
    readonly url: string;
    /** The listening server. */
    readonly #server: ReturnType<typeof createServer>;
    /** The returned browsers that wait to be taken. */
    readonly #callbacks: Callback[] = [];
    /** Wake a waiting receive once a browser returns. */
    #wake: (() => void) | undefined;

    /** Keep a listening server and its redirect URL. */
    private constructor(server: ReturnType<typeof createServer>, port: number) {
        this.#server = server;
        this.url = `http://${LOOPBACK}:${port}${REDIRECT_PATH}`;
    }

    /** Listen on a free loopback port. */
    static async listen(): Promise<LoopbackRedirect> {
        // listen on a free port
        const server = createServer();
        await new Promise<void>((resolve, reject) => {
            server.once("error", reject);
            server.listen(0, LOOPBACK, () => resolve());
        });

        // read the port the server listens on
        const address = server.address();
        if (address === null || typeof address === "string") {
            throw new TypeError("the redirect listens on no port");
        }

        // take each request to the redirect
        const redirect = new LoopbackRedirect(server, address.port);
        server.on("request", (request, response) => redirect.#take(request, response));

        return redirect;
    }

    /** Wait for the browser with a sign-in's state until a signal aborts, and turn others away. */
    async receive(state: string, signal: AbortSignal): Promise<Callback> {
        for (;;) {
            // take a browser with the state
            const callback = this.#callbacks.shift();
            if (callback !== undefined && callback.url.searchParams.get("state") === state) {
                return callback;
            }
            // turn away a browser with another state
            else if (callback !== undefined) {
                callback.answer(400, "No sign-in waits for this page.");
                continue;
            }

            // wait for the next browser or the end
            signal.throwIfAborted();
            const next = Promise.withResolvers<void>();
            this.#wake = next.resolve;
            await Promise.race([next.promise, until(signal)]);
        }
    }

    /** Take a request at the redirect path, answering any other with not found. */
    #take(request: IncomingMessage, response: ServerResponse): void {
        // answer another path at once
        const url = new URL(request.url ?? "/", this.url);
        if (url.pathname !== REDIRECT_PATH) {
            answer(response, 404, "No sign-in waits for this page.");

            return;
        }

        // queue it for the waiting sign-in, which answers once at most
        let isAnswered = false;
        this.#callbacks.push({
            url,
            answer: (status, text) => {
                if (!isAnswered) {
                    isAnswered = true;
                    answer(response, status, text);
                }
            },
        });
        this.#wake?.();
    }

    /** Answer the browsers still waiting and stop listening. */
    async [Symbol.asyncDispose](): Promise<void> {
        // turn away the browsers still waiting
        for (const callback of this.#callbacks.splice(0)) {
            callback.answer(400, "No sign-in waits for this page.");
        }

        // close the listener once its idle connections end
        const closing = new Promise<void>((resolve) => {
            this.#server.close(() => resolve());
        });
        this.#server.closeIdleConnections();
        await closing;
    }
}

/** Answer a browser with a short plain page. */
function answer(response: ServerResponse, status: number, text: string): void {
    // write an uncached plain page
    response.writeHead(status, {
        "content-type": "text/plain; charset=utf-8",
        "cache-control": "no-store",
    });
    response.end(text);
}
