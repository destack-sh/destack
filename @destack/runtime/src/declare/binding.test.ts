import { principal } from "@destack/access";
import { expect, onTestFinished, test } from "@destack/test";
import { Package } from "@destack/package";
import { ResourceId } from "@destack/resource";
import { health } from "../health/index.ts";
import { VERSION_HEADER } from "../request/index.ts";
import { defineProcedure, type ProcedureRouter } from "../runtime/index.ts";
import { defineInstallationBinding } from "./binding.ts";
import { definePackage, type ObjectDeclaration } from "./package.ts";

/** The identity of the notes package. */
const NOTES = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b001",
    name: "@destack/notes",
    version: "2026.9.0",
});

/** The identity of a package the binding does not bind. */
const OTHER = Package.parse({
    ...NOTES,
    id: "package-01996ab0-0000-7000-8000-00000000b002",
    name: "@destack/other",
});

/** An installation binding at a reference, as a host hands it to a workload. */
const bound = (reference: string, credential?: string) => ({
    resource: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
    kind: "installation",
    provider: "http",
    reference,
    ...(credential === undefined ? {} : { credential }),
});

/** A package's note type routing a set of procedures. */
const note = <Router extends ProcedureRouter>(owner: Package, procedures: Router) => {
    const declaration = {
        package: owner,
        name: "note",
        typeReference: { packageId: owner.id, type: "note" },
        hooks: {},
        procedures,
        shared: {},
    };

    return declaration satisfies ObjectDeclaration;
};

/** The notes package, whose note type routes the health procedures. */
const notes = definePackage({ objects: { note: note(NOTES, health) } }, { package: NOTES });

test("call a bound installation at its reference with the credential the host lends, speaking the package's release", async () => {
    // answer health checks
    const calls: (string | null)[][] = [];
    const server = Bun.serve({
        port: 0,
        fetch: (request) => {
            calls.push([request.headers.get("authorization"), request.headers.get(VERSION_HEADER)]);

            return Response.json({ name: "notes", status: "ready" });
        },
    });
    onTestFinished(() => server.stop(true));

    // connect through the binding's HTTP connector and call the package
    const binding = defineInstallationBinding("notes", notes);
    const url = `http://127.0.0.1:${server.port}`;
    const client = await binding.connectors.http.connect(bound(url, "secret"), binding);
    const answer = await client.note.check();
    expect([answer, calls, binding.kind, binding.spec]).toEqual([
        { name: "notes", status: "ready" },
        [["Bearer secret", notes.version]],
        "installation",
        { package: notes.id },
    ]);
});

test("list a binding's calls by their paths in the package's router, and refuse a procedure the package does not route", () => {
    // declare calls of the package's health procedures and of another package's
    const send = defineProcedure({
        authentication: "identity",
        permission: { principals: [principal.user, principal.machine, principal.installation] },
        audit: false,
    });
    const other = definePackage({ objects: { note: note(OTHER, { send }) } }, { package: OTHER });
    const binding = defineInstallationBinding("notes", notes, {
        calls: [notes.router.note.check, notes.router.note.watch],
    });
    const stray = () =>
        defineInstallationBinding("notes", notes, { calls: [other.router.note.send] });

    // keep the dotted paths in the binding's spec and refuse the stray procedure
    expect(binding.spec.calls).toEqual(["note.check", "note.watch"]);
    expect(stray).toThrow(
        "installation binding notes calls a procedure outside its package's routes",
    );
});

test("refuse connecting an installation binding the host lends no credential", async () => {
    const binding = defineInstallationBinding("notes", notes);

    await expect(
        binding.connectors.http.connect(bound("http://127.0.0.1"), binding),
    ).rejects.toThrow("installation binding notes has no credential");
});
