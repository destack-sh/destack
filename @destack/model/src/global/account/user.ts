import { boolean, integer, recordColumns, type Select, table, text } from "@destack/db";

/** User records. */
export const user = table("user", {
    ...recordColumns("user"),
    /** The display name. */
    name: text("name").notNull(),
    /** The canonical sign-in email address. */
    email: text("email").notNull().unique(),
    /** Whether the sign-in email has been verified. */
    emailVerified: boolean("email_verified").notNull().default(false),
    /** The profile image URL. */
    image: text("image"),
    /** Whether sign-in requires a second factor. */
    twoFactorEnabled: boolean("two_factor_enabled").notNull().default(false),
    /** The time platform access was suspended. */
    suspendedAt: integer("suspended_at"),
    /** The time the user requested deletion. */
    deletionRequestedAt: integer("deletion_requested_at"),
});

/** A persisted user record. */
export type User = Select<typeof user>;
