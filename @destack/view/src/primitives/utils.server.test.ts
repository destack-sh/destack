import { expect, test } from "@destack/test";
import { createHydratableSignal } from "./utils.ts";

test("render the server's value of a hydratable signal on the server", () => {
    const [signal] = createHydratableSignal("server", () => "client");

    expect(signal()).toBe("server");
});
