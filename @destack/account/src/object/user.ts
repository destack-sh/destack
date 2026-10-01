import { principal, relation, union } from "@destack/access";
import { unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { defineSchema, identifier, schema, TimeZone } from "@destack/schema";
import { AccountHandle, HandleAvailability } from "./handle.ts";
import { RESIDENCIES } from "./region.ts";
import { sudo } from "./sudo.ts";

/** The longest text a handle check accepts, beyond which nothing is a handle. */
const HANDLE_INPUT = 256;

/** A BCP 47 language tag, such as en-US. */
export const Locale = defineSchema(
    schema.string().regex(/^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|\d{3}))?$/),
);
/** A BCP 47 language tag. */
export type Locale = schema.Infer<typeof Locale>;

/** A person, the user principal. */
export const user = defineObject({
    name: "user",
    tier: "global",
    plural: "users",
    scope: "universe",
    isScope: true,
    represents: principal.user,
    fields: {
        /** The display name. */
        name: field.string(schema.string().max(200)),
        /** The primary email address, unique across users. */
        email: field.string().guard({ read: "update" }),
        /** Whether the user proved control of the email address. */
        emailVerified: field.boolean().default(false).guard({ read: "update" }),
        /** The profile image URL. */
        image: field.string().optional(),
        /** The login the sign-in provider knows the user by. */
        login: field.string().optional().guard({ read: "update" }),
        /** The language and region the user reads, the OpenID Connect locale claim. */
        locale: field.string(Locale).optional().guard({ read: "update" }),
        /** The time zone the user lives in, the OpenID Connect zoneinfo claim. */
        timeZone: field.string(TimeZone).optional().guard({ read: "update" }),
        /** The jurisdiction the user's home space keeps their data in, chosen with their handle. */
        residency: field.enum(RESIDENCIES).optional().guard({ read: "update" }),
        /** The user's home space. */
        home: field.string(identifier("space")).optional().guard({ read: "update" }),
        /** Whether sign-in requires a second factor. */
        twoFactorEnabled: field.boolean().default(false).guard({ read: "update" }),
        /** When the platform suspended the user, in UTC epoch milliseconds. */
        suspendedAt: field.time().optional().guard({ read: "update" }),
    },
    recoverable: { within: { days: 30 }, by: "delete" },
    relations: {
        self: { subjects: [principal.user] },
        delegate: { subjects: [principal.user], grantedBy: "lend" },
        joined: { subjects: [principal.space], grantedBy: null },
    },
    permissions: {
        read: union(relation("self"), relation("joined")),
        update: relation("self"),
        lend: relation("self"),
        impersonate: relation("delegate"),
        delete: relation("self"),
    },
    shareable: { by: "lend" },
    elevated: { lend: sudo, impersonate: sudo, delete: sudo },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        update: method.update("update", {
            fields: ["name", "image", "locale", "timeZone", "residency"],
        }),
        suggestHandle: method({
            permission: "read",
            mutates: false,
            output: schema.object({ handle: AccountHandle }),
        }),
        checkHandle: method({
            permission: "read",
            mutates: false,
            input: schema.object({
                /** The handle as the user typed it. */
                handle: schema.string().max(HANDLE_INPUT),
            }),
            output: schema.object({
                /** The handle checked. */
                handle: schema.string(),
                /** Whether the handle is free, taken, or not a handle at all. */
                availability: HandleAvailability,
            }),
        }),
    },
    constraints: (user) => [unique("user_email").on(user.email)],
});
/** A persisted user record. */
export type User = Select<typeof user.table>;
