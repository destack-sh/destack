import {
    contained,
    none,
    permission,
    principal,
    relation,
    through,
    union,
    universe,
    type Creation,
} from "@destack/access";
import { Scope, type ObjectReference } from "@destack/sync";
import { Snapshot, type Select, type RelationalQueryBuilder } from "@destack/db";
import {
    Claim,
    claimTable,
    defineObject,
    defineShape,
    field,
    type MethodContext,
} from "@destack/object";
import { Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Label } from "../address/label.ts";
import { AccountHandle, HANDLE_LENGTH, type HandleAvailability } from "../address/handle.ts";
import { organisation } from "./organisation.ts";
import { Residency, residency, ResidencyCode, type ResidencyFound } from "./residency.ts";
import { SpaceStack } from "./space-stack.ts";
import { user } from "./user.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";

/** The name of every account's home space. */
export const HOME_SPACE = "home";

/** An account, the scope of its spaces. */
export const account = defineObject(
    {
        name: "account",
        plural: "accounts",
        scope: [user, organisation],
        isScope: true,
        fields: {
            /** The globally unique handle scoping the account's package names. */
            handle: field.string(AccountHandle),
            /** The display name. */
            name: field.string(DisplayName),
            /** The jurisdiction the account's spaces keep data in. */
            residency: field.reference(residency, { delete: "restrict" }),
            /** Whether the account is its user's personal account or a shared one. */
            kind: field.enum(["personal", "shared"]).default("shared"),
        },
        attributes: { kind: "string" },
        indexes: { handle: { on: ["handle"], across: () => universe } },
        recoverable: { within: { days: 30 }, by: "delete" },
        relations: {
            resident: { subjects: [residency.members("server")], grantedBy: "place" },
            claimant: { subjects: [residency.members("claim")], grantedBy: "place" },
            home: { subjects: [principal.space] },
        },
        permissions: {
            // inherit from the organisation: its members view and its administrators manage
            member: through("organisation", "view"),
            read: union(
                through("user", "self"),
                contained(principal.space, principal.machine),
                through("organisation", "view"),
            ),
            comment: through("organisation", "manage"),
            edit: through("organisation", "manage"),
            manage: through("organisation", "manage"),
            bill: union(permission("manage"), through("organisation", "bill")),
            list: union(permission("read"), permission("serve")),
            replicate: union(contained(principal.space), relation("resident")),
            // let the residency's servers, the account's home and the account's own machines serve it
            serve: union(relation("resident"), relation("home"), contained(principal.machine)),
            // let the residency's platform installations claim names in the account
            claim: relation("claimant"),
            // let the person or the organisation's managers found accounts in their scope
            create: union(through("user", "self"), through("organisation", "manage")),
            update: permission("manage"),
            delete: permission("manage"),
            own: permission("manage"),
            impersonate: permission("manage"),
            verify: permission("manage"),
            place: none(),
        },
        shareable: {},
        reserved: ["own", "replicate", "serve", "claim", "place"],
        elevated: { delete: sudo, own: sudo, impersonate: sudo },
        constraints: {
            handle: { kind: "index", on: ["handle"] },
            scope: { kind: "index", on: ["scope"] },
            personal: { kind: "index", on: ["scope"], unique: true, where: { kind: "personal" } },
            kind: {
                kind: "check",
                condition: { OR: [{ kind: "shared" }, { scope: { startsWith: "user-" } }] },
            },
            handleForm: { kind: "check", condition: { handle: Label.condition(HANDLE_LENGTH) } },
        },
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("create", {
            fields: ["handle", "name", "kind"],
            input: schema.object({
                /** The jurisdiction the account keeps data in, its owner's when absent. */
                residencyId: ResidencyCode.exactOptional(),
                /** The stack of the account's home. */
                stack: SpaceStack.exactOptional(),
            }),
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["handle", "name"] }),
    }),
);
/** A persisted account record. */
export type Account = Select<typeof account.table>;

/** The shape replicating every account of a residency to its home, which serves them. */
export const residentShape = defineShape({
    name: "residents",
    parameters: schema.object({
        /** The residency whose accounts are replicated. */
        residency: ResidencyCode,
    }),
    objects: [account],
    queries: ({ residency: residencyId }) => [
        account.query({ scopes: "every", where: { residencyId } }),
    ],
});

