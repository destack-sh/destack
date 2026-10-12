import { DeclarationName, PackageId } from "@destack/package";
import {
    defineSchema,
    type Identifier,
    type JsonObject,
    type JsonValue,
    schema,
} from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { CommandReference } from "../declare/command.ts";
import { Accelerator } from "./accelerator.ts";

/** How strongly a title matching the query from its start ranks. */
const MATCHES = { start: 3, word: 2, inside: 1 } as const;

/** How much higher an entry ranks for the focused window. */
const BOOSTS = { space: 0.25, installation: 0.5, object: 1 } as const;

/** The identity of a space, which entries name their space by. */
const SpaceId = schema.identifier("space");

/** A source of the palette's entries, as the person turns it on or off. */
export const PaletteSource = defineSchema(
    schema.union([schema.enum(["windows", "spaces", "recent"]), PackageId]),
);
/** A source of the palette's entries. */
export type PaletteSource = schema.Infer<typeof PaletteSource>;

/** An entry the command palette offers. */
export const PaletteEntry = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A command calling an object type's method. */
            kind: schema.literal("command"),
            /** The space of the installation declaring it. */
            space: SpaceId,
            /** The installation declaring it, by alias. */
            installation: DeclarationName,
            /** The declaring package. */
            packageId: PackageId,
            /** The command's name in its package. */
            name: DeclarationName,
            /** The title the palette shows. */
            title: schema.string().min(1),
            /** The package declaring the object type it calls. */
            typePackageId: PackageId,
            /** The object type it calls. */
            type: schema.string().min(1),
            /** The method it calls. */
            method: schema.string().min(1),
            /** The key combination that runs it. */
            keybinding: Accelerator.nullable(),
        }),
        schema.object({
            /** A view opening in a window. */
            kind: schema.literal("view"),
            /** The space of the installation declaring it. */
            space: SpaceId,
            /** The installation declaring it, by alias. */
            installation: DeclarationName,
            /** The declaring package. */
            packageId: PackageId,
            /** The view's name. */
            name: DeclarationName,
            /** The title the palette shows. */
            title: schema.string().min(1),
        }),
        schema.object({
            /** A window of this machine, coming into focus. */
            kind: schema.literal("window"),
            /** The window's identifier. */
            id: schema.string().min(1),
            /** The title the palette shows. */
            title: schema.string().min(1),
            /** The link the window shows. */
            link: schema.string().min(1),
        }),
        schema.object({
            /** A space of the person, narrowing the palette to its entries. */
            kind: schema.literal("space"),
            /** The space. */
            id: SpaceId,
            /** The title the palette shows: the space's address. */
            title: schema.string().min(1),
        }),
        schema.object({
            /** An object the person opened lately, opening in the view presenting it. */
            kind: schema.literal("object"),
            /** The space keeping it. */
            space: SpaceId,
            /** The object. */
            object: ObjectReference,
            /** The title the palette shows. */
            title: schema.string().min(1),
        }),
    ]),
);
/** An entry the command palette offers. */
export type PaletteEntry = schema.Infer<typeof PaletteEntry>;

/** What the window the palette opens over shows. */
export const PaletteFocus = defineSchema(
    schema.object({
        /** The space of the window's view or object. */
        space: SpaceId.exactOptional(),
        /** The installation of the window's view, by alias. */
        installation: DeclarationName.exactOptional(),
        /** The object the window presents. */
        object: ObjectReference.exactOptional(),
    }),
);
/** What the window the palette opens over shows. */
export type PaletteFocus = schema.Infer<typeof PaletteFocus>;

