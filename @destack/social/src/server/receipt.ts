import { notification } from "@destack/notification";
import type { Call } from "@destack/object";
import * as base from "../object/receipt.ts";

/** Receipts on the server that read the owner's notifications on the object. */
export const receipt = base.receipt.handle({
    create: async (call, next) => readNotifications(call, await next()),
    update: async (call, next) => readNotifications(call, await next()),
});

/** Mark the caller's notifications on a receipt's object read. */
async function readNotifications(call: Call, row: base.Receipt): Promise<base.Receipt> {
    await call.invoke(notification).readAll({
        where: { thread: row.parentId, readAt: null },
        readAt: call.now,
    });

    return row;
}
