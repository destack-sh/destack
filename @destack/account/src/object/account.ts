import {
    CONTAINED,
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
import { Scope, type ObjectReference, type Subject } from "@destack/sync";
import { Snapshot, type Select, type RelationalQueryBuilder } from "@destack/db";
import { Claim, claimTable, defineObject, field, type MethodContext } from "@destack/object";
import { Identifier, schema } from "@destack/schema";
import { Label } from "../address/label.ts";
import { AccountHandle, HANDLE_LENGTH, type HandleAvailability } from "../address/handle.ts";
import { organization } from "./organization.ts";
import { Residency, residency, ResidencyCode } from "./residency.ts";
import { StackSource } from "./stack-source.ts";
import { user } from "./user.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";
import { AccountError } from "../error/error.ts";

/** The name of every account's home space. */
export const HOME_SPACE = "home";

/** An account, the scope of its spaces. */
export const account = defineObject(
    {
        name: "account",
        plural: "accounts",
        scope: [user, organization],
        isScope: true,
        fields: {
            /** The globally unique handle scoping the account's package names. */
            handle: field.string(AccountHandle),
            /** The display name. */
            name: field.string(DisplayName),
            /** The jurisdiction the account's spaces store data in. */
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
            relay: { subjects: [residency.members("relaying")], grantedBy: "place" },
            installed: { subjects: [residency.members("installed")], grantedBy: "place" },
            // the account's home space, and the installations living in it
            home: { subjects: [principal.space, principal.space.members(CONTAINED)] },
            // the people the account's managers let publish packages under its name
            publisher: { subjects: [principal.user], grantedBy: "manage" },
        },
        permissions: {
            // inherit from the organization: its members view and its administrators manage
            member: through("organization", "view"),
            read: union(
                through("user", "self"),
                contained(principal.space, principal.machine),
                through("organization", "view"),
            ),
            comment: through("organization", "manage"),
            edit: through("organization", "manage"),
            manage: through("organization", "manage"),
            bill: union(permission("manage"), through("organization", "bill")),
            list: union(permission("read"), permission("serve")),
            replicate: union(contained(principal.space), relation("resident")),
            // let the residency's servers and installations, the account's home and its machines serve it
            serve: union(
                relation("resident"),
                relation("home"),
                contained(principal.machine),
                relation("installed"),
            ),
            // let the residency's installations set the account's entitlements
            entitle: relation("installed"),
            // let the residency's machines and home claim names in the account
            claim: relation("claimant"),
            // let the residency's relay publish the tunnels of the account's machines
            relaying: relation("relay"),
            // let the person or the organization's managers found accounts in their scope
            create: union(through("user", "self"), through("organization", "manage")),
            update: permission("manage"),
            delete: permission("manage"),
            // let the account's managers and its publishers publish packages under the account's name
            publish: union(permission("manage"), relation("publisher")),
            administer: permission("manage"),
            impersonate: permission("manage"),
            verify: permission("manage"),
            place: none(),
            // let the account's home declare and provision the account's own resources
            provision: relation("home"),
        },
        shareable: {},
        reserved: ["administer", "replicate", "serve", "entitle", "claim", "place", "provision"],
        elevated: { delete: sudo, administer: sudo, impersonate: sudo },
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
            fields: ["handle", "name", "kind", "residencyId"],
            input: schema.object({
                /** The stack of the account's home. */
                stack: StackSource.exactOptional(),
            }),
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["handle", "name"] }),
    }),
);
/** A persisted account record. */
export type Account = Select<typeof account.table>;

