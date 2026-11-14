import { expect, test } from "@destack/test";
import { type Channel, type ChannelListener, type Commit, DELIVERED } from "../channel/channel.ts";
import { present } from "@destack/schema";
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

test("drop the echo of an own commit made before listening settles, waking on another writer's", async () => {
    // watch commits on a channel whose delivery the test drives
    let listening: ChannelListener<Commit> | undefined;
    const channel: Channel<Commit> = {
        notify: () => undefined,
        listen: (given) => {
            listening = given;

            return () => undefined;
        },
        delivery: () => DELIVERED,
        close: async () => {},
    };
    const scope = TelemetryScope.of(import.meta.destack.package, Telemetry.none(testHost().clock));
    const watch = new CommitWatch(channel, new Set(), "writer", scope, "test");
    watch.declare(["log_note"], ["log_note"]);

    // commit before delivery resumes, then receive its echo and another writer's commit
    void watch.listen();
    void watch.expect("own");
    const listener = present(listening, "the watch's listener");
    listener.resume?.();
    const resumed = watch.generation;
    listener.receive({ kind: "commit", tables: ["log_note"], token: "own" });
    const echoed = watch.generation;
    listener.receive({ kind: "commit", tables: ["log_note"] });

    // expect the echo to wake nobody, and the other writer's commit to wake once
    expect([echoed - resumed, watch.generation - echoed]).toEqual([0, 1]);
});
