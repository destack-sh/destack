import { schema } from "@destack/schema";

/** The longest subject line, in characters: RFC 5322 2.1.1's line length. */
const SUBJECT_LENGTH = 998;

/** The longest text or HTML body, in characters: 64 KiB keeps one message small. */
const BODY_LENGTH = 64 * 1024;

/** The email address a message goes to. */
export const EmailDestination = schema.object({
    channel: schema.literal("email"),
    /** The email address. */
    address: schema.email(),
});

/** An email's subject and bodies. */
export const EmailContent = schema.object({
    channel: schema.literal("email"),
    /** The subject. */
    subject: schema.string().min(1).max(SUBJECT_LENGTH),
    /** The plain text body. */
    text: schema.string().max(BODY_LENGTH),
    /** The HTML body, absent for plain text alone. */
    html: schema.string().max(BODY_LENGTH).exactOptional(),
});
