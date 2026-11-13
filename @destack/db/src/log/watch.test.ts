import { expect, test } from "@destack/test";
import { type Channel, type Commit, DELIVERED } from "../channel/channel.ts";
import { CommitWatch } from "./watch.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { Telemetry } from "@destack/environment";
import { testHost } from "@destack/test/host";

test("announce a commit's tables the database leaves unannounced, nothing when it announces them all, and every table a script changed", () => {
    // watch commits on a channel recording what the connection announces
    const announced: Commit[] = [];
    const channel: Channel<Commit> = {
        notify: (message) => announced.push(message),
        listen: () => () => undefined,
        delivery: () => DELIVERED,
        close: async () => {},
    };
    const scope = TelemetryScope.of(import.meta.destack.package, Telemetry.none(testHost().clock));
    const watch = new CommitWatch(channel, new Set(["cache_note"]), "writer", scope, "test");
    watch.declare(["log_note", "cache_note"], ["log_note"]);

    // commit a logged table with an unlogged one, then only the logged one, then both by a script
    watch.notify(new Set(["log_note", "cache_note"]), new Set());
    watch.notify(new Set(["log_note"]), new Set());
    watch.notify(new Set(["log_note", "cache_note"]), new Set(["log_note", "cache_note"]));
    expect(announced).toEqual([
        { kind: "commit", tables: ["cache_note"] },
        { kind: "commit", tables: ["log_note", "cache_note"] },
    ]);
});
