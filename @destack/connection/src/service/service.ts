import { defineService } from "@destack/service";
import { integration, connection } from "../object/index.ts";

/** The connections to external accounts and the integrations they authorize at. */
export const connectionService = defineService("connection", {
    objects: { integration, connection },
});
