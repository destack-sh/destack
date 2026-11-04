import { ControlLoop, type Controller, Snapshot } from "@destack/db";
import { reconciliation } from "@destack/db/test";
import { present } from "@destack/schema";
import type { ObjectStore } from "../server/store.ts";
import { replicaRequest } from "../replica/request.ts";

/** A store's followed replicas: their keys, and the abort ending their follows. */
export interface ReplicaFollow {
    /** The followed replicas' keys, `<name> <scope>`. */
    readonly keys: readonly string[];
    /** Aborts the follows. */
    readonly controller: AbortController;
    /** The running loop of the follows, settled once aborted. */
    readonly running: Promise<void>;
}

/** Follow the replicas a store requests now, those a filter selects, reporting each failure. */
export async function followReplicas(
    store: ObjectStore,
    options: {
        /** Select a requested replica's key to follow, every one when absent. */
        readonly isFollowed?: (key: string) => boolean;
        /** Report a follow's failure. */
        readonly report: (controller: unknown, key: string, error: unknown) => void;
    },
): Promise<ReplicaFollow> {
    // list the requests once as the sink's consumer does
    const subscriber = present(store.subscriber, "the store's subscriber");
    await store.sink.cursor(subscriber).take(reconciliation());
    const rows = await Snapshot.live(store.database).read(replicaRequest).findMany();
    const keys = rows.map((row) => row.key).filter(options.isFollowed ?? (() => true));

    // follow the selected requests
    const follow: Controller<typeof replicaRequest> = {
        ...store.sink.controller(subscriber),
        pending: { key: { in: [...keys] } },
    };
    const controller = new AbortController();
    const loop = new ControlLoop(
        store.database,
        { controllers: [follow], cursors: [] },
        { report: options.report },
    );

    return { keys, controller, running: loop.run(controller.signal) };
}

/** Run a store's controllers and cursors until they settle, failing on a report. */
export async function settleControl(store: ObjectStore): Promise<void> {
    // run the loop until it idles, collecting its reports
    const stopping = new AbortController();
    const reported: unknown[] = [];
    const loop = new ControlLoop(store.database, store.control(), {
        report: (_controller, _key, error) => reported.push(error),
    });
    const running = loop.run(stopping.signal);
    await loop.idle();
    stopping.abort();
    await running;

    // fail on any report
    if (reported.length > 0) {
        throw new AggregateError(reported, "the store's controllers failed");
    }
}
