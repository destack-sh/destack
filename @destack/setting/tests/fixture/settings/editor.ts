import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { defineSetting } from "../../../src/declare/index.ts";

/** The package release declaring the fixture's settings. */
export const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** A personal editor choice with every supported contextual refinement. */
export const editor = defineSetting({
    package: notes,
    name: "editor.mode",
    title: "Editor mode",
    description: "Keyboard behavior in the note editor.",
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["space", "installation", "device"],
    apply: "immediate",
});

/** A distinct value type resolved alongside the editor mode. */
export const lineNumbers = defineSetting({
    package: notes,
    name: "editor.lineNumbers",
    title: "Line numbers",
    description: "Show line numbers in the editor.",
    schema: schema.boolean(),
    default: true,
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
});
