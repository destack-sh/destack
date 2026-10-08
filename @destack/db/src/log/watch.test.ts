import { expect, test } from "@destack/test";
import type { Channel, Commit } from "../channel/channel.ts";
import { CommitWatch } from "./watch.ts";

test("announce a commit's tables the database leaves unannounced, and nothing when it announces them all", () => {
    // watch commits on a channel recording what the connection announces
    const announced: Commit[] = [];
    const channel: Channel<Commit> = {
        notify: (message) => announced.push(message),
        listen: () => () => undefined,
    };
    const watch = new CommitWatch(channel, new Set(["log_note"]), ["log_note", "cache_note"]);

    // commit a logged table with an unlogged one, then the logged one alone
    watch.notify(["log_note", "cache_note"]);
    watch.notify(["log_note"]);
    expect(announced).toEqual([{ kind: "commit", tables: ["cache_note"] }]);
});
