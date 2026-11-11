import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { defineSetting } from "../../../src/declare/index.ts";
import { t } from "@destack/locale";

/** The package release declaring the fixture's settings. */
export const notes = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@alice/notes",
    version: "2026.9.0",
});

/** A personal editor choice with every contextual refinement. */
export const editor = defineSetting(
    {
        name: "editor.mode",
        title: t`Editor mode`,
        description: t`Keyboard behavior in the note editor.`,
        schema: schema.enum(["standard", "vim"]),
        default: "standard",
        scope: "user",
        overrides: ["space", "installation", "device"],
    },
    { package: notes },
);

/** The person's signature, which each person must set, as it has no default. */
export const signature = defineSetting(
    {
        name: "editor.signature",
        title: t`Signature`,
        description: t`The line closing each note.`,
        schema: schema.string().min(1),
        scope: "user",
        overrides: ["space", "installation", "device"],
        required: true,
    },
    { package: notes },
);

/** A distinct value type resolved alongside the editor mode. */
export const lineNumbers = defineSetting(
    {
        name: "editor.lineNumbers",
        title: t`Line numbers`,
        description: t`Show line numbers in the editor.`,
        schema: schema.boolean(),
        default: true,
        scope: "user",
        overrides: ["device"],
    },
    { package: notes },
);

/** The person's key bindings, each resolved from its own nearest placement. */
export const keybindings = defineSetting(
    {
        name: "editor.keybindings",
        title: t`Key bindings`,
        description: t`The keys running each command, or none to unbind it.`,
        schema: schema.record(schema.string(), schema.string().nullable()),
        default: { "note.archive": "mod+shift+a" },
        scope: "user",
        overrides: ["space", "installation", "device"],
        merge: "key",
    },
    { package: notes },
);

/** How many days a space keeps its notes' history. */
export const history = defineSetting(
    {
        name: "notes.history",
        title: t`History`,
        description: t`How many days a space keeps its notes' history.`,
        schema: schema.number().int().min(1),
        default: 30,
        scope: "space",
    },
    { package: notes },
);
