import type { ClientSocket } from "@destack/environment";
import { schema } from "@destack/schema";

/** The part of a call frame that names its procedure: the path below the peer's origin. */
const CallFrame = schema.looseObject({
    ["p"]: schema.looseObject({ ["u"]: schema.string() }),
});

/** Pass a socket through, handing each text frame it sends to a tap that may rewrite it. */
export function tapSocket(socket: ClientSocket, tap: (frame: string) => string): ClientSocket {
    return {
        get url() {
            return socket.url;
        },
        get protocol() {
            return socket.protocol;
        },
        get readyState() {
            return socket.readyState;
        },
        send: (message) => socket.send(tap(String(message))),
        close: (code, reason) => socket.close(code, reason),
        addEventListener: socket.addEventListener.bind(socket),
    };
}

/** Read the path of the procedure a call frame calls, absent for any other frame. */
export function calledPath(frame: string): string | undefined {
    const parsed = CallFrame.safeParse(JSON.parse(frame));

    return parsed.success ? new URL(parsed.data.p.u, "http://orpc").pathname : undefined;
}
