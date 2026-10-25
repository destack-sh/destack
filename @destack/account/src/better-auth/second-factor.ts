import type { BetterAuthPlugin } from "better-auth";
import {
    APIError,
    createAuthEndpoint,
    createAuthMiddleware,
    getSessionFromCtx,
    sensitiveSessionMiddleware,
} from "better-auth/api";
import {
    deleteSessionCookie,
    expireCookie,
    parseCookies,
    setSessionCookie,
} from "better-auth/cookies";
import type { Select } from "@destack/db";
import type { Environment } from "@destack/environment";
import type { Keyring } from "@destack/identity";
import { present, schema } from "@destack/schema";
import { Challenge, Factor, LOCKOUT_MILLISECONDS, MAX_FAILURES } from "../authentication/factor.ts";
import { AccountError } from "../error/error.ts";
import { refusing } from "./refusal.ts";
import { challenge } from "../object/challenge.ts";
import { factor } from "../object/factor.ts";
import { betterAuthModel } from "./adapter.ts";

/** The Better Auth route a mailed sign-in link opens. */
const LINK_PATH = "/magic-link/verify";

/** The prefix of the Better Auth routes a provider's sign-in returns to. */
const CALLBACK_PREFIX = "/callback/";

/** The routes whose new sessions need no challenge: passkeys, and the second factor's own. */
const EXEMPT_PREFIXES = ["/passkey/", "/two-factor/"];

/** The cookie a browser keeps a sign-in's second-factor challenge token in. */
const CHALLENGE_COOKIE = "challenge";

/** The body of a code a person submits. */
const CodeBody = schema.object({ code: schema.string().min(1) });

