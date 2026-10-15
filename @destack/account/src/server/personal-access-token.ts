import { type GrantPermission } from "@destack/access";
import { type Call, type CallOf, type NextOf, type ResultOf, SCOPE_READ } from "@destack/object";
import { Identifier } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Placement } from "../object/placement.ts";
import { account } from "../object/account.ts";
import { organisation } from "../object/organisation.ts";
import { grant } from "../object/grant.ts";
import { personalAccessToken } from "../object/personal-access-token.ts";

/** The longest token lifetime, 366 days in milliseconds so a leap year fits. */
const LIFETIME_MILLISECONDS = 366 * 24 * 60 * 60 * 1000;

/** Implement personal access tokens. */
export function implementPersonalAccessToken(): typeof personalAccessToken {
    return personalAccessToken.implement({ methods: { issue } });
}

/** Issue a token with the holder's grant of its permissions to it, until its expiry. */
async function issue(
    call: CallOf<typeof personalAccessToken, "issue">,
    next: NextOf<typeof personalAccessToken, "issue">,
): Promise<ResultOf<typeof personalAccessToken, "issue">> {
    // require an expiry and permissions the user may grant
    const { permissions, expiresAt } = call.input;
    await requireIssuable(call, call.input, await userScopes(call, scopesOf(call.input)));

    // create the token, then the holder's grant to it in the same transaction
    const created = await next(call);
    await call.mutate(call.caller, [
        grant.call("create", {
            ...grant.in(call.scope),
            delegate: personalAccessToken.reference(call.scope, created.id),
            permissions,
            expiresAt,
        }),
    ]);

    return created;
}

/** The token fields an issue checks: its expiry and permissions. */
interface TokenRequest {
    /** The expiry, in UTC epoch milliseconds. */
    readonly expiresAt: number;
    /** The permissions the token's grant gives. */
    readonly permissions: readonly GrantPermission[];
}

/** List the scopes a token's permissions apply in, once each. */
function scopesOf(issued: TokenRequest): string[] {
    return [...new Set(issued.permissions.map((permission) => permission.scope))];
}

/** Require a token's expiry within its lifetime, and its permissions declared. */
async function requireIssuable(
    call: Call,
    issued: TokenRequest,
    usable: readonly string[],
): Promise<void> {
    // require an expiry in the future within the longest lifetime
    if (issued.expiresAt <= call.now || issued.expiresAt > call.now + LIFETIME_MILLISECONDS) {
        throw new CallError("BAD_REQUEST", {
            message: "token expiry must lie within 366 days from now",
        });
    }

    // require permissions declared by the served objects
    for (const permission of issued.permissions) {
        const served = call.objects.filter(
            (object) => object.typeReference.packageId === permission.packageId,
        );
        const isDeclared =
            served.some(
                (object) =>
                    object.policy.name === permission.type &&
                    object.permissions.includes(permission.name),
            ) ||
            (permission.name === SCOPE_READ &&
                call.objects.some((object) =>
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
    const unusable = scopesOf(issued).find((scope) => !usable.includes(scope));
    if (unusable !== undefined) {
        throw new CallError("BAD_REQUEST", { message: `token cannot act in ${unusable}` });
    }
}

/** List the scopes a user's token may act in. */
async function userScopes(
    call: Call<typeof personalAccessToken.table>,
    scopes: readonly string[],
): Promise<string[]> {
    // find the named accounts, organisations and spaces
    const accounts = await call
        .read(account)
        .findMany({ where: { id: { in: Identifier.filter("account", scopes) } } });
    const organisations = await call
        .read(organisation)
        .findMany({ where: { id: { in: Identifier.filter("organisation", scopes) } } });
    const spaces = await Placement.find(call, Identifier.filter("space", scopes));

    return [
        call.scope,
        ...[...accounts, ...organisations].map((row) => row.id),
        ...spaces.map((row) => row.id),
    ];
}
