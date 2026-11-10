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

/** What each kind of one-use code is for, as its mail states it. */
export const CODE_PURPOSES = {
    "sign-in": "sign-in",
    "email-verification": "verification",
    "forget-password": "password reset",
    "change-email": "email change",
} as const;

/** The emails an account receives: a sign-in code with its link, or another one-use code. */
export const AccountMail = {
    /** Compose the email with a sign-in code, and a link entering it on the page it opens. */
    signIn(otp: string, link: URL): MailContent {
        return {
            subject: `Your Destack code is ${otp}`,
            text: `Your Destack sign-in code is ${otp}.\n\nOr continue with this link:\n\n${link.href}\n\nIf you did not ask to sign in, ignore this email.`,
            html: html`<p>Your Destack sign-in code is <strong>${otp}</strong>.</p>
                <p>Or <a href="${link.href}">continue with this link</a>.</p>
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
