import { expect, test } from "@destack/test";
import {
    createDerivedStaticStore,
    createHydratableStaticStore,
    createStaticStore,
} from "./static-store.ts";

test("render the server's values of static stores on the server", () => {
    const [store] = createStaticStore({ foo: "server" });
    const [hydratable] = createHydratableStaticStore({ foo: "server" }, () => ({ foo: "client" }));
    const derived = createDerivedStaticStore(() => ({ foo: "server" }));

    expect([{ ...store }, { ...hydratable }, { ...derived }]).toEqual([
        { foo: "server" },
        { foo: "server" },
        { foo: "server" },
    ]);
});
