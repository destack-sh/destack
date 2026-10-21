import { principal } from "@destack/access";
import {
    grant,
    type MethodContext,
    type MethodContextOf,
    type NextOf,
    type ResultOf,
    SCOPE_READ,
} from "@destack/object";
import { Identifier, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { CallError } from "@destack/object";
import { Placement } from "../object/placement.ts";
import { account } from "../object/account.ts";
import { organisation } from "../object/organisation.ts";
import { OAuthConsent } from "../object/oauth-consent.ts";
import { Revocation } from "../credential/revocation.ts";
import { Proof } from "@destack/identity";
import { client } from "../object/client.ts";
import { key, Key } from "../object/key.ts";
import { session, type Session } from "../object/session.ts";

/** The longest token lifetime, 366 days in milliseconds so a leap year fits. */
const LIFETIME_MILLISECONDS = 366 * 24 * 60 * 60 * 1000;

/** Implement clients. */
export function implementClient(): typeof client {
    return client.implement({ methods: { create, issue, attach, subscribe, revoke } });
}

/** Register a client as active now, attaching a calling session that runs on no client yet. */
async function create(
    context: MethodContextOf<typeof client, "create">,
    next: NextOf<typeof client, "create">,
): Promise<ResultOf<typeof client, "create">> {
    // create the client
    const created = await next(
        context.with({ input: { ...context.input, lastActiveAt: context.now } }),
    );

    // attach the calling session, none for a client the issuer's pages register for a device
    const { credential } = context.caller;
    const sessionId =
        credential?.kind === "session" ? session.identifier(credential.id) : undefined;
    const calling =
        sessionId === undefined
            ? undefined
            : await context.read(session).findFirst({ where: { id: sessionId } });
    if (sessionId !== undefined && calling?.clientId === null) {
        await context.mutate(context.self, [
            session.call("attach", { id: sessionId, clientId: created.id }),
        ]);
    }

    return created;
}

/** Issue a token client with the person's grant of its permissions to it, until its expiry. */
async function issue(
    context: MethodContextOf<typeof client, "issue">,
    next: NextOf<typeof client, "issue">,
): Promise<ResultOf<typeof client, "issue">> {
    // require an expiry and permissions the person may grant
    const { permissions, expiresAt } = context.input;
    await requireIssuable(context);

    // create the token client, then the person's grant to it in the same transaction
    const created = await next(
        context.with({ input: { ...context.input, kind: "token", lastActiveAt: context.now } }),
    );
    await context.mutate(context.caller, [
        grant.call("create", {
            ...grant.in(context.scope),
            delegate: principal.client.reference(Scope.universe.id, created.id),
            permissions,
            expiresAt,
        }),
    ]);

    return created;
}

/** Attach the calling session to a client by a proof of one of its keys. */
async function attach(
    context: MethodContextOf<typeof client, "attach">,
): Promise<ResultOf<typeof client, "attach">> {
    // find the client's authenticating key the proof was signed with
    const { target } = context;
    Revocation.requireUnrevoked(target, "client");
    const proof = Proof.read(schema.string().parse(context.input.proof));
    const found = await context.read(key).findFirst({
        where: {
            parentPackageId: client.typeReference.packageId,
            parentType: client.name,
            parentId: target.id,
            thumbprint: proof.thumbprint,
            ...Key.standing(context.now),
        },
    });
    if (found === undefined) {
        throw new CallError("FORBIDDEN", {
            message: "key proof refers to no active key of the client",
        });
    }

    // verify the proof and attach the calling session
    await proof.verify(found.publicKey, target.id, context.now);
    await Key.spend(context, proof);
    await context.mutate(context.self, [
        session.call("attach", { id: callingSession(context), clientId: target.id }),
    ]);

    return context.update({ lastActiveAt: context.now });
}

/** Store a browser client's Web Push subscription. */
async function subscribe(
    context: MethodContextOf<typeof client, "subscribe">,
): Promise<ResultOf<typeof client, "subscribe">> {
    // refuse a withdrawn or non-browser client
    const { target } = context;
    Revocation.requireUnrevoked(target, "client");
    if (target.kind !== "browser") {
        throw new CallError("BAD_REQUEST", { message: "only browsers subscribe to push" });
    }

    return context.update({ push: context.input.push });
}

/** Withdraw a client, ending its keys, sessions, grants, consents and OAuth tokens with it. */
async function revoke(
    context: MethodContextOf<typeof client, "revoke">,
): Promise<ResultOf<typeof client, "revoke">> {
    // read the client's standing keys and sessions
    const { target } = context;
    Revocation.requireUnrevoked(target, "client");
    const keys = await Key.unrevoked(context, client, target.id);
    const sessions = await context
        .read(session)
        .findMany({ where: { clientId: target.id, revokedAt: { isNull: true } } });

    // read the person's grants to it
    const delegate = principal.client.reference(Scope.universe.id, target.id);
    const grants = await context
        .read(grant)
        .findMany({ where: ObjectReference.columns("delegate", delegate) });

    // end the keys, sessions, grants and the provider's grants to it, then the client
    await OAuthConsent.withdraw(context, target.id);
    await context.mutate(context.caller, [
        ...keys.map((row) => key.call("revoke", key.target(row))),
        ...sessions.map((row) => session.call("revoke", session.target(row))),
        ...grants.map((row) => grant.call("revoke", grant.target(row))),
    ]);

    return context.update({ revokedAt: context.now });
}

/** Read the session a call authenticated with, and refuse a call without one. */
function callingSession(context: MethodContext): Session["id"] {
    // read the session credential the call presented
    const { credential } = context.caller;
    if (credential?.kind !== "session") {
        throw new CallError("FORBIDDEN", { message: "a client attaches a signed-in session" });
    }

    return session.identifier(credential.id);
}

/** Require a token's expiry within its lifetime and its permissions declared and usable. */
async function requireIssuable(context: MethodContextOf<typeof client, "issue">): Promise<void> {
    // require an expiry in the future within the longest lifetime
    const issued = context.input;
    if (issued.expiresAt <= context.now || issued.expiresAt > context.now + LIFETIME_MILLISECONDS) {
        throw new CallError("BAD_REQUEST", {
            message: "token expiry must lie within 366 days from now",
        });
    }

    // require permissions in one scope each, declared by the served objects
    for (const permission of issued.permissions) {
        if (permission.scope === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `token permission ${permission.type}.${permission.name} names no scope`,
            });
        }
        const served = context.objects.filter(
            (object) => object.typeReference.packageId === permission.packageId,
        );
        const isDeclared =
            served.some(
                (object) =>
                    object.policy.name === permission.type &&
                    object.permissions.includes(permission.name),
            ) ||
            (permission.name === SCOPE_READ &&
                context.objects.some((object) =>
                    object.scopes.some(
                        (scope) =>
                            scope.typeReference.packageId === permission.packageId &&
                            scope.policy.name === permission.type,
                    ),
                ));
        if (served.length > 0 && !isDeclared) {
            throw new CallError("BAD_REQUEST", {
                message: `unknown permission ${permission.type}.${permission.name}`,
            });
        }
    }

    // require each permission's scope to be one the holder may use
    const scopes = [...new Set(issued.permissions.flatMap((permission) => permission.scope ?? []))];
    const usable = await userScopes(context, scopes);
    const unusable = scopes.find((scope) => !usable.includes(scope));
    if (unusable !== undefined) {
        throw new CallError("BAD_REQUEST", { message: `token cannot act in ${unusable}` });
    }
}

/** List the scopes a user's token may act in. */
async function userScopes(context: MethodContext, scopes: readonly string[]): Promise<string[]> {
    // find the named accounts, organisations and spaces
    const accounts = await context
        .read(account)
        .findMany({ where: { id: { in: Identifier.filter("account", scopes) } } });
    const organisations = await context
        .read(organisation)
        .findMany({ where: { id: { in: Identifier.filter("organisation", scopes) } } });
    const spaces = await Placement.find(context, Identifier.filter("space", scopes));

    return [
        context.scope,
        ...[...accounts, ...organisations].map((row) => row.id),
        ...spaces.map((row) => row.id),
    ];
}
