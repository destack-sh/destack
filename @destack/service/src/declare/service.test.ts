import { expect, test } from "@destack/test";
import { defineProcedure } from "../procedure/index.ts";
import { defineService, type Routed } from "./service.ts";

/** A procedure every declaration of a kind shares. */
const push = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** A declaration routing its own procedures and sharing the push procedure under `replica`. */
const routed: Routed = { procedures: {}, shared: { replica: { push } } };

test("refuse routing a declaration or a procedure under the name of shared procedures", () => {
    // refuse a declaration taking the shared name
    expect(() => defineService("notes", { objects: { replica: routed } })).toThrow(
        new TypeError("service notes routes two procedures under replica"),
    );

    // refuse a procedure taking the shared name
    expect(() => defineService("notes", { objects: { note: routed }, replica: { push } })).toThrow(
        new TypeError("service notes routes two procedures under replica"),
    );

    // route distinct names once each, the shared procedures once for every declaration
    expect(
        Object.keys(defineService("notes", { objects: { note: routed, notebook: routed } }).router),
    ).toEqual(["note", "notebook", "replica"]);
});
