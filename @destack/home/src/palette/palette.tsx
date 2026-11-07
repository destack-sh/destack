import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import { CommandReference } from "@destack/object";
import { present, schema } from "@destack/schema";
import {
    createEffect,
    createSignal,
    onCleanup,
    Show,
    useEnvironment,
    useLocale,
    useOrigin,
    useSetting,
    useView,
} from "@destack/view";
import {
    COMMAND_PARAMETER,
    FOCUS_PARAMETER,
    INSTALLATION_PARAMETER,
    OBJECT_PARAMETER,
    OPEN_PATH,
    SPACE_PARAMETER,
    VIEW_PARAMETER,
} from "@destack/view/declare";
import {
    keybindings,
    type ObjectEntry,
    PaletteFocus,
    sources,
    type ViewEntry,
} from "@destack/view/palette";
import { CommandPalette } from "./command-palette.tsx";
import { PaletteListing } from "./listing.ts";
import { ParentPage } from "./parent.ts";

/** Run the command palette over the person's home and device, reached through the page's origin. */
export default function Commands() {
    // read the command the shell asks to run, the window it opens over and the page's origin
    const view = useView();
    const environment = useEnvironment();
    const space = schema.identifier("space").parse(view.space);
    const page = new URL(environment.navigation.currentEntry.url);
    const query = page.searchParams;
    const asked = query.get(COMMAND_PARAMETER);
    const command = asked === null ? undefined : parseCommand(asked);
    const domain = page.hostname.slice(page.hostname.indexOf(".") + 1);
    const over = query.get(FOCUS_PARAMETER);
    const focus = PaletteFocus.of(over, space, domain);
    const parent = ParentPage.of(over);
    const close = () =>
        parent === undefined ? environment.navigation.navigate("about:blank") : parent.close();
    const origin = useOrigin();
    const localization = useLocale();

    // read the entries of the person's home, whose origin this page has, and of their device, once their choices are read
    const [listing, setListing] = createSignal<{
        readonly listing: PaletteListing;
        readonly focus: PaletteFocus;
    }>();
    const own = present(InstallationOrigin.parse(page.hostname, domain), "the page's origin");
    const person = {
        user: view.user,
        home: {
            id: schema.identifier("space").parse(present(view.home, "the person's home")),
            address: SpaceAddress.of(own),
        },
        device: view.device,
    };
    const stopping = new AbortController();
    onCleanup(() => stopping.abort());
    const bound = useSetting(keybindings);
    const offered = useSetting(sources);
    createEffect(
        () => ({ keybindings: bound(), sources: offered() }),
        (chosen) => {
            if (chosen.keybindings !== undefined && chosen.sources !== undefined) {
                // read the listing with the commands and the selection of the page under the palette
                const preferences = { keybindings: chosen.keybindings, sources: chosen.sources };
                const listed = parent?.list(stopping.signal) ?? Promise.resolve(undefined);
                listed
                    .then(async (under) => {
                        const selected =
                            under === undefined || under.selection.length === 0
                                ? focus
                                : { ...focus, objects: [...under.selection] };
                        const read = await PaletteListing.read(origin, person, preferences, {
                            environment,
                            localization,
                            focus: selected,
                            page: under,
                        });

                        return { listing: read, focus: selected };
                    })
                    .then(setListing, (error: unknown) => globalThis.reportError(error));
            }
        },
    );

    return (
        <Show when={listing()}>
            {(read) => (
                <CommandPalette
                    listing={read().listing}
                    focus={read().focus}
                    onOpen={(entry) => openEntry(read().listing, entry, page.origin)}
                    onWindow={(entry) => {
                        read()
                            .listing.focus(entry)
                            .then(close, (error: unknown) => globalThis.reportError(error));
                    }}
                    onRegistered={(entry, input) =>
                        present(parent, "the page registering the command").run(entry, input)
                    }
                    onClose={close}
                    {...(command === undefined ? {} : { command })}
                />
            )}
        </Show>
    );
}

/** Open a view, or an object in the view presenting it, through the page's open path. */
function openEntry(listing: PaletteListing, entry: ViewEntry | ObjectEntry, origin: string): void {
    // name the view of an installation, or the object
    const space = listing.catalog(entry).space.address;
    const query = new URLSearchParams(
        entry.kind === "view"
            ? {
                  [SPACE_PARAMETER]: space,
                  [INSTALLATION_PARAMETER]: entry.installation,
                  [VIEW_PARAMETER]: entry.name,
              }
            : { [SPACE_PARAMETER]: space, [OBJECT_PARAMETER]: JSON.stringify(entry.object) },
    );

    // open it in the window the palette opens in, or over
    (window.top ?? window).location.href = new URL(`${OPEN_PATH}?${query}`, origin).href;
}

/** Read a command named as `<package>/<name>`. */
function parseCommand(text: string): CommandReference {
    const [packageId, name] = text.split("/");

    return CommandReference.parse({ packageId, name });
}
