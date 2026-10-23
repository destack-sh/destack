import type { OAuthExtensionGrantHandler } from "@better-auth/oauth-provider";
import type { JWTPayload } from "jose";
import { CONTAINED, principal } from "@destack/access";
import { Scope, type Subject, RequestId } from "@destack/sync";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import { type AssertionKey, AssertionVerifier } from "@destack/access/token";
import { Identity } from "../object/identity.ts";
import { Machine, machine } from "../object/machine.ts";
import { space } from "../object/space.ts";
import { account } from "../object/account.ts";
import { user } from "../object/user.ts";
import { Placement } from "../object/placement.ts";
import accountPackage from "../package.ts";
import { Issuance } from "./issuance.ts";
import { PackageResource, type TokenLifetime } from "./resource.ts";
import { proof } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { Clock } from "@destack/environment";

/** The grant type of a key-signed assertion exchanged for an access token (RFC 7523 2.1). */
export const JWT_BEARER = "urn:ietf:params:oauth:grant-type:jwt-bearer";

/** A jwt-bearer request: the assertion, the package and the space the token acts in. */
const BearerRequest = schema.looseObject({
    /** The key-signed assertion. */
    assertion: schema.string().min(1),
    /** The package the token is for, as its resource (RFC 8707). */
    resource: schema.string().min(1),
    /** The space the machine serves, receives or works in, absent for a call outside spaces. */
    space: schema.identifier("space").exactOptional(),
});

/** What the jwt-bearer grant verifies assertions with. */
export interface BearerOptions {
    /** The account database with the machines' keys and the placements. */
    readonly database: DatabaseConnection;
    /** The account objects, which store the spent proofs. */
    readonly objects: Pick<ObjectStore, "mutate" | "serving">;
    /** The token endpoint's URL, every assertion's audience. */
    readonly endpoint: string;
    /** The clock assertions are verified at. */
    readonly clock: Clock;
    /** How long the packages' tokens live. */
    readonly lifetime: TokenLifetime;
    /** The universe's issuer, which its tokens include. */
    readonly issuer: string;
    /** Sign claims with the universe's active key, as the provider signs its tokens. */
    readonly sign: (claims: JWTPayload, header: { readonly typ: string }) => Promise<string>;
}

/** Exchange a machine's or installation's assertion for a package's access token (RFC 7523). */
export function bearerGrant(options: BearerOptions): OAuthExtensionGrantHandler {
    // verify assertions by the machines' keys that authenticate, spending each once
    const { database } = options;
    const assertions = new AssertionVerifier({
        endpoint: options.endpoint,
        audience: accountPackage.id,
        keys: (thumbprint, issuer, now) => assertionKey(database, thumbprint, issuer, now),
        spend: async (digest, expiresAt, now) => {
            await options.objects.mutate(options.objects.serving(proof), {
                id: RequestId.create(now),
                calls: [
                    proof.call("create", { ...proof.in(Scope.universe.id), id: digest, expiresAt }),
                ],
            });
        },
    });

    return async (input) => {
        // read the request and the package it asks a token for
        const request = BearerRequest.safeParse(input.ctx.body);
        if (!request.success) {
            throw Issuance.refusal("invalid_request", "invalid jwt-bearer request");
        }
        const { assertion, resource, space: spaceId } = request.data;
        const audience = PackageResource.package(resource);
        if (audience === undefined) {
            throw Issuance.refusal("invalid_target", `${resource} names no package`);
        }

        // verify the assertion, refusing it as an invalid grant
        const now = options.clock.now();
        const { caller: asserted, credential } = await assertions
            .verify(assertion, now)
            .catch(refuse);

        // bind a machine to a space it serves, receives or works in
        if (spaceId !== undefined && principal.machine.is(asserted.subject)) {
            const machineId = schema.identifier("machine").parse(asserted.subject.id);
            await Placement.requireWorker(Snapshot.live(database), spaceId, machineId).catch(
                refuse,
            );
        }
        // bind an installation to its space alone
        else if (spaceId !== undefined && asserted.subject.scope !== spaceId) {
            throw Issuance.refusal(
                "invalid_grant",
                `${asserted.subject.id} acts in its space alone`,
            );
        }

        // issue the token toward the package, in the space when asked, with a space's containment in its account
        const scoped = {
            ...credential,
            audience,
            ...(spaceId === undefined ? {} : { scope: spaceId }),
        };
        const groups = [...asserted.groups, ...(await containment(database, asserted.subject))];

        return Issuance.sign(
            input,
            { caller: { ...asserted, groups }, credential: scoped },
            { now, lifetime: options.lifetime, issuer: options.issuer, sign: options.sign },
        );
    };
}

/** Refuse a failed verification as an invalid grant (RFC 6749 5.2). */
function refuse(error: unknown): never {
    throw Issuance.grantRefusal(error);
}

/** Read the key an assertion's issuer signs with: a machine's key, or a space's identity key. */
function assertionKey(
    database: DatabaseConnection,
    thumbprint: string,
    issuer: string,
    now: number,
): Promise<AssertionKey | undefined> {
    // read a machine's authenticating key, or the space's current signing key for its installations
    const snapshot = Snapshot.live(database);
    if (machine.identifies(issuer)) {
        return Machine.assertionKey(snapshot, thumbprint, now);
    } else if (space.identifies(issuer)) {
        return Identity.assertionKey(snapshot, thumbprint, issuer);
    }

    return Promise.resolve(undefined);
}

/** Read a space principal's account and the people it is home to, as the sets it belongs to. */
async function containment(database: DatabaseConnection, subject: Subject): Promise<Subject[]> {
    // take a space principal's account from the directory, none for another principal
    if (!principal.space.is(subject)) {
        return [];
    }
    const snapshot = Snapshot.live(database);
    const row = await snapshot
        .read(space.table)
        .findFirst({ where: { id: space.identifier(subject.id) } });
    const owner =
        row === undefined
            ? undefined
            : await snapshot
                  .read(account.table)
                  .findFirst({ where: { id: account.identifier(row.scope) } });
    if (row === undefined || owner === undefined) {
        throw Issuance.refusal("invalid_grant", `the account of space ${subject.id} is unknown`);
    }

    // state the people whose home the space is, as the universe stores them
    const people = await snapshot
        .read(user.table)
        .findMany({ where: { homeId: space.identifier(subject.id) } });

    return [
        { ...account.reference(owner.scope, owner.id), relation: CONTAINED },
        ...people.map((person) => ({
            ...principal.user.reference(Scope.universe.id, person.id),
            relation: "home",
        })),
    ];
}
