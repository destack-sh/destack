import { notification } from "@destack/notification";
import type { Call } from "@destack/object";
import * as base from "./receipt.ts";

export * from "./receipt.ts";

/** Receipts on the server, which read the owner's notifications on the object. */
export const receipt = base.receipt.handle({
    create: async (call, next) => readNotifications(call, (await next()) as base.ReceiptRow),
    update: async (call, next) => readNotifications(call, (await next()) as base.ReceiptRow),
});

/** Mark the caller's notifications on a receipt's object read. */
async function readNotifications(call: Call, row: base.ReceiptRow): Promise<base.ReceiptRow> {
    await call.invoke(notification, "readAll", {
        where: { thread: row.parentId, readAt: null },
        readAt: call.now,
    });

    return row;
}
