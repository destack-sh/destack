import { expect, test } from "@destack/test";
import { type Channel, type Commit, HEARD } from "../channel/channel.ts";
import { CommitWatch } from "./watch.ts";

test("announce a commit's tables the database leaves unannounced, and nothing when it announces them all", () => {
    // watch commits on a channel recording what the connection announces
    const announced: Commit[] = [];
    const channel: Channel<Commit> = {
        notify: (message) => announced.push(message),
        listen: () => () => undefined,
        hear: () => HEARD,
        close: async () => {},
    };
    const watch = new CommitWatch(channel, new Set(["log_note"]), ["log_note", "cache_note"]);

    // commit a logged table with an unlogged one, then only the logged one
    watch.notify(["log_note", "cache_note"]);
    watch.notify(["log_note"]);
    expect(announced).toEqual([{ kind: "commit", tables: ["cache_note"] }]);
});
