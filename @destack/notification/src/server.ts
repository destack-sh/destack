import { implementPackage } from "@destack/object/server";
import notificationPackage from "./package.ts";
import { NotificationServer } from "./server/server.ts";

export * from "./server/index.ts";

/** The notification package's server, which its installations share with their read builds. */
export default implementPackage(notificationPackage, new NotificationServer({}).definition());
