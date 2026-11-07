import type { PackageId } from "@destack/package";
import type { JsonObject } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import {
    PaletteMessage,
    type RegisteredCommand,
    type RegisteredEntry,
} from "@destack/view/palette";

/** The commands a page's view registers, by the package serving it. */
export interface PageCommands {
    /** The package serving the view. */
    readonly packageId: PackageId;
    /** The commands the view registers. */
    readonly commands: readonly RegisteredCommand[];
    /** The objects the person selected in the page. */
    readonly selection: readonly ObjectReference[];
}

/** The page the palette is framed over, which registers commands and closes the palette. */
export class ParentPage {
    /** The parent window. */
    readonly #window: Window;
    /** The parent page's origin, the only one the palette talks to. */
    readonly #origin: string;

    /** Talk to a parent window at its origin. */
    constructor(parent: Window, origin: string) {
        this.#window = parent;
        this.#origin = origin;
    }

    /** Find the page the palette is framed over, at the origin of the location it opens over. */
    static of(focus: string | null): ParentPage | undefined {
        return window.parent === window || focus === null
            ? undefined
            : new ParentPage(window.parent, new URL(focus).origin);
    }

    /** List the commands the page's view registers, once the page answers. */
    list(signal: AbortSignal): Promise<PageCommands> {
        return new Promise((resolve, reject) => {
            // take the first listing the page posts from its origin
            const listener = (event: MessageEvent) => {
                // skip any message but the page's listing
                const message = PaletteMessage.safeParse(event.data);
                if (
                    event.source !== this.#window ||
                    event.origin !== this.#origin ||
                    !message.success ||
                    message.data.action !== "commands.listed"
                ) {
                    return;
                }
                // stop listening and answer the page's commands
                window.removeEventListener("message", listener);
                const { packageId, commands, selection } = message.data;
                resolve({ packageId, commands, selection });
            };
            window.addEventListener("message", listener, { signal });
            signal.addEventListener("abort", () => reject(signal.reason), { once: true });

            // ask for them
            this.#post({ action: "commands.list" });
        });
    }

    /** Run a command the page's view registers with its input, which closes the palette first. */
    run(entry: RegisteredEntry, input: JsonObject): void {
        this.#post({ action: "command.run", name: entry.name, input });
    }

    /** Close the palette. */
    close(): void {
        this.#post({ action: "palette.close" });
    }

    /** Post a message to the page at its origin. */
    #post(message: PaletteMessage): void {
        this.#window.postMessage(message, this.#origin);
    }
}
