import { testWorld } from "../test/browser.ts";
import { SimulatedTelemetry } from "@destack/environment/simulation";
import { TelemetryScope } from "@destack/telemetry";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { StoredItem } from "./storage.ts";

/** The package keeping the string. */
const NOTES = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

test("keep a string under its key, reporting storage the browser refuses and failing on anything else", async () => {
    // keep a string in a tab's storage, reporting to the world's telemetry
    const { tab } = testWorld();
    const recorded = new SimulatedTelemetry(tab);
    using telemetry = new TestTelemetry(recorded);
    const item = new StoredItem("theme", tab, TelemetryScope.of(NOTES, recorded));
    const before = item.read();
    item.write("dark");
    const kept = item.read();

    // refuse writes as a full storage does, then fail as a broken storage does
    tab.storage.setItem = () => {
        throw new DOMException("the storage is full", "QuotaExceededError");
    };
    item.write("light");
    tab.storage.getItem = () => {
        throw new TypeError("the storage broke");
    };
    const broken = (() => {
        try {
            return item.read();
        } catch (error) {
            return error;
        }
    })();

    expect([before, kept, telemetry.take(), broken]).toEqual([
        undefined,
        "dark",
        [{ scope: "@example/notes", name: "QuotaExceededError", message: "the storage is full" }],
        new TypeError("the storage broke"),
    ]);
});
