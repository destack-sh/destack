import { none, through } from "@destack/access";
import { account, organisation } from "@destack/account/object";
import { eq, uniqueIndex, type DatabaseConnection, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { defineSchema, schema, type Identifier } from "@destack/schema";

/** The longest name or address line a provider keeps. */
const LINE_LENGTH = 200;

/** A country by its ISO 3166-1 alpha-2 code. */
export const Country = defineSchema(schema.string().regex(/^[A-Z]{2}$(?![\s\S])/u));

/** A postal address invoices and taxes read. */
export const Address = defineSchema(
    schema.object({
        /** The street and number. */
        line1: schema.string().min(1).max(LINE_LENGTH),
        /** The apartment, suite or building. */
        line2: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
        /** The city or town. */
        city: schema.string().min(1).max(LINE_LENGTH),
        /** The postal code. */
        postalCode: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
        /** The state, province or region. */
        state: schema.string().min(1).max(LINE_LENGTH).exactOptional(),
        /** The country. */
        country: Country,
    }),
);

/** A tax identifier of a customer, such as an EU VAT number. */
export const TaxId = defineSchema(
    schema.object({
        /** The identifier's type, such as eu_vat. */
        type: schema.string().regex(/^[a-z]{2}_[a-z_]+$(?![\s\S])/u),
        /** The identifier. */
        value: schema.string().min(1).max(LINE_LENGTH),
    }),
);

/** The permissions of an account's billing objects: its readers read them and its managers bill. */
export const BILLING_PERMISSIONS = {
    read: through("account", "read"),
    bill: through("account", "bill"),
    provide: none(),
};

/** An account as a buyer: the details its invoices show. */
export const customer = defineObject({
    name: "customer",
    plural: "customers",
    scope: account,
    fields: {
        /** The address invoices go to. */
        email: field.string(schema.email()),
        /** The name invoices show. */
        name: field.string(schema.string().min(1).max(LINE_LENGTH)),
        /** The billing address. */
        address: field.json(Address).optional(),
        /** The tax identifiers invoices show. */
        taxIds: field.json(schema.array(TaxId)).default([]),
        /** The provider's identifier of the customer. */
        providerId: field.string().optional(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("bill", { fields: ["email", "name", "address", "taxIds"] }),
        update: method.update("bill", { fields: ["email", "name", "address", "taxIds"] }),
        /** Record the provider's identifier of the customer. */
        record: method.update(null, { isSystem: true, fields: ["providerId"] }),
        /** Derive the account's entitlements from its subscriptions, purchases and meter events. */
        entitle: method.mutation({ permission: null, isSystem: true }),
    }),
    constraints: (entry) => [uniqueIndex("customer_account").on(entry.scope)],
});
/** A persisted customer. */
export type Customer = Select<typeof customer.table>;

/** The accounts customers bill. */
export const Customer = {
    /** Require an account's customer before the account subscribes or buys. */
    async require(database: DatabaseConnection, accountId: Identifier<"account">): Promise<void> {
        const [row] = await database
            .select({ id: customer.table.id })
            .from(customer.table)
            .where(eq(customer.table.scope, accountId));
        if (row === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `account ${accountId} has no customer`,
            });
        }
    },

    /** Read the account paying an account's charges: its organisation's payer, else the account itself. */
    async payer(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
    ): Promise<Identifier<"account">> {
        // read the account's owner from its copy
        const [copied] = await database
            .select({ scope: account.table.scope })
            .from(account.table)
            .where(eq(account.table.id, accountId));
        if (copied === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no account ${accountId}` });
        }

        // let a user's account pay for itself
        const organisationId = schema.identifier("organisation").safeParse(copied.scope);
        if (!organisationId.success) {
            return accountId;
        }

        // read the organisation's payer from its copy, the account itself without one
        const [owner] = await database
            .select({ payer: organisation.table.payer })
            .from(organisation.table)
            .where(eq(organisation.table.id, organisationId.data));
        if (owner === undefined) {
            throw new ServiceError("NOT_FOUND", { message: `no organisation ${copied.scope}` });
        }

        return owner.payer ?? accountId;
    },
};
