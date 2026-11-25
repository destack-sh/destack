import { Search } from "@destack/db";
import { InstallationOrigin } from "@destack/account/address";
import { DeclarationName, OriginPath, PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Shortcut } from "@destack/locale";
import { OBJECT_PARAMETER, TARGET_PARAMETER } from "../declare/context.ts";

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

/** The method an action entry calls through an installation, as the palette offers it. */
export const ActionCallEntry = schema.object({
    /** The space of the installation declaring the action. */
    space: SpaceId,
    /** The installation declaring the action, by name. */
    installation: DeclarationName,
    /** The package declaring the object type it calls. */
    typePackageId: PackageId,
    /** The object type it calls. */
    type: schema.string().min(1),
    /** The method it calls. */
    method: schema.string().min(1),
});
/** The method an action entry calls through an installation. */
export type ActionCallEntry = schema.Infer<typeof ActionCallEntry>;

/** An action as the palette offers it: a call through an installation, or the page's own. */
export const ActionEntry = schema.object({
    /** An action. */
    kind: schema.literal("action"),
    /** The package declaring it. */
    packageId: PackageId,
    /** The action's name in its package. */
    name: DeclarationName,
    /** The title the palette shows, in the person's language. */
    title: schema.string().min(1),
    /** The group the palette shows the title under, in the person's language. */
    category: schema.string().min(1).nullable(),
    /** The key combination that runs it. */
    keybinding: Shortcut.nullable(),
    /** The method it calls, null for an action the page under the palette handles. */
    call: ActionCallEntry.nullable(),
});
/** An action as the palette offers it. */
export type ActionEntry = schema.Infer<typeof ActionEntry>;

/** An entry the command palette offers. */
export const PaletteEntry = schema.discriminatedUnion("kind", [
    ActionEntry,
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
const paletteFocus = schema.object({
    /** The space of the window's view or object. */
    space: SpaceId.exactOptional(),
    /** The installation of the window's view, by name. */
    installation: DeclarationName.exactOptional(),
    /** The objects the window acts on: its page's selection, else the object it presents. */
    objects: schema.array(ObjectReference),
});

/** What the window the palette opens over shows. */
export const PaletteFocus = Object.assign(paletteFocus, {
    /** Read what a window shows in a space from its location under a domain. */
    of(
        location: string | null,
        space: Identifier<"space">,
        domain: string,
    ): schema.Infer<typeof paletteFocus> {
        // read nothing from a window showing no location yet
        if (location === null) {
            return { objects: [] };
        }
        const url = new URL(location);

        // read the installation of its origin
        const origin = InstallationOrigin.parse(url.hostname, domain);

        // read the object of an opening or the one a view opens for
        const parameter = url.pathname === OriginPath.open ? OBJECT_PARAMETER : TARGET_PARAMETER;
        const object = readReference(url.searchParams.get(parameter));

        return {
            space,
            ...(origin === undefined ? {} : { installation: DeclarationName.parse(origin.name) }),
            objects: object === undefined ? [] : [object],
        };
    },

    /** List the identifiers of the focused objects an action's call acts on, those of its type. */
    ids(focus: schema.Infer<typeof paletteFocus>, call: ActionCallEntry): string[] {
        return focus.objects
            .filter(
                (object) => object.packageId === call.typePackageId && object.type === call.type,
            )
            .map((object) => object.id);
    },
});
/** What the window the palette opens over shows. */
export type PaletteFocus = schema.Infer<typeof paletteFocus>;

/** The palette's ranking of entries and their sources. */
export const Palette = {
    /** Rank the entries that match a query. */
    rank(entries: readonly PaletteEntry[], query: string, focus: PaletteFocus): PaletteEntry[] {
        const search = Search.of(query);
        const scored = entries.flatMap((entry) => {
            const match = Palette.match(entry, search);

            return match === undefined
                ? []
                : [{ entry, score: match + Palette.boost(entry, focus) }];
        });

        return scored
            .toSorted(
                (left, right) =>
                    right.score - left.score || left.entry.title.localeCompare(right.entry.title),
            )
            .map(({ entry }) => entry);
    },

    /** Score how an entry's words match a search, undefined where they do not. */
    match(entry: PaletteEntry, search: Search): number | undefined {
        const installation = "installation" in entry ? entry.installation : "";
        const category = "category" in entry ? (entry.category ?? "") : "";

        return search.score([entry.title, installation, category]);
    },

    /** Score how an entry relates to the focused window. */
    boost(entry: PaletteEntry, focus: PaletteFocus): number {
        // read the entry's space and installation against the focus
        const space = Palette.space(entry);
        const installation =
            entry.kind === "view"
                ? entry.installation
                : entry.kind === "action"
                  ? entry.call?.installation
                  : undefined;
        const isSpace = space !== undefined && space === focus.space;
        const isInstallation = isSpace && installation === focus.installation;

        // read whether an action's call acts on the focused objects' type
        const call = entry.kind === "action" ? entry.call : null;
        const isObject = call !== null && PaletteFocus.ids(focus, call).length > 0;

        return (
            (isSpace ? BOOSTS.space : 0) +
            (isInstallation ? BOOSTS.installation : 0) +
            (isObject ? BOOSTS.object : 0)
        );
    },

    /** Read the space an entry belongs to, absent for a window or an action of the page. */
    space(entry: PaletteEntry): Identifier<"space"> | undefined {
        if (entry.kind === "window") {
            return undefined;
        } else if (entry.kind === "action") {
            return entry.call?.space;
        } else if (entry.kind === "space") {
            return entry.id;
        }

        return entry.space;
    },

    /** Read the source an entry comes from: its kind's, or the package declaring it. */
    source(entry: PaletteEntry): PaletteSource {
        const sources = { window: "windows", space: "spaces", object: "recent" } as const;

        return entry.kind === "view" || entry.kind === "action"
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
};

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
