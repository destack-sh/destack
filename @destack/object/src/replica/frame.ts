import { type Authentication, Caller, CALLER_HEADER } from "@destack/access";
import { present, schema } from "@destack/schema";
import { CallError } from "../error/call.ts";
import { CONNECTION_PING } from "./connection.ts";

/** The header a host refuses a socket's call's caller in, a call error's JSON. */
export const CALLER_REFUSAL_HEADER = "Destack-Caller-Refusal";

/** A call frame's head in oRPC's peer format: its identifier, type and headers. */
const CallFrameHead = schema.looseObject({
    ["i"]: schema.string(),
    ["t"]: schema.number().exactOptional(),
    ["p"]: schema.looseObject({
        ["h"]: schema.record(schema.string(), schema.string()).exactOptional(),
    }),
});

/** A connection's text frames as its peers send them: calls, their streams' events and aborts. */
export const CallFrame = {
    /** Read a call frame's call as a request below a socket's URL, absent for any other frame. */
    request(frame: string, url: string): Request | undefined {
        const head = callHead(frame);

        return head === undefined
            ? undefined
            : new Request(url, { headers: new Headers(head["p"]["h"]) });
    },

    /** Authorize a call frame with a token taken for it alone, as a gateway signs each call. */
    async authorize(frame: string, token: () => Promise<string>): Promise<string> {
        // leave any frame but a call, taking no token for it
        if (callHead(frame) === undefined) {
            return frame;
        }
        const taken = await token();

        return rewriteCall(frame, (headers) => {
            headers.set("authorization", `Bearer ${taken}`);
        });
    },

    /** Forward a call frame's caller a host vouched for, absent for an anonymous one. */
    forward(frame: string, authentication: Authentication | undefined): string {
        return rewriteCall(frame, (headers) => {
            // replace the call's own credentials with the vouched caller
            headers.delete("authorization");
            headers.delete(CALLER_REFUSAL_HEADER);
            headers.delete(CALLER_HEADER);
            if (authentication !== undefined) {
                Caller.forward(authentication, headers);
            }
        });
    },

    /** Refuse a call frame's caller, which its instance answers as the call's failure. */
    refuse(frame: string, refusal: CallError<string, unknown>): string {
        return rewriteCall(frame, (headers) => {
            headers.delete("authorization");
            headers.delete(CALLER_HEADER);
            headers.set(CALLER_REFUSAL_HEADER, JSON.stringify(refusal.toJSON()));
        });
    },

    /** Read the refusal a host forwarded with a call, absent for a call it vouched for. */
    refusal(request: Request): CallError<string, unknown> | undefined {
        // read no refusal without the header, and refuse a malformed one its host wrote
        const refused = request.headers.get(CALLER_REFUSAL_HEADER);
        if (refused === null) {
            return undefined;
        }

        return present(CallError.fromJson(JSON.parse(refused)), "the host's refusal of the caller");
    },
};

/** Rewrite a call frame's headers, leaving a liveness probe and any other frame as it is. */
function rewriteCall(frame: string, edit: (headers: Headers) => void): string {
    // leave any frame but a call
    const head = callHead(frame);
    if (head === undefined) {
        return frame;
    }

    // edit the call's headers
    const payload = head["p"];
    const headers = new Headers(payload["h"]);
    edit(headers);

    return JSON.stringify({ ...head, ["p"]: { ...payload, ["h"]: Object.fromEntries(headers) } });
}

/** Read a call frame's head, absent for a liveness probe and any frame but a call. */
function callHead(frame: string): schema.Infer<typeof CallFrameHead> | undefined {
    const head = frame === CONNECTION_PING ? undefined : CallFrameHead.safeParse(JSON.parse(frame));

    return head?.success === true && head.data["t"] === undefined ? head.data : undefined;
}
