import { SmtpError } from "./error.ts";

/** The longest reply line in octets, CRLF included, as RFC 5321 section 4.5.3.1.5 sets. */
export const MAX_REPLY_LINE_OCTETS = 512;

/** One line of a server reply: `250-text` continues the reply, `250 text` ends it. */
const REPLY_LINE = /^([2-5]\d\d)(?:([ -])(.*))?$/;

/** An RFC 3463 enhanced status code leading a reply line, such as `5.1.1`. */
const ENHANCED_CODE = /^([245])(\.\d{1,3}\.\d{1,3})(?: |$)/;

/** One parsed reply line. */
export interface ReplyLine {
    /** The reply code. */
    readonly code: number;
    /** Whether the line ends its reply. */
    readonly isLast: boolean;
    /** The text after the code and separator. */
    readonly text: string;
}

/** A server reply to one command, as the server sent it. */
export class Reply {
    /** The RFC 5321 reply code, such as 250. */
    readonly code: number;
    /** The RFC 3463 enhanced status code of the first line, such as 2.1.5. */
    readonly enhancedCode: string | undefined;
    /** The lines' text after their codes, joined by line feeds. */
    readonly text: string;

    /** Create a reply. */
    constructor(code: number, enhancedCode: string | undefined, text: string) {
        this.code = code;
        this.enhancedCode = enhancedCode;
        this.text = text;
    }

    /** Parse one reply line without its CRLF, refusing a long line or a line of another shape. */
    static parseLine(bytes: Uint8Array): ReplyLine {
        // refuse a line over the limit
        if (bytes.length + 2 > MAX_REPLY_LINE_OCTETS) {
            throw new SmtpError(
                "PROTOCOL",
                `server sent a reply line over ${MAX_REPLY_LINE_OCTETS} octets`,
            );
        }

        // decode the line, refusing bytes outside UTF-8
        let line: string;
        try {
            line = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
        } catch (error) {
            throw new SmtpError("PROTOCOL", "server sent a reply line outside UTF-8", undefined, {
                cause: error,
            });
        }

        // match the line's shape
        const parsed = REPLY_LINE.exec(line);
        if (parsed === null) {
            throw new SmtpError(
                "PROTOCOL",
                `server sent ${JSON.stringify(line)}, which is no reply line`,
            );
        }

        return { code: Number(parsed[1]), isLast: parsed[2] !== "-", text: parsed[3] ?? "" };
    }

    /** Join a reply's lines, lifting an enhanced status code of the reply's class out of each. */
    static join(lines: readonly ReplyLine[]): Reply {
        // read the enhanced code of the first line when its class matches the reply's
        const code = lines[0]!.code;
        const replyClass = String(Math.floor(code / 100));
        const first = ENHANCED_CODE.exec(lines[0]!.text);
        const enhancedCode = first?.[1] === replyClass ? `${first[1]}${first[2]}` : undefined;

        // strip the matching enhanced code from each line
        const text = lines
            .map((line) => {
                const enhanced = ENHANCED_CODE.exec(line.text);

                return enhanced?.[1] === replyClass
                    ? line.text.slice(enhanced[0].length)
                    : line.text;
            })
            .join("\n");

        return new Reply(code, enhancedCode, text);
    }

    /** The reply's class, its code's first digit: 2 completes, 3 continues, 4 and 5 fail. */
    get class(): number {
        return Math.floor(this.code / 100);
    }

    /** Return the reply when it completes positively, and reject the step otherwise. */
    require(step: string): Reply {
        if (this.class !== 2) {
            throw new SmtpError(
                "REJECTED",
                `server answered ${step} with ${this.code} ${this.text}`,
                this,
            );
        }

        return this;
    }
}
