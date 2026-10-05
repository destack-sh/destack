import { MimeMessage } from "@destack/mail/mime";
import { SesSender } from "../aws/mail.ts";

/** The settings whose absence prints sign-in mail, by environment name. */
const SETTINGS = ["DESTACK_MAIL_REGION", "DESTACK_MAIL_FROM"];

/** What each kind of one-use code is for, as its mail names it. */
const PURPOSES = {
    "sign-in": "sign-in",
    "email-verification": "verification",
    "forget-password": "password reset",
    "change-email": "email change",
};

/** How the universe delivers sign-in links and codes. */
export interface SignInMail {
    /** Deliver a sign-in link. */
    readonly sendMagicLink: (message: { email: string; url: string }) => Promise<void>;
    /** Deliver a one-use code. */
    readonly sendCode: (message: {
        email: string;
        otp: string;
        type: keyof typeof PURPOSES;
    }) => Promise<void>;
    /** The line the universe logs naming the delivery. */
    readonly description: string;
}

/** A sender mailing composed messages from one address. */
interface MailSender {
    /** The sender's address, with its display name. */
    readonly from: string;
    /** The line naming the delivery. */
    readonly description: string;
    /** Send a composed message to one address. */
    send(to: string, message: MimeMessage): Promise<void>;
}

/** Send sign-in mail through SES when the mail settings are present, or print it for the local universe. */
export function signInMail(environment: Readonly<Record<string, string | undefined>>): SignInMail {
    // print links and codes without any mail setting
    const isUnset = SETTINGS.every((name) => {
        const value = environment[name];

        return value === undefined || value === "";
    });
    if (isUnset) {
        return {
            sendMagicLink: async ({ email, url }) => {
                process.stdout.write(`sign-in link for ${email}: ${url}\n`);
            },
            sendCode: async ({ email, otp, type }) => {
                process.stdout.write(`${type} code for ${email}: ${otp}\n`);
            },
            description: `mail: printing sign-in links and codes here, since ${SETTINGS.join(" and ")} are unset`,
        };
    }

    return composedMail(SesSender.read(environment));
}

/** Compose sign-in links and codes as messages a sender mails. */
function composedMail(sender: MailSender): SignInMail {
    // compose each message as the sender
    const send = async (to: string, subject: string, text: string, html: string) => {
        const message = await MimeMessage.compose({
            from: sender.from,
            to: [to],
            subject,
            date: new Date(),
            key: crypto.randomUUID(),
            text,
            html,
        });
        await sender.send(to, message);
    };

    return {
        sendMagicLink: ({ email, url }) =>
            send(
                email,
                "Sign in to Destack",
                `Sign in to Destack with this link:\n\n${url}\n\nIf you did not ask to sign in, ignore this email.`,
                `<p>Sign in to Destack with this link:</p><p><a href="${escape(url)}">Sign in</a></p><p>If you did not ask to sign in, ignore this email.</p>`,
            ),
        sendCode: ({ email, otp, type }) =>
            send(
                email,
                "Your Destack code",
                `Your Destack ${PURPOSES[type]} code is ${otp}.\n\nIf you did not ask for it, ignore this email.`,
                `<p>Your Destack ${PURPOSES[type]} code is <strong>${escape(otp)}</strong>.</p><p>If you did not ask for it, ignore this email.</p>`,
            ),
        description: `mail: sending sign-in links and codes ${sender.description}`,
    };
}

/** Escape text for an HTML attribute or element. */
function escape(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
}
