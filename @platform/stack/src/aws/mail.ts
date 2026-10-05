import { Mailbox, type MimeMessage } from "@destack/mail/mime";
import { SesClient } from "@destack/mail/aws";

/** The settings that send mail through SES, by environment name. */
const SETTINGS: readonly string[] = [
    "DESTACK_MAIL_REGION",
    "DESTACK_MAIL_FROM",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
];

/** A sender mailing composed messages through SES from one address. */
export class SesSender {
    /** The sender's address, with its display name. */
    readonly from: string;
    /** The line naming the delivery. */
    readonly description: string;
    /** The SES client with the process's AWS credentials. */
    readonly #client: SesClient;

    /** Keep a client sending as one address. */
    private constructor(client: SesClient, from: string, region: string) {
        this.#client = client;
        this.from = from;
        this.description = `through SES in ${region} from ${from}`;
    }

    /** Read the SES settings from the environment, refusing a partial set. */
    static read(environment: Readonly<Record<string, string | undefined>>): SesSender {
        // require every setting
        const read = (name: string) => {
            const value = environment[name];

            return value === undefined || value === "" ? undefined : value;
        };
        const region = read("DESTACK_MAIL_REGION");
        const from = read("DESTACK_MAIL_FROM");
        const accessKeyId = read("AWS_ACCESS_KEY_ID");
        const secretAccessKey = read("AWS_SECRET_ACCESS_KEY");
        if (
            region === undefined ||
            from === undefined ||
            accessKeyId === undefined ||
            secretAccessKey === undefined
        ) {
            const missing = SETTINGS.filter((name) => read(name) === undefined);
            throw new Error(`mail through SES needs ${missing.join(", ")} as well`);
        }

        // send as the sender, with the process's AWS credentials
        const sessionToken = read("AWS_SESSION_TOKEN");
        const credentials = {
            accessKeyId,
            secretAccessKey,
            ...(sessionToken === undefined ? {} : { sessionToken }),
        };
        const client = new SesClient({ region, credentials: async () => credentials });

        return new SesSender(client, from, region);
    }

    /** Send a composed message to one address. */
    async send(to: string, message: MimeMessage): Promise<void> {
        await this.#client.send(
            { sender: Mailbox.parse(this.from).address, recipients: [to] },
            message,
        );
    }
}
