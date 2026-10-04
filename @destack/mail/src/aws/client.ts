import { schema } from "@destack/schema";
import type { Envelope, MimeMessage } from "../mime/index.ts";
import { SesError, type SesErrorCode } from "./error.ts";
import { signRequest, type AwsCredentials } from "./signature.ts";

/** The SESv2 operation that sends one message. */
const SEND_PATH = "/v2/email/outbound-emails";

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

/** The AWS error codes of expired credentials, which a later send asks fresh. */
const EXPIRED = new Set(["ExpiredTokenException", "ExpiredToken", "RequestExpired"]);

/** The AWS error codes of permanent refusals, by failure code. */
const PERMANENT: Readonly<Record<string, SesErrorCode>> = {
    MessageRejected: "REJECTED",
    MailFromDomainNotVerifiedException: "UNVERIFIED",
    AccountSuspendedException: "SUSPENDED",
    SendingPausedException: "SUSPENDED",
};

/** SES's answer to a sent message. */
const Sent = schema.looseObject({ MessageId: schema.string().min(1) });

/** SES's error body, which carries the message and sometimes the code. */
const Refusal = schema.looseObject({
    __type: schema.string().exactOptional(),
    code: schema.string().exactOptional(),
    message: schema.string().exactOptional(),
    Message: schema.string().exactOptional(),
});

/** Post one request to SES, as fetch does. */
export type SesFetch = (url: string, init: RequestInit) => Promise<Response>;

/** Where and as whom a client sends mail through SES. */
export interface SesOptions {
    /** The AWS region whose SES endpoint sends, such as eu-central-1. */
    readonly region: string;
    /** The signing credentials, asked once per send so temporary ones refresh. */
    readonly credentials: () => Promise<AwsCredentials>;
    /** The configuration set whose event destinations and suppression apply. */
    readonly configurationSet?: string;
    /** The fetch function. */
    readonly fetch?: SesFetch;
}

/** A client of the SESv2 HTTP API sending composed messages. */
export class SesClient {
    /** The region. */
    readonly region: string;
    /** The SES endpoint's origin, such as https://email.eu-central-1.amazonaws.com. */
    readonly endpoint: string;
    /** The options. */
    readonly #options: SesOptions;
    /** The fetch function. */
    readonly #fetch: SesFetch;

    /** Create a client, refusing a malformed region. */
    constructor(options: SesOptions) {
        if (!REGION.test(options.region)) {
            throw new SesError(
                "INVALID_OPTIONS",
                `region ${JSON.stringify(options.region)} is no AWS region name`,
            );
        }
        this.region = options.region;
        this.endpoint = `https://email.${options.region}.amazonaws.com`;
        this.#options = options;
        this.#fetch = options.fetch ?? fetch;
    }

    /** Send a message to the envelope's recipients from its sender and return SES's MessageId. */
    async send(envelope: Envelope, message: MimeMessage): Promise<string> {
        // carry the message's bytes as SES's raw content
        const configurationSet = this.#options.configurationSet;
        const body = JSON.stringify({
            FromEmailAddress: envelope.sender,
            Destination: { ToAddresses: envelope.recipients },
            Content: { Raw: { Data: btoa(message.content) } },
            ...(configurationSet === undefined ? {} : { ConfigurationSetName: configurationSet }),
        });

        // sign the request with the current credentials
        const url = new URL(SEND_PATH, this.endpoint);
        const signed = await signRequest(
            { method: "POST", url, headers: { "content-type": "application/json" }, body },
            {
                credentials: await this.#options.credentials(),
                region: this.region,
                service: SERVICE,
                date: new Date(),
            },
        );

        // post it, reading a network failure as retryable
        let response: Response;
        try {
            response = await this.#fetch(url.href, {
                method: "POST",
                headers: signed.headers,
                body,
            });
        } catch (error) {
            throw new SesError("CONNECTION", `SES at ${this.endpoint} did not answer`, undefined, {
                cause: error,
            });
        }

        // read the MessageId, or the refusal
        if (!response.ok) {
            throw await refusal(response);
        }
        const sent = Sent.safeParse(await response.json().catch(() => undefined));
        if (!sent.success) {
            throw new SesError("INVALID_RESPONSE", "SES answered a send without a MessageId", {
                awsCode: undefined,
                status: response.status,
            });
        }

        return sent.data.MessageId;
    }
}

/** Read SES's refusal as a failure: its AWS code from the error type header or the body. */
async function refusal(response: Response): Promise<SesError> {
    // read the code before any colon of the header, or before any hash of the body's type
    const text = await response.text();
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch {
        parsed = undefined;
    }
    const read = Refusal.safeParse(parsed);
    const fields: schema.Infer<typeof Refusal> = read.success ? read.data : {};
    const header = response.headers.get("x-amzn-errortype")?.split(":")[0];
    const awsCode = header ?? fields.code ?? fields.__type?.split("#").pop();
    const detail = fields.message ?? fields.Message ?? text;

    // classify the refusal
    const status = response.status;
    const answer = { awsCode, status };
    const message = `SES refused the send with HTTP ${status}${awsCode === undefined ? "" : ` ${awsCode}`}: ${detail}`;
    const permanent = awsCode === undefined ? undefined : PERMANENT[awsCode];
    if (status === 429 || (awsCode !== undefined && THROTTLING.has(awsCode))) {
        return new SesError("THROTTLED", message, answer);
    } else if (status >= 500) {
        return new SesError("UNAVAILABLE", message, answer);
    } else if (awsCode !== undefined && EXPIRED.has(awsCode)) {
        return new SesError("EXPIRED", message, answer);
    } else if (permanent !== undefined) {
        return new SesError(permanent, message, answer);
    } else if (status === 401 || status === 403) {
        return new SesError("UNAUTHORIZED", message, answer);
    } else {
        return new SesError("INVALID_REQUEST", message, answer);
    }
}
