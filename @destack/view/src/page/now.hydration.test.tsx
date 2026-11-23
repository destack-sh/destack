import { expect, test, vi } from "@destack/test";
import { flush } from "solid-js";
import { TEST_START } from "@destack/test/world";
import { render, wait } from "../test/dom.ts";
import { renderOnServer } from "../test/hydration.ts";
import { CurrentTime } from "./now.fixture.tsx";

/** The server's HTML of the fixture, which renders before any time is known. */
const { CurrentTime: html = "" } = await renderOnServer("src/page/now.fixture.tsx");

test("render no time on the server, hydrate without a mismatch, then show the tab's time once mounted", async () => {
    // collect the warnings hydration gives for nodes it misses
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    // hydrate the server's paragraph, then let the mount run
    const container = document.createElement("div");
    container.innerHTML = html;
    render(() => <CurrentTime />, { container, hydrate: true });
    const hydrated = container.textContent;
    await wait(0);
    flush();
    const warnings = [...warn.mock.calls];
    warn.mockRestore();

    expect([html, hydrated, container.textContent, warnings]).toEqual([
        "<p _hk=000>time <!--$-->unknown<!--/--></p>",
        "time unknown",
        `time ${TEST_START}`,
        [],
    ]);
});
