import { expect, test } from "@destack/test";
import { DurableObjectHibernation, type HibernatingSocket } from "./hibernation.ts";

test("answer a hibernated socket's close with its code after handing it to the host, a lost connection as going away", () => {
    // keep one hibernated socket under a tag, as a Durable Object woken by its close
    const events: unknown[] = [];
    const socket: HibernatingSocket = {
        send: () => undefined,
        accept: () => undefined,
        close: (code, reason) => events.push(["answered", code, reason]),
        serializeAttachment: () => undefined,
        deserializeAttachment: () => ({
            instance: "instance-a",
            acceptance: { id: "socket-a", attachment: null },
            isVouched: false,
        }),
    };
    const hibernation = new DurableObjectHibernation(
        { acceptWebSocket: () => undefined, getTags: () => ["store"] },
        "store",
    );

    // hand its close to a host recording what it is told
    hibernation.close(
        {
            websocket: {
                open: (kept) => events.push(["opened", kept.attachment.acceptance.id]),
                message: () => undefined,
                close: (kept, code, reason) =>
                    events.push(["closed", kept.attachment.acceptance.id, code, reason]),
            },
        },
        socket,
        4000,
        "bye",
    );
    hibernation.close({ websocket: { message: () => undefined } }, socket, 1006, "");

    // expect the host to learn of the socket and its close, then each close answered
    expect(events).toEqual([
        ["opened", "socket-a"],
        ["closed", "socket-a", 4000, "bye"],
        ["answered", 4000, "bye"],
        ["answered", 1001, ""],
    ]);
});
