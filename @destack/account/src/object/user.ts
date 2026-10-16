import { anyone, Invitation, principal, relation, union } from "@destack/access";
import {
    and,
    type DatabaseConnection,
    eq,
    Expression,
    isNull,
    type Select,
    type SQL,
} from "@destack/db";
import { defineObject, field } from "@destack/object";
import { LocaleTag } from "@destack/locale";
import { type Identifier, schema, TimeZone } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Subject } from "@destack/sync";
import { AccountHandle, HandleAvailability } from "../address/handle.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";
import { Recipient } from "./recipient.ts";

/** The longest text a handle check accepts, beyond which nothing is a handle. */
const HANDLE_INPUT_LENGTH = 256;

/** A person, the user principal. */
export const user = defineObject(
    {
        name: "user",
        plural: "users",
        scope: "universe",
        isScope: true,
        represents: principal.user,
        fields: {
            /** The display name. */
            name: field.string(DisplayName),
            /** The primary email address, unique across users. */
            email: field.string(schema.email()).guard({ read: "self" }),
            /** Whether the user proved control of the email address. */
            emailVerified: field.boolean().default(false).guard({ read: "self" }),
            /** The profile image URL. */
            image: field.string().optional(),
            /** The login the sign-in provider knows the user by. */
            login: field.string().optional().guard({ read: "self" }),
            /** The time zone the user lives in, the OpenID Connect zoneinfo claim. */
            timeZone: field.string(TimeZone).guard({ read: "self" }),
            /** The language and region the user reads, public so spaces write to them in it. */
            locale: field.string(LocaleTag),
            /** Whether sign-in requires a second factor. */
            twoFactorEnabled: field.boolean().default(false).guard({ read: "self" }),
            /** When the platform suspended the user, in UTC epoch milliseconds. */
            suspendedAt: field.time().optional().guard({ read: "self" }),
            /** The home space of the person's personal account, acting for them. */
            home: field.reference(principal.space),
        },
        recoverable: { within: { days: 30 }, by: "delete" },
        relations: {
            self: { subjects: [principal.user] },
            // the person's home space, acting for them in their own scope
            home: { subjects: [principal.space] },
            delegate: { subjects: [principal.user], grantedBy: "delegate" },
            public: { subjects: [anyone.all()] },
        },
        permissions: {
            // anyone reads a person's public profile, their own private fields alone
            read: union(relation("self"), relation("public")),
            // the person's private fields
            self: relation("self"),
            // the person, their home, and through roles any machine
            home: union(relation("self"), relation("home")),
            // the person's home, following replicas on their behalf
            follow: relation("home"),
            // the person's home, reading where the person receives notifications
            recipient: relation("home"),
            update: relation("self"),
            delegate: relation("self"),
            impersonate: relation("delegate"),
            delete: relation("self"),
        },
        shareable: { by: "delegate" },
        elevated: { delegate: sudo, impersonate: sudo, delete: sudo },
        constraints: {
            email: { kind: "unique", on: ["email"] },
        },
        convert: {
            // fill an empty name from the email's local part, which a replica without the email takes from its home
            "2026.9.0": {
                name: Expression.case(
                    Expression.column("name"),
                    [
                        {
                            when: "",
                            then: Expression.coalesce(
                                Expression.before(Expression.column("email"), "@"),
                                Expression.column("name"),
                            ),
                        },
                    ],
                    Expression.column("name"),
                ),
            },
        },
    },
    (method) => ({
        update: method.update("update", {
            fields: ["name", "image", "timeZone", "locale"],
        }),
        suggestHandle: method.query({
            permission: "read",
            output: schema.object({ handle: AccountHandle }),
        }),
        invited: method.query({
            permission: "update",
            output: schema.object({
                /** The pending invitations to the user or the user's verified email. */
                items: schema.array(Invitation.schema),
            }),
        }),
        /** Read where the person receives notifications beyond an inbox, and in which language. */
        recipient: method.query({ permission: "recipient", output: Recipient }),
        /** Read the home space of the person's personal account. */
        home: method.query({
            permission: "home",
            output: schema.identifier("space"),
        }),
        checkHandle: method.query({
            permission: "read",
            input: schema.object({
                /** The handle as the user typed it. */
                handle: schema.string().max(HANDLE_INPUT_LENGTH),
            }),
            output: schema.object({
                /** The handle checked. */
                handle: schema.string(),
                /** Whether the handle is free, taken, or not a handle at all. */
                availability: HandleAvailability,
            }),
        }),
    }),
);
/** A persisted user record. */
export type User = Select<typeof user.table>;

/** Users: their access at birth, their identification and their standing. */
export const User = {
    /** Build a person's access at birth: their scope's owner, shown to anyone. */
    creation(self: Subject) {
        return {
            owner: self,
            relationships: [{ relation: "public", subject: anyone.reference("*", "*") }],
        };
    },

    /** Read the user a subject key names, refusing another principal's key. */
    identify(key: string): Identifier<"user"> {
        const subject = Subject.read(key);
        if (!principal.user.is(subject)) {
            throw new CallError("NOT_FOUND", { message: `${key} is no person` });
        }

        return schema.identifier("user").parse(subject.id);
    },

    /** Match the active users: unsuspended and undeleted. */
    isActive(): SQL {
        return and(isNull(user.table.suspendedAt), isNull(user.table.deletedAt));
    },

    /** Report whether a user may sign in: unsuspended and undeleted. */
    async maySignIn(database: DatabaseConnection, userId: string): Promise<boolean> {
        const [active] = await database
            .select({ id: user.table.id })
            .from(user.table)
            .where(and(eq(user.table.id, schema.identifier("user").parse(userId)), User.isActive()))
            .limit(1);

        return active !== undefined;
    },
};
