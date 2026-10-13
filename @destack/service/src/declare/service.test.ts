import { principal } from "@destack/access";
import { expect, test } from "@destack/test";
import { defineProcedure } from "../procedure/index.ts";
import { defineService, type ObjectDeclaration } from "./service.ts";

/** A procedure every declaration of a kind shares. */
const push = defineProcedure({
    authentication: "identity",
    permission: { principals: [principal.user, principal.machine, principal.installation] },
    audit: false,
});

/** A declaration sharing the push procedure under `replica`. */
const routed: ObjectDeclaration = {
    package: { name: "@destack/test" },
    name: "routed",
    typeReference: { packageId: "package-01996ab0-0000-7000-8000-00000000c002", type: "routed" },
    hooks: {},
    procedures: {},
    shared: { replica: { push } },
};

test("refuse routing a declaration or a procedure under the name of shared procedures", () => {
    // refuse a declaration taking the shared name
    expect(() => defineService("notes", { objects: { replica: routed } })).toThrow(
        new TypeError("service notes routes two procedures under replica"),
    );

    // refuse a procedure taking the shared name
    expect(() => defineService("notes", { objects: { note: routed }, replica: { push } })).toThrow(
        new TypeError("service notes routes two procedures under replica"),
    );

    // route each name once
    expect(
        Object.keys(defineService("notes", { objects: { note: routed, notebook: routed } }).router),
    ).toEqual(["note", "notebook", "replica"]);
});
