import { principal } from "@destack/access";
import { PackageVersion, PackageId } from "@destack/package";
import { expect, test, refusal } from "@destack/test";
import { Procedure } from "../procedure/index.ts";
import { defineMount, definePackage, type ObjectDeclaration } from "./package.ts";
import { defineInstallationBinding } from "./binding.ts";
import { Capability } from "./description.ts";
import { t } from "@destack/locale";

/** A procedure every object type shares. */
const push = Procedure.of({
    package: PackageVersion.of(import.meta.destack.package),
    authentication: "identity",
    permission: { principals: [principal.user, principal.machine, principal.installation] },
    audit: false,
});

/** A procedure of the note type. */
const create = Procedure.of({
    package: PackageVersion.of(import.meta.destack.package),
    authentication: "public",
    permission: "anyone",
    audit: false,
});

/** A note type routing a separate create beside the shared push. */
const note: ObjectDeclaration = {
    package: { name: "@destack/object" },
    name: "note",
    typeReference: { packageId: "package-01a0d462-1b26-7401-be27-6258e0a32141", type: "note" },
    hooks: {},
    procedures: { create },
    shared: { replica: { push } },
};

test("declare a package handle with its identity, the router its objects derive, and its adapters", async () => {
    // declare notes with an npm adapter for anyone, a hook adapter for users and a sync adapter for itself
    const npm = defineMount({ name: "npm", path: "/npm" });
    const hook = defineMount({ name: "hook", path: "/hook", callers: [principal.user] });
    const sync = defineMount({ name: "sync", path: "/sync", callers: "self" });
    const handle = definePackage({
        objects: [note],
        mounts: [npm, hook, sync],
        since: "2026.9.0",
    });
    const declaring = PackageVersion.declaring(handle);

    expect({
        identity: [handle.id, handle.name],
        declaring: PackageVersion.declaring(handle),
        since: handle.since,
        routes: Object.keys(handle.router),
        adapters: handle.adapters,
    }).toEqual({
        identity: ["package-01a0d462-1b26-7401-be27-6258e0a32141", "@destack/object"],
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

test("refuse adapters at an empty path, and adapters at or below another's path", async () => {
    // refuse an adapter at the mount itself
    expect(await refusal(() => defineMount({ name: "root", path: "/" }))).toEqual([
        "BAD_REQUEST",
        "DEFINITION_INVALID",
        "mount 'root' path '/' is invalid: it is not below the package",
    ]);

    // refuse an adapter at another's path, and one below it
    const npm = defineMount({ name: "npm", path: "/npm" });
    const twin = defineMount({ name: "twin", path: "/npm" });
    const nested = defineMount({ name: "nested", path: "/npm/tarballs" });
    expect(await refusal(() => definePackage({ mounts: [npm, twin] }))).toEqual([
        "BAD_REQUEST",
        "DEFINITION_INVALID",
        "adapter path '/npm' is invalid: it overlaps another",
    ]);
    expect(await refusal(() => definePackage({ mounts: [npm, nested] }))).toEqual([
        "BAD_REQUEST",
        "DEFINITION_INVALID",
        "adapter path '/npm/tarballs' is invalid: it overlaps another",
    ]);
});

test("refuse an adapter at an object's routes, which would take its calls", async () => {
    const routed = {
        ...note,
        procedures: { create: create.route({ method: "POST", path: "/notes" }) },
    };
    const notes = defineMount({ name: "notes", path: "/notes" });

    expect(await refusal(() => definePackage({ objects: [routed], mounts: [notes] }))).toEqual([
        "BAD_REQUEST",
        "DEFINITION_INVALID",
        "adapter path '/notes' is invalid: it overlaps an object's routes",
    ]);
});

test("refuse a name taken twice in one namespace, resources and secrets sharing one", async () => {
    const npm = defineMount({ name: "npm", path: "/npm" });
    const module = { package: PackageVersion.of(import.meta.destack.package) };
    const api = defineInstallationBinding("api", definePackage({}), {}, module);

    // accept one name in two namespaces, and refuse a resource and a secret sharing one
    expect([
        await refusal(() => definePackage({ mounts: [npm], views: [npm] })),
        await refusal(() => definePackage({ resources: [api, api] })),
    ]).toEqual([
        "done",
        ["BAD_REQUEST", "DEFINITION_INVALID", "declaration 'api' is invalid: it is a duplicate"],
    ]);
});

test("refuse routing an object type under the name of shared procedures, and route each name once", () => {
    // refuse a type taking the shared name
    expect(() => definePackage({ objects: [{ ...note, name: "replica" }] }).router).toThrow(
        new TypeError("package '@destack/object' routes two procedures under 'replica'"),
    );

    // route each type and the shared procedures once
    const notebook = { ...note, name: "notebook" };
    expect(Object.keys(definePackage({ objects: [note, notebook] }).router)).toEqual([
        "note",
        "notebook",
        "replica",
    ]);
});

test("bundle what a capability needs: its sandbox abilities, permissions and features", () => {
    // declare an optional Gmail import calling Gmail's API on paid plans
    const importing = {
        reason: t`Import contacts from Gmail`,
        optional: true,
        sandbox: { network: { connect: ["gmail.googleapis.com"] } },
        features: [
            { packageId: PackageId.parse(note.typeReference.packageId), name: "integrations" },
        ],
    };
    const scanning = { reason: t`Scan receipts`, sandbox: { camera: { reason: "scans" } } };

    // accept the bundle, and refuse a sandbox entry with a separate reason
    expect([
        definePackage({ capabilities: [{ name: "gmail-import", ...importing }] }).capabilities,
        Capability.safeParse(scanning).error?.issues.map((issue) => [
            issue.code,
            issue.path.join("."),
        ]),
    ]).toEqual([{ "gmail-import": importing }, [["unrecognized_keys", "sandbox.camera"]]]);
});