/** The residency, relations and payers of accounts. */
export const Account = {
    /** Read the account of a scope: a person's personal one, an organization's payer, or itself. */
    async of(reader: Pick<Snapshot, "read">, scope: string): Promise<Account> {
        // refuse a scope without an account
        const found = await findAccount(reader, scope);
        if (found === undefined) {
            throw AccountError.of("ACCOUNT_NOT_FOUND", `${scope} has no account`);
        }

        return found;
    },

    /** Read the account paying for a scope: an organization's account is paid by its payer. */
    async payer(reader: Pick<Snapshot, "read">, scope: string): Promise<Identifier<"account">> {
        const owned = await Account.of(reader, scope);
        const payer = organization.identifies(owned.scope)
            ? await Account.of(reader, owned.scope)
            : owned;

        return payer.id;
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

    /** Read the person whose home a space is: its personal account's owner, absent otherwise. */
    async homeOwner(reader: Pick<Snapshot, "read">, spaceId: string): Promise<Subject | undefined> {
        // read the account the space is the home of, answering a personal account's person
        const id = Account.ofHome(spaceId);
        const owner = await reader.read(account.table).findFirst({ where: { id } });

        return owner?.kind === "personal"
            ? user.reference(Scope.universe.id, owner.scope)
            : undefined;
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

    /** Read the home space of the account a scope belongs to. */
    async homeOf(reader: Pick<Snapshot, "read">, scope: string): Promise<Identifier<"space">> {
        return Account.home((await Account.of(reader, scope)).id);
    },

    /** Locate the space of the residency the account a scope belongs to stores its data in. */
    async locate(reader: Pick<Snapshot, "read">, scope: string): Promise<Identifier<"space">> {
        return Residency.space(reader, (await Account.of(reader, scope)).residencyId);
    },

    /** Read an account's installations' relationship to its residency as residents. */
    resident(row: Pick<Account, "id" | "scope" | "residencyId">) {
        return {
            object: residency.reference(Scope.universe.id, row.residencyId),
            relation: "resident",
            subject: { ...account.reference(row.scope, row.id), relation: CONTAINED },
        };
    },

    /** Relate a new account's owner, home and residency servers. */
    async creation(context: MethodContext, object: ObjectReference): Promise<Creation> {
        // read the residency the account stores data in
        const residencyId = ResidencyCode.parse(context.input["residencyId"]);

        // relate the residency's servers, claimants, relay and installations, and the account's home
        const resident = { relation: "resident", subject: Residency.servers(residencyId) };
        const claimant = { relation: "claimant", subject: Residency.claimants(residencyId) };
        const relay = { relation: "relay", subject: Residency.relays(residencyId) };
        const installed = { relation: "installed", subject: Residency.installations(residencyId) };
        const homed = principal.space.reference(Scope.universe.id, Account.home(object.id));
        const home = { relation: "home", subject: homed };
        const inside = { relation: "home", subject: { ...homed, relation: CONTAINED } };

        // leave an organization's account to the organization's members
        const scope = await Scope.object(context, context.scope);
        if (organization.is(scope)) {
            return { relationships: [resident, claimant, relay, installed, home, inside] };
        }

        // require a creator of a user's account
        const creator = context.caller.subject;
        if (creator === undefined) {
            throw AccountError.of("CREATOR_REQUIRED", "account needs a creator");
        }

        return {
            relationships: [
                { relation: "owner", subject: creator },
                resident,
                claimant,
                relay,
                installed,
                home,
                inside,
            ],
            owner: { ...object, relation: "owner" },
        };
    },
};

/** Find the account of a scope, absent for a scope without one. */
async function findAccount(
    reader: Pick<Snapshot, "read">,
    scope: string,
): Promise<Account | undefined> {
    // read an account itself
    if (account.identifies(scope)) {
        return reader.read(account.table).findFirst({ where: { id: account.identifier(scope) } });
    }
    // read an organization's payer
    else if (organization.identifies(scope)) {
        const owner = await reader
            .read(organization.table)
            .findFirst({ where: { id: organization.identifier(scope) } });

        return owner === undefined
            ? undefined
            : reader.read(account.table).findFirst({ where: { id: owner.payer } });
    }
    // read a person's personal account
    else if (user.identifies(scope)) {
        return reader.read(account.table).findFirst({
            where: { scope: user.identifier(scope), kind: "personal" },
        });
    }
    // find none for any other scope
    else {
        return undefined;
    }
}
