import type { Call, CallOf, NextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { OAuthConsent } from "../object/oauth-consent.ts";
import { Revocation } from "../credential/revocation.ts";
import { Proof } from "@destack/identity";
import { client } from "../object/client.ts";
import { key, Key } from "../object/key.ts";
import { session, type Session } from "../object/session.ts";

/** Implement clients. */
export function implementClient(): typeof client {
    return client.implement({ methods: { create, attach, subscribe, revoke } });
}

/** Register a client as active now, attaching a calling session that runs on no client yet. */
async function create(
    call: CallOf<typeof client, "create">,
    next: NextOf<typeof client, "create">,
): Promise<ResultOf<typeof client, "create">> {
    // create the client
    const created = await next(call.with({ input: { ...call.input, lastActiveAt: call.now } }));

    // attach the calling session, none for a client the issuer's pages register for a device
    const { credential } = call.caller;
    const sessionId =
        credential?.kind === "session" ? session.identifier(credential.id) : undefined;
    const calling =
        sessionId === undefined
            ? undefined
            : await call.read(session).findFirst({ where: { id: sessionId } });
    if (sessionId !== undefined && calling?.clientId === null) {
        await call.mutate(call.self, [
            session.call("attach", { id: sessionId, clientId: created.id }),
        ]);
    }

    return created;
}

/** Attach the calling session to a client by a proof of one of its keys. */
async function attach(
    call: CallOf<typeof client, "attach">,
): Promise<ResultOf<typeof client, "attach">> {
    // find the client's authenticating key the proof was signed with
    const { target } = call;
    Revocation.requireUnrevoked(target, "client");
    const proof = Proof.read(schema.string().parse(call.input.proof));
    const found = await call.read(key).findFirst({
        where: {
            parentPackageId: client.typeReference.packageId,
            parentType: client.name,
            parentId: target.id,
            thumbprint: proof.thumbprint,
            ...Key.standing(call.now),
        },
    });
    if (found === undefined) {
        throw new CallError("FORBIDDEN", {
            message: "key proof refers to no active key of the client",
        });
    }

    // verify the proof and attach the calling session
    await proof.verify(found.publicKey, target.id, call.now);
    await Key.spend(call, proof);
    await call.mutate(call.self, [
        session.call("attach", { id: callingSession(call), clientId: target.id }),
    ]);

    return call.update({ lastActiveAt: call.now });
}

/** Keep a browser client's Web Push subscription. */
async function subscribe(
    call: CallOf<typeof client, "subscribe">,
): Promise<ResultOf<typeof client, "subscribe">> {
    // refuse a withdrawn or non-browser client
    const { target } = call;
    Revocation.requireUnrevoked(target, "client");
    if (target.kind !== "browser") {
        throw new CallError("BAD_REQUEST", { message: "only browsers subscribe to push" });
    }

    return call.update({ push: call.input.push });
}

/** Withdraw a client, ending its keys, sessions, consents and OAuth tokens with it. */
async function revoke(
    call: CallOf<typeof client, "revoke">,
): Promise<ResultOf<typeof client, "revoke">> {
    // read the client's standing keys and sessions
    const { target } = call;
    Revocation.requireUnrevoked(target, "client");
    const keys = await Key.unrevoked(call, client, target.id);
    const sessions = await call
        .read(session)
        .findMany({ where: { clientId: target.id, revokedAt: { isNull: true } } });

    // end the keys, sessions and the provider's grants to it, then the client
    await OAuthConsent.withdraw(call, target.id);
    await call.mutate(call.caller, [
        ...keys.map((row) => key.call("revoke", key.target(row))),
        ...sessions.map((row) => session.call("revoke", session.target(row))),
    ]);

    return call.update({ revokedAt: call.now });
}

/** Read the session a call authenticated with, and refuse a call without one. */
function callingSession(call: Call): Session["id"] {
    // read the session credential the call presented
    const { credential } = call.caller;
    if (credential?.kind !== "session") {
        throw new CallError("FORBIDDEN", { message: "a client attaches a signed-in session" });
    }

    return session.identifier(credential.id);
}
