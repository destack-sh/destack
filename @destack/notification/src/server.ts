import { defineServer } from "@destack/object/server";
import notificationPackage from "./package.ts";
import { NotificationServer } from "./server/server.ts";

export * from "./server/index.ts";

/** The notification package's server: activities, announcements and their delivery as messages. */
export default defineServer(notificationPackage, new NotificationServer({}).definition());
