import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { type Call, defineObject, field } from "@destack/object";
import { ServiceError } from "@destack/service/error";
import { defineSchema, schema, type Identifier } from "@destack/schema";

/** The longest name or address line a provider keeps. */
const LINE_LENGTH = 200;

/** A country by its ISO 3166-1 alpha-2 code. */
export const Country = defineSchema(schema.string().regex(/^[A-Z]{2}$/u));

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
        type: schema.string().regex(/^[a-z]{2}_[a-z_]+$/u),
        /** The identifier. */
        value: schema.string().min(1).max(LINE_LENGTH),
    }),
);

/** The permissions of an account's billing objects. */
export const BILLING_PERMISSIONS = {
    read: through("account", "read"),
    bill: through("account", "bill"),
    provide: through("account", "serve"),
};

/** An account as a buyer: the details its invoices show. */
export const customer = defineObject({
    name: "customer",
    plural: "customers",
    scope: space,
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
        /** The address invoices go to. */
        email: field.string(schema.email()).optional(),
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
    constraints: {
        account: { kind: "index", on: ["accountId"], unique: true },
        provider: {
            kind: "index",
            on: ["providerId"],
            unique: true,
            where: { providerId: { isNotNull: true } },
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("bill", {
        fields: ["accountId", "email", "name", "address", "taxIds"],
    }),
    update: method.update("bill", { fields: ["email", "name", "address", "taxIds"] }),
    /** Record the provider's identifier of the customer. */
    record: method.update("provide", { isInternal: true, fields: ["providerId"] }),
}));
/** A persisted customer. */
export type Customer = Select<typeof customer.table>;

/** The accounts customers bill. */
export const Customer = {
    /** Require an account's customer before the account subscribes or buys. */
    async require(call: Pick<Call, "read">, accountId: Identifier<"account">): Promise<void> {
        const [row] = await call.read(customer).rows({ accountId });
        if (row === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `account ${accountId} has no customer`,
            });
        }
    },
};
