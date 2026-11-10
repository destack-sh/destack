import type { AuthContext } from "better-auth";
import { AccessContext, accessGrant, Caller, Grant, principal } from "@destack/access";
import { type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import type { Environment } from "@destack/environment";
import { grant } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import { ObjectReference, RequestId, Scope, type Subject } from "@destack/sync";
import { ConsentList } from "../authentication/request.ts";
import { ActiveSession } from "../better-auth/active-session.ts";
import { AccountError } from "../error/error.ts";
import { oauthConsent } from "../object/oauth-consent.ts";
import { user } from "../object/user.ts";
import { requireOrigin, requireQuery } from "./authorization.ts";
import { OAuthClient } from "./party.ts";

/** What the consents endpoint reads and writes with. */
export interface ConsentEndpointOptions {
    /** The universe's database. */
    readonly database: DatabaseConnection;
    /** The account objects, which revoke as the person. */
    readonly objects: Pick<ObjectStore, "mutate">;
    /** The clock and randomness. */
    readonly environment: Environment;
    /** Read the session a request carries, absent without one. */
    readonly session: (headers: Headers) => Promise<string | undefined>;
    /** The origins a revocation may come from. */
    readonly context: Promise<Pick<AuthContext, "isTrustedOrigin">>;
}

/** The signed-in person's consents, which their list of apps reads and revokes. */
export class ConsentEndpoint {
    /** The endpoint's dependencies. */
    readonly #options: ConsentEndpointOptions;

    /** Answer the person's list of apps. */
    constructor(options: ConsentEndpointOptions) {
        this.#options = options;
    }

    /** List the person's consents, or revoke one with its grant from a trusted origin. */
    async fetch(request: Request): Promise<Response> {
        // read the person signed in
        const userId = await this.#person(request.headers);

        // list their consents
        if (request.method === "GET") {
            return Response.json(ConsentList.parse({ consents: await this.#list(userId) }));
        }
        // revoke one by its client, from a trusted origin
        else if (request.method === "DELETE") {
            await requireOrigin(request, this.#options.context);
            await this.#revoke(userId, requireQuery(request, "client_id"));

            return new Response(null, { status: 204 });
        }

        return new Response(null, { status: 405, headers: { allow: "GET, DELETE" } });
    }

    /** Read the signed-in person, refusing a request without a session. */
    async #person(headers: Headers): Promise<Identifier<"user">> {
        // read the session the request carries
        const { session, database, environment } = this.#options;
        const id = await session(headers);
        const active =
            id === undefined
                ? undefined
                : await ActiveSession.read(id, database, environment.clock.now());

        // refuse a request without one
        if (active === undefined) {
            throw AccountError.of("SIGN_IN_REQUIRED", "the person is not signed in");
        }

        return user.identifier(active.userId);
    }

    /** List a person's consents with their clients' names, newest first. */
    async #list(userId: Identifier<"user">) {
        // read the person's consents
        const { database } = this.#options;
        const consents = await Snapshot.live(database)
            .read(oauthConsent.table)
            .findMany({ where: { scope: userId } });

        // name each consent's client and its live grant
        const listed = await Promise.all(
            consents.map(async (consent) => {
                // read the client, and how long the consent's grant lasts
                const party = await OAuthClient.find(consent.clientId, database);
                const asked = consent.grant ?? undefined;
                const lifetime =
                    asked === undefined ? undefined : await this.#lifetime(userId, asked.delegate);

                return {
                    client: { id: party.clientId, name: party.name },
                    scopes: consent.scopes,
                    grantedAt: consent.grantedAt,
                    grant:
                        asked === undefined || lifetime === undefined
                            ? null
                            : { ...asked, lifetime },
                };
            }),
        );

        // order them newest first, then by client
        return listed.toSorted(
            (first, second) =>
                second.grantedAt - first.grantedAt ||
                first.client.id.localeCompare(second.client.id),
        );
    }

    /** Read how long a person's grant to a delegate lasts, absent once it ended. */
    async #lifetime(
        userId: Identifier<"user">,
        delegate: Subject,
    ): Promise<"lasting" | "sign-in" | undefined> {
        // find the unexpired grant, which a sign-in's end revokes
        const { database, environment } = this.#options;
        const now = environment.clock.now();
        const view = Snapshot.live(database);
        const person = principal.user.reference(Scope.universe.id, userId);
        const found = await Grant.find(view, { delegator: person, delegate }, now);
        if (found === undefined) {
            return undefined;
        }

        return found.sessionId === null ? "lasting" : "sign-in";
    }

    /** Revoke a person's consent to a client and the grant it gave, in one commit. */
    async #revoke(userId: Identifier<"user">, clientId: string): Promise<void> {
        // find the consent, refusing a client the person never consented to
        const { database, objects, environment } = this.#options;
        const now = environment.clock.now();
        const view = Snapshot.live(database);
        const consent = await view
            .read(oauthConsent.table)
            .findFirst({ where: { clientId, scope: userId } });
        if (consent === undefined) {
            throw AccountError.of("CONSENT_NOT_FOUND", "this account never allowed this app");
        }

        // find the grant the consent gave its installation
        const person = principal.user.reference(Scope.universe.id, userId);
        const granted =
            consent.grant === undefined || consent.grant === null
                ? undefined
                : await view.read(accessGrant).findFirst({
                      where: {
                          ...ObjectReference.columns("delegator", person),
                          ...ObjectReference.columns("delegate", consent.grant.delegate),
                      },
                  });

        // revoke both as the person
        const calls = [
            oauthConsent.call("revoke", oauthConsent.target(consent)),
            ...(granted === undefined ? [] : [grant.call("revoke", grant.target(granted))]),
        ];
        await Transaction.run(database, async (transaction) => {
            await objects.mutate(
                AccessContext.of(Caller.of(person), now),
                { id: RequestId.create(environment), calls },
                { transaction },
            );
        });
    }
}
