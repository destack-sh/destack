import { defineDatabase } from "@destack/db";
import { journal } from "@destack/audit/stack";
import { address, device, pushEndpoint, user } from "@destack/account/object";
import { message } from "@destack/message";
import { setting } from "@destack/setting/object";
import { delivery } from "../object/delivery.ts";
import { notification } from "../object/activity.ts";

/** The database of one home's inbox: its notifications, their deliveries and the journal, with copies of the users it projects for, of their settings, devices and push endpoints, and of the messages its deliveries send. */
export const inboxDatabase = defineDatabase({
    name: "main",
    tables: [...notification.tables, ...delivery.tables, journal],
    copies: [
        user.table,
        address.table,
        setting.table,
        device.table,
        pushEndpoint.table,
        message.table,
    ],
});
