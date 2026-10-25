import { type AwsCredentials, AwsSigner } from "@destack/identity/aws";
import { schema } from "@destack/schema";
import type { Environment } from "@destack/environment";
import type { Envelope, MimeMessage } from "../mime/index.ts";
import type { FamilyError } from "@destack/error";
import { MessageError, SesFailure } from "../error/error.ts";

/** The SESv2 operation that sends one message. */
const SEND_PATH = "/v2/email/outbound-emails";

/** How long a send may take before it retries: fifteen seconds, far past SES's sub-second sends. */
const TIMEOUT_MILLISECONDS = 15_000;

/** The service name SES signs requests under. */
const SERVICE = "ses";

/** An AWS region name, such as eu-central-1 or us-gov-west-1. */
const REGION = /^[a-z]{2}(?:-[a-z]+)+-\d+$/u;

/** The AWS error codes of throttling and of exceeded sending quotas, which pass with time. */
const THROTTLING = new Set([
    "TooManyRequestsException",
    "ThrottlingException",
    "Throttling",
    "LimitExceededException",
]);

/** The AWS error codes of expired credentials, which a later send renews. */
const EXPIRED = new Set(["ExpiredTokenException", "ExpiredToken", "RequestExpired"]);

/** The failure reasons of permanent refusals, by AWS error code. */
const PERMANENT: ReadonlyMap<
    string,
    "MESSAGE_REJECTED" | "SENDER_UNVERIFIED" | "SENDING_SUSPENDED"
> = new Map([
    ["MessageRejected", "MESSAGE_REJECTED"],
    ["MailFromDomainNotVerifiedException", "SENDER_UNVERIFIED"],
    ["AccountSuspendedException", "SENDING_SUSPENDED"],
    ["SendingPausedException", "SENDING_SUSPENDED"],
]);

/** SES's answer to a sent message. */
const Sent = schema.looseObject({ MessageId: schema.string().min(1) });

/** SES's error body, with the message and sometimes the code. */
const ErrorBody = schema.looseObject({
    __type: schema.string().exactOptional(),
    code: schema.string().exactOptional(),
    message: schema.string().exactOptional(),
    Message: schema.string().exactOptional(),
});

/** Where and as whom a client sends mail through SES. */
export interface SesOptions {
    /** The AWS region requests are signed for, such as eu-central-1. */
    readonly region: string;
    /** The SESv2 origin: the region's (`SesClient.endpoint`), or a local inbox in development. */
    readonly endpoint: string;
    /** The signing credentials, asked once per send so temporary ones refresh. */
    readonly credentials: () => Promise<AwsCredentials>;
    /** The configuration set whose event destinations and suppression apply. */
    readonly configurationSet?: string;
    /** The network sends go through, the clock their timeouts run on, and the signing WebCrypto. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "network">;
}

/** A client of the SESv2 HTTP API sending composed messages. */
export class SesClient {
    /** The options. */
    readonly options: SesOptions;
    /** The signer of SES requests with the last credentials, absent before the first send. */
    #signer: AwsSigner | undefined;

    /** Build the origin of a region's SESv2 API. */
    static endpoint(region: string): string {
        return `https://email.${region}.amazonaws.com`;
    }

    /** Create a client, refusing a malformed region. */
    constructor(options: SesOptions) {
        if (!REGION.test(options.region)) {
            throw new TypeError(
                `region ${JSON.stringify(options.region)} is not an AWS region name`,
            );
        }
        this.options = options;
    }

    /** Read a signer of the current credentials, reusing its daily keys until they change. */
    async #upsertSigner(credentials: AwsCredentials): Promise<AwsSigner> {
        // reuse the signer of unchanged credentials
        const kept = this.#signer;
        if (
            kept?.credentials.accessKeyId === credentials.accessKeyId &&
            kept.credentials.secretAccessKey === credentials.secretAccessKey &&
            kept.credentials.sessionToken === credentials.sessionToken
        ) {
            return kept;
        }

