import { expect, test } from "@destack/test";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { testSimulation } from "../test/index.ts";
import { BrowserTab } from "./browser.ts";

/** The module the world defines the device worker's body for. */
const DEVICE = new URL("https://app.test/device.worker.js");

test("start the device worker at once, then ask it for the device, while the person has not answered the browser's persistence prompt", async () => {
    // define a device worker serving at a tab's first request
    await using simulation = testSimulation();
    const requests: unknown[] = [];
    simulation.workers.define(DEVICE, (scope) => {
        scope.addEventListener("message", (event) => {
            requests.push(event.data);
            scope.postMessage({ kind: "serving" });
        });
    });

    // start it from a tab whose browser prompts the person, who has not answered yet
    const tab = new BrowserSimulation(simulation, { persistentStorage: "prompt" }).environment(
        "tab",
    );
    const stop = await BrowserTab.worker(
        DEVICE,
        "device",
        Promise.resolve({ kind: "open" }),
        tab,
    )();
    await stop();

    // expect the device served the tab's request, the storage not persisted yet
    expect({ requests, isPersisted: tab.storage.isPersisted }).toEqual({
        requests: [{ kind: "open" }],
        isPersisted: false,
    });
});
