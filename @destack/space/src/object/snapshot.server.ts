import { ServiceError } from "@destack/service/error";
import * as base from "./snapshot.ts";

export * from "./snapshot.ts";

/** Snapshots on the server. */
export const snapshot = base.snapshot.handle({
    create: async () => {
        // TODO #Incomplete: take the snapshot through a snapshot controller
        throw new ServiceError("NOT_IMPLEMENTED", { message: "no controller takes snapshots yet" });
    },
    delete: async () => {
        // TODO #Incomplete: remove the snapshot through a snapshot controller
        throw new ServiceError("NOT_IMPLEMENTED", {
            message: "no controller removes snapshots yet",
        });
    },
});

/** Restorations on the server. */
export const restoration = base.restoration.handle({
    create: async () => {
        // TODO #Incomplete: run the restoration through a restoration controller
        throw new ServiceError("NOT_IMPLEMENTED", {
            message: "no controller runs restorations yet",
        });
    },
});
