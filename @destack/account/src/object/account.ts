import {
    condition,
    intersection,
    principal,
    relation,
    through,
    union,
    type Creation,
} from "@destack/access";
import { Condition } from "@destack/db/query";
import { Scope, type ObjectReference } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import { check, dialectSQL, index, sql, unique, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field, method, type Call } from "@destack/object";
import { identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { AccountHandle } from "./handle.ts";
import { organisation } from "./organisation.ts";
import { region, Residency, RESIDENCIES } from "./region.ts";
import { user } from "./user.ts";
import { sudo } from "./sudo.ts";

/** An account, the scope of its spaces. */
export const account = defineObject({
    name: "account",
    tier: "global",
    plural: "accounts",
    scope: [user, organisation],
    isScope: true,
    fields: {
        /** The globally unique handle scoping the account's package names. */
        handle: field.string(AccountHandle),
        /** The display name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The residency new spaces take. */
        defaultResidency: field.enum(RESIDENCIES).guard({ read: "use" }),
        /** Whether the account is its user's own or shared. */
        kind: field.enum(["personal", "shared"]).default("shared"),

        // the account-wide policies, held in the region administering them
        /** The package admission policy every space of the account inherits. */
        packagePolicyId: field
            .string(identifier("package-policy"))
            .optional()
            .guard({ read: "use" }),
        /** The region holding the package policy. */
        packagePolicyRegion: field
            .reference(region, { delete: "restrict" })
            .optional()
            .guard({ read: "use" }),
        /** The network policy every space of the account inherits. */
        networkPolicyId: field
            .string(identifier("network-policy"))
            .optional()
            .guard({ read: "use" }),
        /** The region holding the network policy. */
        networkPolicyRegion: field
            .reference(region, { delete: "restrict" })
            .optional()
            .guard({ read: "use" }),
    },
    attributes: { kind: "string" },
    recoverable: { within: { days: 30 }, by: "delete" },
    relations: {
        root: { subjects: [user, organisation.members("owner")], grantedBy: "own" },
        member: { subjects: [user] },
        host: { subjects: [principal.host.all()], grantedBy: null },
    },
    permissions: {
        read: union(
            relation("root"),
            relation("member"),
            relation("host"),
            intersection(condition(Condition.eq("kind", "personal")), through("user", "read")),
        ),
        use: union(relation("root"), relation("member"), relation("host")),
        replicate: relation("host"),
        update: relation("root"),
        delete: relation("root"),
        share: relation("root"),
        own: relation("root"),
        impersonate: relation("root"),
        verify: relation("root"),
    },
    shareable: { by: "share" },
    reserved: ["own", "replicate"],
    elevated: { delete: sudo, own: sudo, impersonate: sudo },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("update", {
            fields: ["handle", "name", "kind"],
            input: schema.object({
                /** The residency new spaces take, its owner's when absent. */
                defaultResidency: Residency.optional(),
            }),
            creation: rootAccount,
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["handle", "name", "defaultResidency"] }),
    },
    constraints: (account) => [
        unique("account_handle_unique").on(account.handle),
        index("account_scope").on(account.scope),
        uniqueIndex("account_personal")
            .on(account.scope)
            .where(sql`${account.kind} = 'personal'`),
        check("account_kind", sql`${account.kind} = 'shared' OR ${account.scope} LIKE 'user-%'`),
        check(
            "account_network_policy",
            sql`(${account.networkPolicyId} IS NULL) = (${account.networkPolicyRegion} IS NULL)`,
        ),
        check(
            "account_package_policy",
            sql`(${account.packagePolicyId} IS NULL) = (${account.packagePolicyRegion} IS NULL)`,
        ),
        check(
            "account_handle",
            dialectSQL({
                sqlite: sql`length(${account.handle}) BETWEEN 1 AND 63 AND ${account.handle} NOT GLOB '*[^a-z0-9-]*' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
                postgresql: sql`length(${account.handle}) BETWEEN 1 AND 63 AND (${account.handle} COLLATE "C") !~ '[^a-z0-9-]' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
            }),
        ),
    ],
});
/** A persisted account record. */
export type Account = Select<typeof account.table>;

/** Root a new account at its creator or its organisation's owners. */
async function rootAccount(call: Call, object: ObjectReference): Promise<Creation> {
    // require a creator
    const creator = call.caller;
    if (creator === undefined) {
        throw new ServiceError("FORBIDDEN", { message: "account needs a creator" });
    }

    // root it at the creator of a user's account, or at the owners of an organisation's
    const owner = await Scope.object(Snapshot.live(call.database), call.scope);
    const root = owner.type === organisation.name ? { ...owner, relation: "owner" } : creator;

    return {
        relationships: [
            { relation: "root", subject: root },
            { relation: "host", subject: principal.host.reference(object.id, "*") },
        ],
        owner: { ...object, relation: "root" },
    };
}
