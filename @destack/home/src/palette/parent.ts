import { type ActionEntry, PaletteMessage } from "@destack/view/palette";

/** The page the palette is framed over, which handles actions and closes the palette. */
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

    /** List the actions the page runs now and its selection, once the page answers. */
    list(signal: AbortSignal): Promise<Extract<PaletteMessage, { action: "actions.listed" }>> {
        return new Promise((resolve, reject) => {
            // take the first listing the page posts from its origin
            const listener = (event: MessageEvent) => {
                // skip any message but the page's listing
                const message = PaletteMessage.safeParse(event.data);
                if (
                    event.source !== this.#window ||
                    event.origin !== this.#origin ||
                    !message.success ||
                    message.data.action !== "actions.listed"
                ) {
                    return;
                }
                // stop listening and answer the page's listing
                window.removeEventListener("message", listener);
                resolve(message.data);
            };
            window.addEventListener("message", listener, { signal });
            signal.addEventListener("abort", () => reject(signal.reason), { once: true });

            // ask for them
            this.#post({ action: "actions.list" });
        });
    }

    /** Run an action the page handles, which closes the palette first. */
    run(entry: ActionEntry): void {
        this.#post({
            action: "action.run",
            command: { packageId: entry.packageId, name: entry.name },
        });
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