/** The residency, relations and payers of accounts. */
export const Account = {
    /** Read the account paying for a scope. */
    async payer(reader: Pick<Snapshot, "read">, scope: string): Promise<Identifier<"account">> {
        // read an account's owning scope, or take the organisation itself
        const accountId = schema.identifier("account").safeParse(scope);
        const owner = accountId.success
            ? (await reader.read(account.table).findFirst({ where: { id: accountId.data } }))?.scope
            : scope;
        if (owner === undefined) {
            throw new CallError("NOT_FOUND", { message: `no account ${scope}` });
        }

        // let an account its user owns pay for itself
        const organisationId = schema.identifier("organisation").safeParse(owner);
        if (accountId.success && !organisationId.success) {
            return accountId.data;
        }

        // read the organisation's payer
        const paying = organisationId.success
            ? await reader
                  .read(organisation.table)
                  .findFirst({ where: { id: organisationId.data } })
            : undefined;
        if (paying === undefined) {
            throw new CallError("NOT_FOUND", { message: `no payer of ${scope}` });
        }

        return paying.payer;
    },

    /** Report whether a handle is free, or claimed by an account, a trashed one too. */
    async availability(
        claims: RelationalQueryBuilder<typeof claimTable>,
        handle: string,
    ): Promise<HandleAvailability> {
        // refuse a malformed handle
        if (!AccountHandle.safeParse(handle).success) {
            return "invalid";
        }

        // find an account claiming it
        const { index, key } = Claim.key(account, "handle", [handle]);
        const claimed = await Claim.claimed(claims, { index, keys: [key] });

        return claimed.has(key) ? "taken" : "available";
    },

    /** Derive the identifier of a person's personal account from the person's UUID. */
    personal(userId: string): Identifier<"account"> {
        return Identifier.derive(
            "account",
            Identifier.uuid(schema.identifier("user").parse(userId)),
        );
    },

    /** Derive the identifier of an account's home space from the account's UUID. */
    home(accountId: string): Identifier<"space"> {
        return Identifier.derive(
            "space",
            Identifier.uuid(schema.identifier("account").parse(accountId)),
        );
    },

    /** Derive the identifier of the account a home space belongs to, as `home` derives the home. */
    ofHome(spaceId: string): Identifier<"account"> {
        return Identifier.derive(
            "account",
            Identifier.uuid(schema.identifier("space").parse(spaceId)),
        );
    },

    /** Read the residency a new account keeps data in. */
    async residency(
        reader: Pick<Snapshot, "read">,
        scope: string,
        chosen: ResidencyCode | undefined,
    ): Promise<ResidencyCode> {
        // require a chosen residency to exist
        if (chosen !== undefined) {
            return (await Residency.require(reader, chosen)).id;
        }

        // read the residency of the owning organisation's payer
        const owner = await Scope.object(reader, scope);
        let owned: Pick<Account, "residencyId"> | null | undefined;
        if (organisation.is(owner)) {
            const id = schema.identifier("organisation").parse(owner.id);
            const owning = await reader.read(organisation.table).findFirst({ where: { id } });
            owned =
                owning === undefined
                    ? undefined
                    : await reader.read(account.table).findFirst({ where: { id: owning.payer } });
        }
        // read the residency of the owning user's personal account
        else {
            const person = schema.identifier("user").parse(owner.id);
            [owned] = await reader
                .read(account.table)
                .findMany({ where: { scope: person, kind: "personal" } });
        }

        // refuse an owner without such an account
        if (owned === null || owned === undefined) {
            throw new CallError("NOT_FOUND", { message: `no account owner ${owner.id}` });
        }

        return owned.residencyId;
    },

    /** Find the residency an account keeps its data in, none for an unknown account. */
    async findResidency(
        reader: Pick<Snapshot, "read">,
        accountId: Identifier<"account">,
    ): Promise<ResidencyFound | null> {
        const kept = await reader.read(account.table).findFirst({ where: { id: accountId } });
        const found =
            kept === undefined
                ? undefined
                : await reader.read(residency.table).findFirst({ where: { id: kept.residencyId } });

        return found === undefined ? null : { id: found.id, name: found.name, home: found.home };
    },

    /** Read the home space of a person's or organisation's paying account, or an account's. */
    async homeOf(reader: Pick<Snapshot, "read">, owner: string): Promise<Identifier<"space">> {
        // read a person's personal account
        let found: Pick<Account, "id"> | null | undefined;
        if (user.identifies(owner)) {
            [found] = await reader.read(account.table).findMany({
                where: {
                    scope: user.identifier(owner),
                    kind: "personal",
                },
            });
        }
        // read an organisation's payer
        else if (organisation.identifies(owner)) {
            const paying = await reader.read(organisation.table).findFirst({
                where: {
                    id: organisation.identifier(owner),
                },
            });
            found = paying === undefined ? null : { id: paying.payer };
        }
        // read the account itself
        else {
            found = await reader
                .read(account.table)
                .findFirst({ where: { id: account.identifier(owner) } });
        }

        // refuse an owner without an account
        if (found === null || found === undefined) {
            throw new CallError("NOT_FOUND", { message: `no ${owner}` });
        }

        return Account.home(found.id);
    },

    /** Relate a new account's owner, home and residency servers. */
    async creation(context: MethodContext, object: ObjectReference): Promise<Creation> {
        // resolve the residency before the handler runs
        const chosen = context.input["residencyId"];
        const residencyId = await Account.residency(
            context,
            context.scope,
            chosen === undefined ? undefined : ResidencyCode.parse(chosen),
        );

        // relate the servers of the residency, its platform installations claiming names, and the home
        const resident = { relation: "resident", subject: Residency.servers(residencyId) };
        const claimant = { relation: "claimant", subject: Residency.claimants(residencyId) };
        const home = {
            relation: "home",
            subject: principal.space.reference(Scope.universe.id, Account.home(object.id)),
        };

        // leave an organisation's account to the organisation's members
        const scope = await Scope.object(context, context.scope);
        if (organisation.is(scope)) {
            return { relationships: [resident, claimant, home] };
        }

        // require a creator of a user's account
        const creator = context.caller.subject;
        if (creator === undefined) {
            throw new CallError("FORBIDDEN", { message: "account needs a creator" });
        }

        return {
            relationships: [{ relation: "owner", subject: creator }, resident, claimant, home],
            owner: { ...object, relation: "owner" },
        };
    },
};
