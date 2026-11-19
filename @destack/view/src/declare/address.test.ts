import { expect, test } from "@destack/test";
import { ViewAddress } from "./address.ts";

test("write a location in an installation's view as an open path and read it back", () => {
    const address = ViewAddress.parse({
        space: "team--ada",
        installation: "tasks",
        view: "board",
        path: "/task/42?tab=comments",
    });

    const path = ViewAddress.openPath(address);
    const read = ViewAddress.safeParseOpenPath(new URL(path, "https://home.example"));

    expect({ path, read: read.data }).toEqual({
        path: "/.destack/open?space=team--ada&installation=tasks&view=board&path=%2Ftask%2F42%3Ftab%3Dcomments",
        read: address,
    });
});
