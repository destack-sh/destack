import { Failure } from "@destack/error";
import type { Localization } from "@destack/locale";
import { ObjectTypePool, type ObjectType } from "@destack/object";
import { ObjectConnection } from "@destack/object/client";
import { schema } from "@destack/schema";
import { ObjectReference, type Subject } from "@destack/sync";
import { HOME_PATH } from "@destack/view/declare";
import {
    type ActionEntry,
    type CommandEntry,
    type ObjectEntry,
    Palette,
    type PaletteEntry,
    PaletteFocus,
    type PaletteSource,
    type ViewEntry,
    type WindowEntry,
} from "@destack/view/palette";
import { type PaletteSpace, SpaceCatalog } from "./catalog.ts";

/** The most objects the person opened lately that the palette offers: a screenful. */
const RECENT_LIMIT = 20;

/** A window of this computer, as its local objects list it. */
const WindowRow = schema.looseObject({
    /** The window. */
    id: schema.string().min(1),
    /** The link it shows. */
    link: schema.string().min(1),
    /** The title the desktop shows, absent until it opens the window. */
    title: schema.string().nullable(),
});

/** An object a person opened on this computer, as its local objects list it. */
const VisitRow = schema.looseObject({
    /** The space whose origin opened it. */
    spaceId: schema.identifier("space"),
    /** The object. */
    object: ObjectReference,
});

/** What the person chose of the palette: their keybindings, and the sources they turned off. */
export interface PalettePreferences {
    /** The key combinations running commands, by command. */
    readonly keybindings: Readonly<Record<string, string | null>>;
    /** The sources by name, false for one turned off. */
    readonly sources: Readonly<Partial<Record<PaletteSource, boolean>>>;
}

/** The palette's entries of the person's home and device, and why some are missing. */
export class PaletteListing {
    /** The entries of the sources the person leaves on. */
    readonly entries: readonly PaletteEntry[];
    /** Why the entries of some sources are missing, one message each. */
    readonly failures: readonly string[];
    /** The catalog of the person's home. */
    readonly #home: SpaceCatalog | undefined;
    /** The local objects of the device hosting the palette, absent outside one. */
    readonly #local: LocalObjects | undefined;

    /** Create a listing from its entries, its failures and where they come from. */
    constructor(
        entries: readonly PaletteEntry[],
        failures: readonly string[],
        from: {
            readonly home: SpaceCatalog | undefined;
            readonly local: LocalObjects | undefined;
        },
    ) {
        // set the entries and failures, and the catalog and local objects they come from
        this.entries = entries;
        this.failures = failures;
        this.#home = from.home;
        this.#local = from.local;
    }

    /** Read the person's home, their device's objects and the actions of the page under it. */
    static async read(
        connect: (path: string, scope: string) => ObjectConnection,
        person: {
            readonly user: Subject;
            readonly home: PaletteSpace;
            readonly device: string | undefined;
        },
        preferences: PalettePreferences,
        reading: {
            readonly localization: Localization;
            readonly focus: PaletteFocus;
            readonly actions: readonly ActionEntry[];
        },
    ): Promise<PaletteListing> {
        // read the person's home's catalog
        const { localization, focus, actions } = reading;
        const [home] = await Promise.allSettled([SpaceCatalog.read(person.home, connect)]);
        const catalog = home.status === "fulfilled" ? home.value : undefined;

        // read the device's windows and the objects the person opened lately, or why not
        const { device } = person;
        const [local] =
            device === undefined
                ? []
                : await Promise.allSettled([
                      PaletteListing.#readLocal(connect, device, person.user, catalog),
                  ]);

        // offer the page's actions, and the home's commands applying to the focused object
        const declared =
            catalog === undefined
                ? []
                : await PaletteListing.#applying(
                      catalog,
                      catalog.entries(preferences.keybindings, localization),
                      focus,
                  );

        // offer the entries of every source left on, and name each failure
        const entries: PaletteEntry[] = [
            ...actions,
            ...(local?.status === "fulfilled" ? local.value.entries : []),
            ...(catalog === undefined
                ? []
                : [
                      {
                          kind: "space",
                          id: catalog.space.id,
                          title: catalog.space.address,
                      } as const,
                      ...declared,
                  ]),
        ];
        const failures = [home, ...(local === undefined ? [] : [local])].flatMap((result) =>
            result.status === "rejected" ? [Failure.of(result.reason).message] : [],
        );

