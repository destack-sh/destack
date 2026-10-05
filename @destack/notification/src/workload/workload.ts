import { device, pushEndpoint } from "@destack/account/object";
import { Catalog } from "@destack/locale";
import { message } from "@destack/message";
import { ObjectServer } from "@destack/object/server";
import { ServiceError } from "@destack/service/error";
import { defineWorkload } from "@destack/service/workload";
import { setting } from "@destack/setting/object";
import { serveInbox } from "../server/inbox.ts";
import { inboxService } from "../service/service.ts";
import { inboxDatabase } from "../stack/index.ts";

/** A person's inbox in their home: the notifications other spaces' activities project into it, delivered as messages, installed in each home. */
export const inboxWorkload = defineWorkload({
    name: "inbox",
    start: async (context) => {
        // require the home's installation the inbox serves
        const { installation, resources } = context;
        if (installation === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "the inbox runs as a home's installation",
            });
        }

        // read the inbox's own catalogs, copying its residents' scopes and its messages through its cell
        const catalogs = await Catalog.read(installation.build);
        const handled = serveInbox({ catalogs });

        return {
            services: [
                ObjectServer.serve(inboxService, {
                    database: inboxDatabase.get(resources),
                    callKey: context.callKey,
                    ...(context.history === undefined ? {} : { history: context.history }),
                    installation,
                    ...(context.runs === undefined ? {} : { runs: context.runs }),
                    handled: Object.values(handled),
                    policies: [setting, device, pushEndpoint, message],
                }),
            ],
        };
    },
});
