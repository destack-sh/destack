import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { PrintTransport } from "../print/print.ts";
import type { AwsCredentials } from "@destack/identity/aws";
import { SesClient, SesTransport } from "../ses/index.ts";
import type { MailTransport } from "../transport/index.ts";

/** Where a deployment sends its mail: through SES, or printed to its runtime's own log. */
const mailSender = schema.discriminatedUnion("kind", [
    schema.object({
        /** Send through Amazon SES. */
        kind: schema.literal("ses"),
        /** The AWS region the mail is signed for, such as eu-central-1. */
        region: schema.string().min(1),
        /** The mailbox the mail comes from, such as `Destack <notices@destack.app>`. */
        from: schema.string().min(1),
    }),
    schema.object({
        /** Print each email to the runtime's own log, as a stage without real recipients does. */
        kind: schema.literal("print"),
    }),
]);

/** Where a deployment sends its mail: through SES, or printed to its runtime's own log. */
export const MailSender = Object.assign(mailSender, {
    /** Open the transport a sender describes, SES signing with credentials, printing to a log. */
    transport(
        sender: MailSender,
        options: {
            /** The credentials SES signs with, asked once per send. */
            readonly credentials: () => Promise<AwsCredentials>;
            /** Write one printed email to the runtime's own log. */
            readonly print: (text: string) => void;
            /** The network, clock and crypto SES sends with. */
            readonly environment: Pick<Environment, "clock" | "crypto" | "network">;
        },
    ): MailTransport {
        // send through SES in its region
        if (sender.kind === "ses") {
            const client = new SesClient({
                region: sender.region,
                endpoint: SesClient.endpoint(sender.region),
                credentials: options.credentials,
                environment: options.environment,
            });

            return new SesTransport(client, { from: sender.from });
        }
        // print each email instead
        else {
            return new PrintTransport(options.print);
        }
    },
});
/** Where a deployment sends its mail: through SES, or printed to its runtime's own log. */
export type MailSender = schema.Infer<typeof mailSender>;