        return new PaletteListing(
            entries.filter((entry) => Palette.isOffered(entry, preferences.sources)),
            failures,
            {
                home: catalog,
                local: local?.status === "fulfilled" ? local.value.reached : undefined,
            },
        );
    }

    /** Bring a window of this computer into focus through its local objects. */
    async focus(entry: WindowEntry): Promise<void> {
        // require the device's local objects
        if (this.#local === undefined) {
            throw new TypeError("the palette reads no device's windows");
        }

        await this.#local.focus(entry.id);
    }

    /** Find the catalog of the space a command, a view or an object belongs to. */
    catalog(entry: CommandEntry | ViewEntry | ObjectEntry): SpaceCatalog {
        if (this.#home === undefined || this.#home.space.id !== entry.space) {
            throw new TypeError(`the palette read no space '${entry.space}'`);
        }

        return this.#home;
    }

    /** Leave out the commands on the focused objects' type that do not apply to all of them. */
    static async #applying(
        catalog: SpaceCatalog,
        entries: readonly PaletteEntry[],
        focus: PaletteFocus,
    ): Promise<PaletteEntry[]> {
        // check each command on the focused objects' type against those objects
        const checked = await Promise.all(
            entries.map((entry) => {
                const ids = entry.kind === "command" ? PaletteFocus.ids(focus, entry) : [];

                return entry.kind === "command" && ids.length > 0
                    ? catalog.appliesTo(entry, ids)
                    : Promise.resolve(true);
            }),
        );

        return entries.filter((_entry, index) => checked[index] === true);
    }

    /** Read the device's windows and the person's recent objects through its local objects. */
    static async #readLocal(
        connect: (path: string, scope: string) => ObjectConnection,
        device: string,
        user: Subject,
        home: SpaceCatalog | undefined,
    ): Promise<{ readonly reached: LocalObjects; readonly entries: PaletteEntry[] }> {
        const reached = await LocalObjects.open(connect, device);
        const [windows, recent] = await Promise.all([
            reached.windows(),
            reached.recent(user, home),
        ]);

        return { reached, entries: [...windows, ...recent] };
    }
}

/** The local objects of the device hosting the palette: its windows and the person's visits. */
class LocalObjects {
    /** The device's home. */
    readonly #remote: ObjectConnection;
    /** The device's window type, as its store describes it. */
    readonly #window: ObjectType;
    /** The device's visit type, as its store describes it. */
    readonly #visit: ObjectType;

    /** Reach the device's windows and visits through its home. */
    private constructor(remote: ObjectConnection, window: ObjectType, visit: ObjectType) {
        this.#remote = remote;
        this.#window = window;
        this.#visit = visit;
    }

    /** Open the device's local objects at its home, by the types its store describes. */
    static async open(
        connect: (path: string, scope: string) => ObjectConnection,
        device: string,
    ): Promise<LocalObjects> {
        // describe the local types the device's store serves
        const remote = connect(`${HOME_PATH}/${device}`, device);
        const types = ObjectTypePool.of([]).declare(await remote.describe());

        // find the window and visit types
        const typed = (name: string) => {
            const type = types.find((each) => each.name === name);
            if (type === undefined) {
                throw new TypeError(`the device keeps no '${name}' objects`);
            }

            return type;
        };

        return new LocalObjects(remote, typed("window"), typed("visit"));
    }

    /** List the windows the device shows. */
    async windows(): Promise<PaletteEntry[]> {
        const rows = await this.#remote.list(this.#window, { where: { status: "open" } });

        return rows.map((row): PaletteEntry => {
            const { id, link, title } = WindowRow.parse(row);

            return { kind: "window", id, link, title: title ?? link };
        });
    }

    /** List the objects the person opened lately on the device, titled by their home. */
    async recent(user: Subject, home: SpaceCatalog | undefined): Promise<PaletteEntry[]> {
        // read the person's latest visits in their home
        const rows = await this.#remote.list(this.#visit, {
            where: { subjectId: user.id },
            orderBy: { openedAt: "desc" },
            limit: RECENT_LIMIT,
        });
        const visits = rows
            .map((row) => VisitRow.parse(row))
            .filter((visit) => visit.spaceId === home?.space.id);

        // title each visited object the home still lists, the latest first
        const titles =
            home === undefined
                ? new Map<string, string>()
                : await home.titles(visits.map((visit) => visit.object));

        return visits.flatMap((visit): PaletteEntry[] => {
            const title = titles.get(ObjectReference.key(visit.object));

            return title === undefined
                ? []
                : [{ kind: "object", space: visit.spaceId, object: visit.object, title }];
        });
    }

    /** Bring a window of this computer into focus. */
    async focus(id: string): Promise<void> {
        await this.#remote.run(this.#window, "focus", { id });
    }
}
