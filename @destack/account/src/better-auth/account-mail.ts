import { html } from "./markup.ts";

/** An email an account receives: its subject, plain text and HTML. */
export type MailContent = {
    /** The subject. */
    readonly subject: string;
    /** The plain text body. */
    readonly text: string;
    /** The HTML body. */
    readonly html: string;
};

/** One email to one address, under the key a provider deduplicates by. */
export type Mail = MailContent & {
    /** The address. */
    readonly to: string;
    /** The key the provider deduplicates by. */
    readonly key: string;
    /** The origination time, in milliseconds since the epoch. */
    readonly date: number;
};

/** What a transport made of one email: sent, or refused for now or for good. */
export type MailOutcome =
    | { readonly kind: "sent" }
    | { readonly kind: "deferred" | "failed"; readonly refusal: { readonly message: string } };

/** A way to send account email, as message's mail transports do. */
export interface MailTransport {
    /** Send one email, reading what the transport made of it. */
    readonly send: (mail: Mail) => Promise<MailOutcome>;
}

/** What each kind of one-use code is for, as its mail states it. */
export const CODE_PURPOSES = {
    "sign-in": "sign-in",
    "email-verification": "verification",
    "forget-password": "password reset",
    "change-email": "email change",
} as const;

/** The emails an account receives: a sign-in link, or a one-use code. */
export const AccountMail = {
    /** Compose the email with a sign-in link. */
    link(url: string): MailContent {
        return {
            subject: "Sign in to Destack",
            text: `Sign in to Destack with this link:\n\n${url}\n\nIf you did not ask to sign in, ignore this email.`,
            html: html`<p>Sign in to Destack with this link:</p>
                <p><a href="${url}">Sign in</a></p>
                <p>If you did not ask to sign in, ignore this email.</p>`.text,
        };
    },

    /** Compose the email with a one-use code. */
    code(otp: string, type: keyof typeof CODE_PURPOSES): MailContent {
        return {
            subject: "Your Destack code",
            text: `Your Destack ${CODE_PURPOSES[type]} code is ${otp}.\n\nIf you did not ask for it, ignore this email.`,
            html: html`<p>Your Destack ${CODE_PURPOSES[type]} code is <strong>${otp}</strong>.</p>
                <p>If you did not ask for it, ignore this email.</p>`.text,
        };
    },
};
