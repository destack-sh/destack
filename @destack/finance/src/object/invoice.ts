import { account } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { Currency } from "../rate/amount.ts";

/** The fields the provider writes on an invoice. */
const INVOICE_FIELDS = [
    "number",
    "status",
    "currency",
    "subtotal",
    "tax",
    "total",
    "amountPaid",
    "periodStart",
    "periodEnd",
    "hostedUrl",
    "pdfUrl",
    "issuedAt",
    "providerId",
] as const;

/** A bill the provider issued to an account, mirrored as the provider reports it. */
export const invoice = defineObject({
    name: "invoice",
    plural: "invoices",
    scope: account,
    fields: {
        /** The number the invoice carries once finalized, absent on a draft. */
        number: field.string(schema.string().min(1)).optional(),
        /** Where the invoice stands at the provider. */
        status: field.enum(["draft", "open", "paid", "void", "uncollectible"]),
        /** The currency of the amounts. */
        currency: field.string(Currency),
        /** The amount before tax, in the currency's minor unit. */
        subtotal: field.integer(),
        /** The tax. */
        tax: field.integer(),
        /** The amount due. */
        total: field.integer(),
        /** The amount paid so far. */
        amountPaid: field.integer(),
        /** When the billed period started. */
        periodStart: field.time().optional(),
        /** When the billed period ended. */
        periodEnd: field.time().optional(),
        /** The provider's page showing and paying the invoice. */
        hostedUrl: field.string(schema.url()).optional(),
        /** The provider's PDF of the invoice. */
        pdfUrl: field.string(schema.url()).optional(),
        /** When the provider issued the invoice. */
        issuedAt: field.time(),
        /** The provider's identifier of the invoice. */
        providerId: field.string(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true, fields: INVOICE_FIELDS }),
        update: method.update(null, { isSystem: true, fields: INVOICE_FIELDS }),
    }),
});
/** A persisted invoice. */
export type Invoice = Select<typeof invoice.table>;
