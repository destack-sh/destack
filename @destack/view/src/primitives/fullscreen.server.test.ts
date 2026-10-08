import { expect, test } from "@destack/test";
import { untouched } from "../test/server.ts";
import { createFullscreen, fullscreen } from "./fullscreen.ts";

test("open no fullscreen on the server", async () => {
    const { enter, exit, isActive } = createFullscreen(() => undefined);
    fullscreen()(untouched<HTMLElement>());

    expect([await enter(), await exit(), isActive()]).toEqual([undefined, undefined, false]);
});
