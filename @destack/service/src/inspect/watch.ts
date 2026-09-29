import { reference } from "@destack/package/declare";
import { WatchDescription, type Watch } from "../watch/index.ts";

/** Describe a watch. */
export function describeWatch(watch: Watch): WatchDescription {
    return WatchDescription.parse({
        name: watch.name,
        object: reference(watch.object),
        where: watch.where,
        on: watch.on,
        from: watch.from,
        maxLag: watch.maxLag,
    });
}