        // sign with the new credentials from now on
        const { region, environment } = this.options;
        const signer = new AwsSigner({
            service: SERVICE,
            region,
            credentials,
            crypto: environment.crypto,
        });
        this.#signer = signer;

        return signer;
    }

    /** Send a message to the envelope's recipients from its sender and return SES's MessageId. */
    async send(envelope: Envelope, message: MimeMessage, now: Date): Promise<string> {
        // send the message's bytes as SES's raw content
        const configurationSet = this.options.configurationSet;
        const body = JSON.stringify({
            FromEmailAddress: envelope.sender,
            Destination: { ToAddresses: envelope.recipients },
            Content: { Raw: { Data: btoa(message.content) } },
            ...(configurationSet === undefined ? {} : { ConfigurationSetName: configurationSet }),
        });

        // sign the request with the current credentials
        const { endpoint, environment } = this.options;
        const url = new URL(SEND_PATH, endpoint);
        const signer = await this.#upsertSigner(await this.options.credentials());
        const headers = await signer.authorize(
            {
                method: "POST",
                path: AwsSigner.path(url.pathname, "double"),
                query: [],
                headers: { "content-type": "application/json", host: url.host },
                payloadHash: await AwsSigner.payloadHash(body, environment.crypto),
            },
            now,
        );

        // post it within the timeout, reading a lapsed or lost request as retryable
        const signal = environment.clock.timeout(TIMEOUT_MILLISECONDS);
        const request = new Request(url.href, { method: "POST", headers, body, signal });
        let response: Response;
        try {
            response = await environment.network.fetch(request);
        } catch (error) {
            const reason = signal.aborted ? "PROVIDER_TIMEOUT" : "PROVIDER_UNREACHABLE";
            throw MessageError.of(reason, `SES at ${endpoint} did not respond`, { cause: error });
        }

        // read the MessageId, or the refusal
        if (!response.ok) {
            throw await refusal(response);
        }
        const sent = Sent.safeParse(await response.json().catch(() => undefined));
        if (!sent.success) {
            throw MessageError.of(
                "RESPONSE_INVALID",
                "SES responded to a send without a MessageId",
            );
        }

        return sent.data.MessageId;
    }
}

/** Read SES's refusal as a failure: its AWS code from the error type header or the body. */
async function refusal(response: Response): Promise<FamilyError> {
    // read the code before any colon of the header, or before any hash of the body's type
    const text = await response.text();
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch {
        parsed = undefined;
    }
    const read = ErrorBody.safeParse(parsed);
    const fields: schema.Infer<typeof ErrorBody> = read.success ? read.data : {};
    const header = response.headers.get("x-amzn-errortype")?.split(":")[0];
    const awsCode = header ?? fields.code ?? fields.__type?.split("#").pop();
    const detail = fields.message ?? fields.Message ?? text;

    // classify the refusal
    const status = response.status;
    const data: SesFailure = awsCode === undefined ? {} : { awsCode };
    const message = `SES refused the send with HTTP ${status}${awsCode === undefined ? "" : ` ${awsCode}`}: ${detail}`;
    const permanent = awsCode === undefined ? undefined : PERMANENT.get(awsCode);
    if (status === 429 || (awsCode !== undefined && THROTTLING.has(awsCode))) {
        return MessageError.of("PROVIDER_THROTTLED", message, { data });
    } else if (status >= 500) {
        return MessageError.of("PROVIDER_UNAVAILABLE", message, { data });
    } else if (awsCode !== undefined && EXPIRED.has(awsCode)) {
        return MessageError.of("CREDENTIALS_EXPIRED", message, { data });
    } else if (permanent !== undefined) {
        return MessageError.of(permanent, message, { data });
    } else if (status === 401 || status === 403) {
        return MessageError.of("CREDENTIALS_REFUSED", message, { data });
    } else {
        return MessageError.of("REQUEST_INVALID", message, { data });
    }
}
