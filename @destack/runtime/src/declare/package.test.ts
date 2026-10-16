import { principal } from "@destack/access";
import { Package, PackageError } from "@destack/package";
import { expect, test } from "@destack/test";
import { defineProcedure } from "../procedure/index.ts";
import { defineAdapter, definePackage, type ObjectDeclaration } from "./package.ts";

/** A procedure every object type shares. */
const push = defineProcedure({
    authentication: "identity",
    permission: { principals: [principal.user, principal.machine, principal.installation] },
    audit: false,
});

/** A procedure of the note type. */
const create = defineProcedure({ authentication: "public", permission: "anyone", audit: false });

/** A note type routing its own create beside the shared push. */
const note: ObjectDeclaration = {
    package: { name: "@destack/runtime" },
    name: "note",
    typeReference: { packageId: "package-01a0c80b-614f-73ee-8687-f9c041e21f17", type: "note" },
    hooks: {},
    procedures: { create },
    shared: { replica: { push } },
};

test("declare a package handle with its identity, the router its objects derive, and its adapters", () => {
    // declare notes with an npm adapter for anyone, a hook adapter for users and a sync adapter for itself
    const npm = defineAdapter("npm", { path: "/npm" });
    const hook = defineAdapter("hook", { path: "/hook", callers: [principal.user] });
    const sync = defineAdapter("sync", { path: "/sync", callers: "self" });
    const handle = definePackage({
        objects: { note },
        adapters: { npm, hook, sync },
        since: "2026.9.0",
    });
    const declaring = Package.declaring(handle);

    expect({
        identity: [handle.id, handle.name],
        declaring: Package.declaring(handle),
        since: handle.since,
        routes: Object.keys(handle.router),
        adapters: handle.adapters,
    }).toEqual({
        identity: ["package-01a0c80b-614f-73ee-8687-f9c041e21f17", "@destack/runtime"],
        declaring: { id: handle.id, name: handle.name, version: handle.version },
        since: "2026.9.0",
        routes: ["note", "replica"],
        adapters: {
            npm: { package: declaring, name: "npm", path: "/npm" },
            hook: {
                package: declaring,
                name: "hook",
                path: "/hook",
                callers: [{ packageId: principal.user.definition.packageId, type: "user" }],
            },
            sync: { package: declaring, name: "sync", path: "/sync", callers: "self" },
        },
    });
});

test("refuse adapters at an empty path, and adapters at or below another's path", () => {
    // refuse an adapter at the mount itself
    expect(() => defineAdapter("root", { path: "/" })).toThrow(
        new PackageError("INVALID_DEFINITION", "adapter root has no path below the mount: /"),
    );

    // refuse an adapter at another's path, and one below it
    const npm = defineAdapter("npm", { path: "/npm" });
    const twin = defineAdapter("twin", { path: "/npm" });
    const nested = defineAdapter("nested", { path: "/npm/tarballs" });
    expect(() => definePackage({ adapters: { npm, twin } })).toThrow(
        new PackageError("INVALID_DEFINITION", "adapter path /npm overlaps another"),
    );
    expect(() => definePackage({ adapters: { npm, nested } })).toThrow(
        new PackageError("INVALID_DEFINITION", "adapter path /npm/tarballs overlaps another"),
    );
});

test("refuse a declaration listed under another name, and a name taken twice", () => {
    const npm = defineAdapter("npm", { path: "/npm" });

    expect(() => definePackage({ adapters: { git: npm } })).toThrow(
        new PackageError("INVALID_DEFINITION", "declaration npm is listed under another name: git"),
    );
    expect(() => definePackage({ adapters: { npm }, views: { npm } })).toThrow(
        new PackageError("INVALID_DEFINITION", "duplicate declaration: npm"),
    );
});

test("refuse routing an object type under the name of shared procedures, and route each name once", () => {
    // refuse a type taking the shared name
    expect(() => definePackage({ objects: { replica: note } })).toThrow(
        new TypeError("package @destack/runtime routes two procedures under replica"),
    );

    // route each type and the shared procedures once
    const notebook = { ...note, name: "notebook" };
    expect(Object.keys(definePackage({ objects: { note, notebook } }).router)).toEqual([
        "note",
        "notebook",
        "replica",
    ]);
});
