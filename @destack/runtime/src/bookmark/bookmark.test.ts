import { expect, test } from "@destack/test";
import { Bookmark } from "./bookmark.ts";

test("keep the newest watermark per scope, and the one of the log a moved scope writes to now", () => {
    // merge two positions of one log, and then one of the log the scope moved to
    const bookmark = new Bookmark();
    bookmark.observe({ scope: "space-1", log: "log-a", epoch: "epoch-1", sequence: 7 });
    bookmark.observe({ scope: "space-1", log: "log-a", epoch: "epoch-1", sequence: 5 });
    const before = [...bookmark.watermarks];
    bookmark.observe({ scope: "space-1", log: "log-b", epoch: "epoch-1", sequence: 2 });

    // keep the later of one log's positions, then replace it with the other log's
    expect([before, bookmark.watermarks]).toEqual([
        [{ scope: "space-1", log: "log-a", epoch: "epoch-1", sequence: 7 }],
        [{ scope: "space-1", log: "log-b", epoch: "epoch-1", sequence: 2 }],
    ]);
});