/** The palette's ranking of entries, their sources and their keybindings. */
export const Palette = {
    /** Rank the entries that match a query. */
    rank(entries: readonly PaletteEntry[], query: string, focus: PaletteFocus): PaletteEntry[] {
        const scored = entries.flatMap((entry) => {
            const match = Palette.match(entry, query);

            return match === 0 ? [] : [{ entry, score: match + Palette.boost(entry, focus) }];
        });

        return scored
            .toSorted(
                (left, right) =>
                    right.score - left.score || left.entry.title.localeCompare(right.entry.title),
            )
            .map(({ entry }) => entry);
    },

    /** Score how an entry's title or installation matches a query. */
    match(entry: PaletteEntry, query: string): number {
        // match every entry for an empty query
        const wanted = query.trim().toLowerCase();
        const installation = "installation" in entry ? entry.installation : "";
        const text = `${entry.title} ${installation}`.toLowerCase();
        if (wanted === "") {
            return MATCHES.inside;
        }

        // prefer a match at the start, then at a word's start
        if (text.startsWith(wanted)) {
            return MATCHES.start;
        } else if (text.split(/\s+/u).some((word) => word.startsWith(wanted))) {
            return MATCHES.word;
        }

        return text.includes(wanted) ? MATCHES.inside : 0;
    },

    /** Score how an entry relates to the focused window. */
    boost(entry: PaletteEntry, focus: PaletteFocus): number {
        // read the entry's space and installation against the focus
        const space = Palette.space(entry);
        const isSpace = space !== undefined && space === focus.space;
        const isInstallation =
            isSpace && "installation" in entry && entry.installation === focus.installation;

        // read the object it names or acts on
        const { object } = focus;
        const isObject =
            object !== undefined &&
            ((entry.kind === "command" &&
                object.packageId === entry.typePackageId &&
                object.type === entry.type) ||
                (entry.kind === "object" &&
                    ObjectReference.key(object) === ObjectReference.key(entry.object)));

        return (
            (isSpace ? BOOSTS.space : 0) +
            (isInstallation ? BOOSTS.installation : 0) +
            (isObject ? BOOSTS.object : 0)
        );
    },

    /** Read the space an entry belongs to, absent for a window of this machine. */
    space(entry: PaletteEntry): Identifier<"space"> | undefined {
        if (entry.kind === "window") {
            return undefined;
        } else if (entry.kind === "space") {
            return entry.id;
        }

        return entry.space;
    },

    /** Read the source an entry comes from. */
    source(entry: PaletteEntry): PaletteSource {
        const sources = { window: "windows", space: "spaces", object: "recent" } as const;

        return entry.kind === "command" || entry.kind === "view"
            ? entry.packageId
            : sources[entry.kind];
    },

    /** Report whether the person's sources offer an entry: every source but those turned off. */
    isOffered(
        entry: PaletteEntry,
        sources: Readonly<Partial<Record<PaletteSource, boolean>>>,
    ): boolean {
        return sources[Palette.source(entry)] !== false;
    },

    /** Read the key combination that runs a command. */
    keybinding(
        reference: CommandReference,
        declared: Accelerator | undefined,
        overrides: Readonly<Record<string, Accelerator | null>>,
    ): Accelerator | null {
        const key = CommandReference.format(reference);

        return key in overrides ? (overrides[key] ?? null) : (declared ?? null);
    },
};

/** A command the palette offers. */
export type CommandEntry = Extract<PaletteEntry, { readonly kind: "command" }>;

/** A view the palette offers. */
export type ViewEntry = Extract<PaletteEntry, { readonly kind: "view" }>;

/** A window of this machine the palette offers. */
export type WindowEntry = Extract<PaletteEntry, { readonly kind: "window" }>;

/** A space of the person the palette offers. */
export type SpaceEntry = Extract<PaletteEntry, { readonly kind: "space" }>;

/** An object opened lately the palette offers. */
export type ObjectEntry = Extract<PaletteEntry, { readonly kind: "object" }>;

/** An object a command's picker offers. */
export interface PaletteObject {
    /** The object's identifier. */
    readonly id: string;
    /** The text naming it. */
    readonly title: string;
}

/** How a command's method takes its input. */
export interface CommandShape {
    /** Whether the method acts on one object, which the palette picks. */
    readonly isTargeted: boolean;
    /** The JSON Schema of the input beside the object. */
    readonly input: JsonObject;
}

/** The entries the palette offers. */
export interface PaletteListing {
    /** The entries. */
    readonly entries: readonly PaletteEntry[];
    /** Why the entries of some spaces are missing, one message each. */
    readonly failures: readonly string[];
}

/** What the palette reaches as the person. */
export interface PaletteClient {
    /** List the entries the palette offers. */
    list(): Promise<PaletteListing>;
    /** Read how a command's method takes its input. */
    shape(command: CommandEntry): Promise<CommandShape>;
    /** List the objects of a command's type whose title matches a text. */
    objects(command: CommandEntry, title: string): Promise<readonly PaletteObject[]>;
    /** Call a command's method with its input, the chosen object's identifier among it. */
    call(command: CommandEntry, input: JsonObject): Promise<JsonValue>;
    /** Open a view, or an object in the view presenting it. */
    open(entry: ViewEntry | ObjectEntry): Promise<void>;
    /** Bring a window of this machine into focus. */
    focus(window: WindowEntry): Promise<void>;
    /** Close the palette. */
    close(): void;
}
