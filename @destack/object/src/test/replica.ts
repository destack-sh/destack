import { ControlLoop, type Controller, Snapshot } from "@destack/db";
import { reconciliation } from "@destack/db/test";
import { present } from "@destack/schema";
import type { ObjectStore } from "../server/store.ts";
import { replicaRequest } from "../replica/request.ts";
import { worldTelemetry } from "./world.ts";

/** A store's followed replicas: their keys, and the abort ending their follows. */
export interface ReplicaFollow {
    /** The followed replicas' keys, `<name> <scope>`. */
    readonly keys: readonly string[];
    /** Aborts the follows. */
    readonly controller: AbortController;
    /** The running loop of the follows, settled once aborted. */
    readonly running: Promise<void>;
}

/** Follow the replicas a store requests now, those a filter selects. */
export async function followReplicas(
    store: ObjectStore,
    options: {
        /** Select a requested replica's key to follow, every one when absent. */
        readonly isFollowed?: (key: string) => boolean;
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
        when: { key: { in: [...keys] } },
    };
    const controller = new AbortController();
    const loop = new ControlLoop(store.database, [
        { controllers: [follow], projections: [], cursors: [] },
    ]);

    return { keys, controller, running: loop.run(controller.signal) };
}

/** Run a store's controllers and cursors until they settle, failing on a captured failure. */
export async function settleControl(store: ObjectStore): Promise<void> {
    // run the store's loop until it idles, from its position
    const telemetry = worldTelemetry(store.environment);
    const before = telemetry.exceptions().length;
    const stopping = new AbortController();
    const running = store.run(stopping.signal);
    await store.idle();
    stopping.abort();
    await running;

    // fail on any failure the loop captured meanwhile, as each one's name and message
    const failed = telemetry
        .exceptions()
        .slice(before)
        .map((exception) => `${exception.name}: ${exception.message}`);
    if (failed.length > 0) {
        telemetry.take();
        throw new AggregateError(failed, "the store's controllers failed");
    }
}
