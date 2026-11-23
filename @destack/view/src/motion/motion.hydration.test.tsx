import { expect, test } from "@destack/test";
import { renderOnServer } from "../test/hydration.ts";

test("render a reduced motion follower on the server without reading the display", async () => {
    // render the fixture on a server render's environment, which refuses the display
    const rendered = await renderOnServer("src/motion/motion.fixture.tsx");

    expect(rendered).toEqual({ MotionState: "<p _hk=001>moving</p>" });
});
