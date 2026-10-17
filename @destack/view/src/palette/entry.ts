import { InstallationOrigin } from "@destack/account/address";
import { DeclarationName, PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { CommandReference } from "../declare/command.ts";
import { OBJECT_PARAMETER, OPEN_PATH, TARGET_PARAMETER } from "../declare/context.ts";
import { Accelerator } from "./accelerator.ts";

/** How strongly a title matching the query from its start ranks. */
const MATCHES = { start: 3, word: 2, inside: 1 } as const;

/** How much higher an entry ranks for the focused window. */
const BOOSTS = { space: 0.25, installation: 0.5, object: 1 } as const;

/** The identity of an entry's space. */
const SpaceId = schema.identifier("space");

/** A source of the palette's entries, as the person turns it on or off: a kind, or a package. */
export const PaletteSource = schema.union([
    schema.enum(["windows", "spaces", "recent"]),
    PackageId,
]);
/** A source of the palette's entries. */
export type PaletteSource = schema.Infer<typeof PaletteSource>;

/** An entry the command palette offers. */
export const PaletteEntry = schema.discriminatedUnion("kind", [
    schema.object({
        /** A command calling an object type's method. */
        kind: schema.literal("command"),
        /** The space of the installation declaring it. */
        space: SpaceId,
        /** The installation declaring it, by name. */
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
        /** The installation declaring it, by name. */
        installation: DeclarationName,
        /** The declaring package. */
        packageId: PackageId,
        /** The view's name. */
        name: DeclarationName,
        /** The title the palette shows. */
        title: schema.string().min(1),
    }),
    schema.object({
        /** A window of this computer, coming into focus. */
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
]);
/** An entry the command palette offers. */
export type PaletteEntry = schema.Infer<typeof PaletteEntry>;

/** What the window the palette opens over shows. */
export const PaletteFocus = Object.assign(
    schema.object({
        /** The space of the window's view or object. */
        space: SpaceId.exactOptional(),
        /** The installation of the window's view, by name. */
        installation: DeclarationName.exactOptional(),
        /** The object the window presents. */
        object: ObjectReference.exactOptional(),
    }),
    {
        /** Read what a window shows in a space from its location under a domain. */
        of(location: string | null, space: Identifier<"space">, domain: string): PaletteFocus {
            // read nothing from a window showing no location yet
            if (location === null) {
                return {};
            }
            const url = new URL(location);

            // read the installation of its origin
            const origin = InstallationOrigin.parse(url.hostname, domain);

            // read the object of an opening or the one a view opens for
            const parameter = url.pathname === OPEN_PATH ? OBJECT_PARAMETER : TARGET_PARAMETER;
            const object = readReference(url.searchParams.get(parameter));

            return {
                space,
                ...(origin === undefined
                    ? {}
                    : { installation: DeclarationName.parse(origin.name) }),
                ...(object === undefined ? {} : { object }),
            };
        },
    },
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

        // read the object a command acts on
        const { object } = focus;
        const isObject =
            object !== undefined &&
            entry.kind === "command" &&
            object.packageId === entry.typePackageId &&
            object.type === entry.type;

        return (
            (isSpace ? BOOSTS.space : 0) +
            (isInstallation ? BOOSTS.installation : 0) +
            (isObject ? BOOSTS.object : 0)
        );
    },

    /** Read the space an entry belongs to, absent for a window of this computer. */
    space(entry: PaletteEntry): Identifier<"space"> | undefined {
        if (entry.kind === "window") {
            return undefined;
        } else if (entry.kind === "space") {
            return entry.id;
        }

        return entry.space;
    },

    /** Read the source an entry comes from: its kind's, or the package declaring it. */
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

/** A window of this computer the palette offers. */
export type WindowEntry = Extract<PaletteEntry, { readonly kind: "window" }>;

/** A space of the person the palette offers. */
export type SpaceEntry = Extract<PaletteEntry, { readonly kind: "space" }>;

/** An object opened lately the palette offers. */
export type ObjectEntry = Extract<PaletteEntry, { readonly kind: "object" }>;

/** Read an object reference written as JSON, absent for none. */
function readReference(json: string | null): ObjectReference | undefined {
    return json === null ? undefined : ObjectReference.parse(JSON.parse(json));
}
