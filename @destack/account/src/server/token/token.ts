import type { Restriction } from "@destack/access";
import { inArray, type Table } from "@destack/db";
import { type Call, type Handler, SCOPE_READ } from "@destack/object";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { DirectoryDatabase, type Zone } from "@destack/directory";
import { account } from "../../object/account.ts";
import { organisation } from "../../object/organisation.ts";
import { ServiceAccountStanding } from "../../object/service.ts";
import * as base from "../../object/token.ts";

/** The longest token lifetime, 366 days in milliseconds so a leap year fits. */
const LIFETIME_MILLISECONDS = 366 * 24 * 60 * 60 * 1000;

/** Personal access tokens on the server, restricted to existing scopes that their user's access narrows further. */
export const personalAccessToken = base.personalAccessToken.handle({
    issue: issue(userScopes),
});

/** Service tokens on the server. */
export const serviceToken = base.serviceToken.handle({ issue: issue(serviceAccountScopes) });

/** Issue a token after checking its expiry and restrictions. */
function issue<Definition extends Table>(
    reached: (call: Call<Definition>, scopes: readonly string[]) => Promise<string[]>,
): Handler<Definition> {
    return async (call, next) => {
        // require an expiry in the future within the longest lifetime
        const input = call.input as {
            readonly expiresAt: number;
            readonly restrictions: Restriction[];
        };
        if (input.expiresAt <= call.now || input.expiresAt > call.now + LIFETIME_MILLISECONDS) {
            throw new ServiceError("BAD_REQUEST", {
                message: "token expiry must lie within 366 days from now",
            });
        }

        // require restrictions to refer to declared permissions
        for (const restriction of input.restrictions) {
            const served = call.objects.filter(
                (object) => object.policy.definition.packageId === restriction.packageId,
            );
            const isDeclared =
                served.some(
                    (object) =>
                        object.policy.name === restriction.type &&
                        object.permissions.includes(restriction.name),
                ) ||
                (restriction.name === SCOPE_READ &&
                    call.objects.some((object) =>
                        object.scopes.some(
                            (scope) =>
                                scope.policy.definition.packageId === restriction.packageId &&
                                scope.policy.name === restriction.type,
                        ),
                    ));
            if (served.length > 0 && !isDeclared) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `unknown permission ${restriction.type}.${restriction.name}`,
                });
            }
        }

        // require each restriction's scope to be one the holder reaches
        const scopes = [...new Set(input.restrictions.map((restriction) => restriction.scope))];
        const found = new Set(await reached(call, scopes));
        const unreached = scopes.find((scope) => !found.has(scope));
        if (unreached !== undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `token cannot act in ${unreached}`,
            });
        }

        return next();
    };
}

/** List the scopes a user's token may restrict itself to: its own and the existing accounts, organisations and spaces. */
async function userScopes(
    call: Call<typeof base.personalAccessToken.table>,
    scopes: readonly string[],
): Promise<string[]> {
    // find the named accounts, organisations and spaces
    const database = call.database;
    const accounts = await database
        .select({ id: account.table.id })
        .from(account.table)
        .where(inArray(account.table.id, identifiers("account", scopes)));
    const organisations = await database
        .select({ id: organisation.table.id })
        .from(organisation.table)
        .where(inArray(organisation.table.id, identifiers("organisation", scopes)));
    const spaces = await zones(call, scopes);

    return [call.scope, ...[...accounts, ...organisations, ...spaces].map((row) => row.id)];
}

/** List the scopes a service account's token may restrict itself to: its own and its account's spaces. */
async function serviceAccountScopes(
    call: Call<typeof base.serviceToken.table>,
    scopes: readonly string[],
): Promise<string[]> {
    // require the holding service account to stand
    const holder = identifier("service-account").parse(call.parent()!.id);
    const scope = identifier("account").parse(call.scope);
    const standing = await ServiceAccountStanding.read(scope, holder, call.database);
    if (standing === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "service account not found" });
    } else if (standing.revokedAt !== null) {
        throw new ServiceError("CONFLICT", { message: "service account is revoked" });
    }

    // find the named spaces of the account
    const spaces = (await zones(call, scopes)).filter((zone) => zone.scope === scope);

    return [call.scope, ...spaces.map((zone) => zone.id)];
}

/** Locate the named spaces in the directory. */
async function zones(call: Call, scopes: readonly string[]): Promise<Zone[]> {
    const directory = new DirectoryDatabase(call.database);
    const located = await Promise.all(
        identifiers("space", scopes).map((scope) => directory.locate(scope)),
    );

    return located.filter((zone) => zone !== undefined);
}

/** Keep the scopes that are identifiers of one prefix, typed as such. */
function identifiers<const Prefix extends string>(prefix: Prefix, scopes: readonly string[]) {
    const validator = identifier(prefix);

    return scopes.flatMap((scope) => {
        const parsed = validator.safeParse(scope);

        return parsed.success ? [parsed.data] : [];
    });
}