/** What the second factor needs: the keyring sealing secrets and the environment it runs at. */
export interface SecondFactorOptions {
    /** The page a sign-in's challenge is answered on. */
    readonly challengeUri: string;
    /** The keyring sealing each factor's secret. */
    readonly keyring: Keyring;
    /** The clock, WebCrypto and randomness codes and challenges use. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
}

/** Require an enrolled authenticator after every first-factor sign-in. */
export function betterAuthSecondFactor(options: SecondFactorOptions): BetterAuthPlugin {
    const { keyring, environment } = options;

    return {
        id: "destack-second-factor",
        schema: { factor: betterAuthModel(factor), challenge: betterAuthModel(challenge) },
        endpoints: {
            /** Enroll an authenticator for the signed-in person, confirmed by its first code. */
            enrollFactor: createAuthEndpoint(
                "/two-factor/enable",
                { method: "POST", use: [sensitiveSessionMiddleware] },
                async (context) =>
                    refusing(async () => {
                        // refuse enrolling over a verified factor, which only disabling removes
                        const person = context.context.session.user;
                        const { adapter } = context.context;
                        const verified = await adapter.findOne({
                            model: "factor",
                            where: [
                                { field: "userId", value: person.id },
                                { field: "status", value: "verified" },
                            ],
                        });
                        if (verified !== null) {
                            throw AccountError.of(
                                "FACTOR_ENROLLED",
                                "the person has an authenticator: disable it first",
                            );
                        }

                        // replace a pending enrollment with a new one, never a verified one
                        const enrolled = await Factor.enroll(person, keyring, environment);
                        await adapter.deleteMany({
                            model: "factor",
                            where: [
                                { field: "userId", value: person.id },
                                { field: "status", value: "pending" },
                            ],
                        });
                        await adapter.create({
                            model: "factor",
                            data: { userId: person.id, ...enrolled.row },
                        });

                        return context.json({
                            totpURI: enrolled.uri,
                            backupCodes: enrolled.recoveryCodes,
                        });
                    }),
            ),

            /** Accept a code, confirming an enrollment or answering a challenge. */
            verifyFactor: createAuthEndpoint(
                "/two-factor/verify-totp",
                { method: "POST", body: CodeBody },
                async (context) =>
                    refusing(async () => {
                        // check the signed-in person's code against their factor
                        const current = await getSessionFromCtx(context);
                        if (current !== null) {
                            const row = await readFactor(context, current.user.id);
                            await attempt(context, options, row, async (claimed) => ({
                                lastStep: await Factor.verify(
                                    claimed,
                                    context.body.code,
                                    keyring,
                                    environment,
                                ),
                                status: "verified",
                            }));

                            return context.json({ status: true });
                        }

                        // answer the sign-in's challenge with the code
                        return answer(context, options, async (row) => {
                            const step = await Factor.verify(
                                row,
                                context.body.code,
                                keyring,
                                environment,
                            );

                            return { lastStep: step };
                        });
                    }),
            ),

            /** Answer a sign-in's challenge with a recovery code, spending it. */
            recoverFactor: createAuthEndpoint(
                "/two-factor/verify-backup-code",
                { method: "POST", body: CodeBody },
                async (context) =>
                    refusing(() =>
                        answer(context, options, async (row) => ({
                            recoveryCodes: await Factor.recover(
                                row,
                                context.body.code,
                                environment.crypto,
                            ),
                        })),
                    ),
            ),

            /** Remove the signed-in person's authenticator once a current code proves it. */
            disableFactor: createAuthEndpoint(
                "/two-factor/disable",
                { method: "POST", body: CodeBody, use: [sensitiveSessionMiddleware] },
                async (context) =>
                    refusing(async () => {
                        // check a current code against the person's factor, counting the attempt
                        const person = context.context.session.user;
                        const row = await readFactor(context, person.id);
                        await attempt(context, options, row, async (claimed) => ({
                            lastStep: await Factor.verify(
                                claimed,
                                context.body.code,
                                keyring,
                                environment,
                            ),
                        }));

                        // delete the person's factor
                        await context.context.adapter.deleteMany({
                            model: "factor",
                            where: [{ field: "userId", value: person.id }],
                        });

                        return context.json({ status: true });
                    }),
            ),
        },
        hooks: {
            after: [
                {
                    matcher: (context) =>
                        !EXEMPT_PREFIXES.some(
                            (prefix) => context.path?.startsWith(prefix) === true,
                        ),
                    handler: createAuthMiddleware(async (context) => {
                        // leave a response without a new session, such as a refresh of the request's own
                        const created = context.context.newSession;
                        const sessionCookie = context.context.authCookies.sessionToken.name;
                        const cookies = parseCookies(context.headers?.get("cookie") ?? "");
                        const [incoming] = (cookies.get(sessionCookie) ?? "").split(".");
                        if (created === null || created.session.token === incoming) {
                            return undefined;
                        }

                        // leave a sign-in of a person without a verified factor
                        const verified = await context.context.adapter.findOne({
                            model: "factor",
                            where: [
                                { field: "userId", value: created.user.id },
                                { field: "status", value: "verified" },
                            ],
                        });
                        if (verified === null) {
                            return undefined;
                        }

                        // replace the first factor's session with a challenge for the second
                        deleteSessionCookie(context, true);
                        await context.context.internalAdapter.deleteSession(created.session.token);
                        context.context.setNewSession(null);
                        const issued = await Challenge.issue(environment);
                        await context.context.adapter.create({
                            model: "challenge",
                            data: {
                                userId: created.user.id,
                                digest: issued.digest,
                                expiresAt: issued.expiresAt,
                            },
                        });
                        const cookie = context.context.createAuthCookie(CHALLENGE_COOKIE, {
                            maxAge: Math.floor((issued.expiresAt - environment.clock.now()) / 1000),
                        });
                        await context.setSignedCookie(
                            cookie.name,
                            issued.token,
                            context.context.secret,
                            cookie.attributes,
                        );

                        // send a navigation to the challenge's page, and a request its answer
                        if (isNavigation(context.path)) {
                            const page = new URL(options.challengeUri);
                            const location = context.context.responseHeaders?.get("location");
                            if (location !== null && location !== undefined) {
                                page.searchParams.set("callbackURL", location);
                            }
                            throw context.redirect(page.href);
                        }

                        return context.json({
                            twoFactorRedirect: true,
                            twoFactorMethods: ["totp"],
                        });
                    }),
                },
            ],
        },
    };
}

/** An endpoint context of the second factor's routes, as Better Auth hands its endpoints. */
type FactorContext = Parameters<typeof setSessionCookie>[0];

/** Read a person's enrolled factor, refusing a person without one. */
async function readFactor(
    context: FactorContext,
    userId: string,
): Promise<Select<typeof factor.table>> {
    const row = await context.context.adapter.findOne<Select<typeof factor.table>>({
        model: "factor",
        where: [{ field: "userId", value: userId }],
    });
    if (row === null) {
        throw AccountError.of("FACTOR_MISSING", "the person has no authenticator");
    }

    return row;
}

/** What an accepted code changes on its factor. */
type FactorUse = Partial<
    Pick<Select<typeof factor.table>, "lastStep" | "recoveryCodes" | "status">
>;

/** Check a code against a factor, counting the attempt toward its lockout before the check. */
async function attempt(
    context: FactorContext,
    options: SecondFactorOptions,
    row: Select<typeof factor.table>,
    check: (claimed: Select<typeof factor.table>) => Promise<FactorUse>,
): Promise<void> {
    // refuse a locked factor
    const { adapter } = context.context;
    const now = options.environment.clock.now();
    Factor.requireUnlocked(row, now);

    // claim an attempt below the most failures while unlocked, locking the factor once they run out
    const id = { field: "id", value: row.id } as const;
    const isLapsed = { field: "lockedUntil", operator: "lte", value: now } as const;
    const claimed = await adapter.incrementOne<Select<typeof factor.table>>({
        model: "factor",
        where: [id, { field: "failures", operator: "lt", value: MAX_FAILURES }, isLapsed],
        increment: { failures: 1 },
    });
    if (claimed === null) {
        // lock it once, clearing the failures so its codes count anew once the lock lapses
        await adapter.incrementOne({
            model: "factor",
            where: [id, { field: "failures", operator: "gte", value: MAX_FAILURES }, isLapsed],
            increment: {},
            set: { failures: 0, lockedUntil: now + LOCKOUT_MILLISECONDS },
        });
        throw AccountError.of("FACTOR_LOCKED", "too many wrong codes: try again later");
    }

    // record the accepted code where no other attempt wrote since the claim, clearing the failures
    const used = await check(claimed);
    const written = await adapter.incrementOne({
        model: "factor",
        where: [
            { field: "id", value: row.id },
            { field: "revision", value: claimed.revision },
        ],
        increment: {},
        set: { ...used, failures: 0 },
    });
    if (written === null) {
        throw AccountError.of("ATTEMPT_CONCURRENT", "another code was checked at the same time");
    }
}

/** Answer a sign-in's challenge by checking its person's factor, signing the browser in. */
async function answer(
    context: FactorContext & { readonly body: { readonly code: string } },
    options: SecondFactorOptions,
    check: (row: Select<typeof factor.table>) => Promise<FactorUse>,
) {
    // read the challenge the browser keeps, refusing a lapsed one
    const { adapter, internalAdapter } = context.context;
    const { environment } = options;
    const cookie = context.context.createAuthCookie(CHALLENGE_COOKIE);
    const token = await context.getSignedCookie(cookie.name, context.context.secret);
    if (token === null || token === false) {
        throw AccountError.of("CHALLENGE_EXPIRED", "the sign-in has no second step in progress");
    }
    const digest = await Challenge.digest(token, environment.crypto);
    const pending = await adapter.findOne<Select<typeof challenge.table>>({
        model: "challenge",
        where: [{ field: "digest", value: digest }],
    });
    if (pending === null) {
        throw AccountError.of("CHALLENGE_EXPIRED", "the sign-in has no second step in progress");
    }
    Challenge.require(pending, environment.clock.now());

    // check the code against the person's factor, then spend the challenge and sign the person in
    const row = await readFactor(context, pending.userId);
    await attempt(context, options, row, check);
    await adapter.delete({ model: "challenge", where: [{ field: "id", value: pending.id }] });
    expireCookie(context, cookie);
    const person = present(
        await internalAdapter.findUserById(pending.userId),
        "the challenge's user",
    );
    const session = await internalAdapter.createSession(person.id);
    if (session === null) {
        throw new APIError("INTERNAL_SERVER_ERROR", { message: "failed to create session" });
    }
    await setSessionCookie(context, { session, user: person });

    return context.json({ token: session.token, user: person });
}

/** Report whether a route signs a browser in by navigation, through a link or a provider. */
function isNavigation(path: string | undefined): boolean {
    return path === LINK_PATH || path?.startsWith(CALLBACK_PREFIX) === true;
}
