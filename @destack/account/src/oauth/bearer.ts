import { CONTAINED, principal } from "@destack/access";
import { Scope, type Subject } from "@destack/sync";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { schema } from "@destack/schema";
import { type AssertionKey, AssertionVerifier, PartitionId } from "@destack/access/token";
import { Identity } from "../object/identity.ts";
import { Machine, machine } from "../object/machine.ts";
import { space } from "../object/space.ts";
import { account } from "../object/account.ts";
import { user } from "../object/user.ts";
import { Placement } from "../object/placement.ts";
import accountPackage from "../package.ts";
import { OAuthError } from "../authentication/token.ts";
import { AccessToken, type TokenGrant } from "./token.ts";
import { PackageResource } from "./resource.ts";
import type { Crypto, Environment } from "@destack/environment";

/** The grant type of a key-signed assertion exchanged for an access token (RFC 7523 2.1). */
export const JWT_BEARER = "urn:ietf:params:oauth:grant-type:jwt-bearer";

/** The identifier schema of spaces, the partitions a machine works in beside its own. */
const SPACE_IDENTIFIER = schema.identifier("space");

/** A jwt-bearer request: the assertion, the package and the partition the token acts in. */
const BearerRequest = schema.looseObject({
    /** The key-signed assertion. */
    assertion: schema.string().min(1),
    /** The package the token is for, as its resource (RFC 8707). */
    resource: schema.string().min(1),
    /** The partition the call goes to: a space the machine serves, its own partition, or none. */
    partition: PartitionId.exactOptional(),
});

/** What the jwt-bearer grant verifies assertions with. */
export interface BearerOptions {
    /** The account database with the machines' keys and the placements. */
    readonly database: DatabaseConnection;
    /** The token endpoint's URL, every assertion's audience. */
    readonly endpoint: string;
    /** The clock assertions are verified at, and the WebCrypto verifying them. */
    readonly environment: Environment;
}

/** Exchange a machine's or installation's assertion for a package's access token (RFC 7523). */
export function bearerGrant(options: BearerOptions): TokenGrant {
    // verify assertions by the machines' keys that authenticate, spending each once
    const { database } = options;
    const assertions = new AssertionVerifier({
        endpoint: options.endpoint,
        audience: accountPackage.id,
        keys: (thumbprint, issuer, now) =>
            assertionKey(database, thumbprint, issuer, now, options.environment.crypto),
        crypto: options.environment.crypto,
        database,
    });

    return async ({ body }) => {
        // read the request and the package it asks a token for
        const request = BearerRequest.safeParse(body);
        if (!request.success) {
            throw OAuthError.of("INVALID_REQUEST", "invalid jwt-bearer request");
        }
        const { assertion, resource, partition } = request.data;
        const audience = PackageResource.package(resource);
        if (audience === undefined) {
            throw OAuthError.of("INVALID_TARGET", `${resource} names no package`);
        }

        // verify the assertion, refusing it as an invalid grant
        const now = options.environment.clock.now();
        const { caller: asserted, credential } = await assertions.verify(assertion, now);

        // take the universe or a machine's own partition as they are
        const isMachine = principal.machine.is(asserted.subject);
        const isOpen = partition === undefined || (isMachine && partition === asserted.subject.id);
        const spaceId = SPACE_IDENTIFIER.safeParse(partition);

        // bind a machine to a space it serves, receives or works in
        if (!isOpen && isMachine && spaceId.success) {
            const machineId = schema.identifier("machine").parse(asserted.subject.id);
            await Placement.requireWorker(Snapshot.live(database), spaceId.data, machineId);
        }
        // refuse a machine any other partition
        else if (!isOpen && isMachine) {
            throw OAuthError.of(
                "INVALID_GRANT",
                `${asserted.subject.id} acts in its own partition and its spaces alone`,
            );
        }
        // bind an installation to its space alone
        else if (!isOpen && asserted.subject.scope !== partition) {
            throw OAuthError.of("INVALID_GRANT", `${asserted.subject.id} acts in its space alone`);
        }

        // issue the token for the partition the call goes to, else the universe, lapsing with the token alone
        //  as an assertion's expiry bounds only its presentation (RFC 7523)
        const { scope: _asserted, ...unscoped } = credential;
        const scoped = {
            ...unscoped,
            audience,
            expiresAt: Number.POSITIVE_INFINITY,
            ...(partition === undefined ? {} : { scope: partition }),
        };

        // add a space's containment to the caller's groups
        const groups = [...asserted.groups, ...(await containment(database, asserted.subject))];
        const authentication = { caller: { ...asserted, groups }, credential: scoped };

        return {
            authentication,
            clientId: AccessToken.client(authentication),
            scopes: [],
            resource,
        };
    };
}

/** Read the key an assertion's issuer signs with: a machine's key, or a space's identity key. */
function assertionKey(
    database: DatabaseConnection,
    thumbprint: string,
    issuer: string,
    now: number,
    crypto: Crypto,
): Promise<AssertionKey | undefined> {
    // read a machine's authenticating key, or the space's current signing key for its installations
    const snapshot = Snapshot.live(database);
    if (machine.identifies(issuer)) {
        return Machine.assertionKey(snapshot, thumbprint, now);
    } else if (space.identifies(issuer)) {
        return Identity.assertionKey(snapshot, thumbprint, issuer, crypto);
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
        throw OAuthError.of("INVALID_GRANT", `the account of space ${subject.id} is unknown`);
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
