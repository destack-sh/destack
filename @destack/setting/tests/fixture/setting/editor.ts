import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { defineSetting } from "../../../src/declare/index.ts";

/** The package release declaring the fixture's settings. */
export const notes = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** A personal editor choice with every contextual refinement. */
export const editor = defineSetting(
    {
        name: "editor.mode",
        title: "Editor mode",
        description: "Keyboard behavior in the note editor.",
        schema: schema.enum(["standard", "vim"]),
        default: "standard",
        scope: "user",
        overrides: ["space", "installation", "device"],
        apply: "immediate",
    },
    { package: notes },
);

/** A distinct value type resolved alongside the editor mode. */
export const lineNumbers = defineSetting(
    {
        name: "editor.lineNumbers",
        title: "Line numbers",
        description: "Show line numbers in the editor.",
        schema: schema.boolean(),
        default: true,
        scope: "user",
        overrides: ["device"],
        apply: "immediate",
    },
    { package: notes },
);

/** The person's key bindings of commands, each command's binding resolved from its own nearest placement. */
export const keybindings = defineSetting(
    {
        name: "editor.keybindings",
        title: "Key bindings",
        description: "The keys running each command, or none to unbind it.",
        schema: schema.record(schema.string(), schema.string().nullable()),
        default: { "note.archive": "mod+shift+a" },
        scope: "user",
        overrides: ["space", "installation", "device"],
        apply: "immediate",
        merge: "key",
    },
    { package: notes },
);
