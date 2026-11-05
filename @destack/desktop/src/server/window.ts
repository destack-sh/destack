import type { ObjectClient } from "@destack/object/client";
import type { Session } from "@destack/machine/device";
import { type Condition, Observation } from "@destack/object";
import type { Address } from "@destack/view/declare";
import { Identifier } from "@destack/schema";
import { Link } from "../link/index.ts";
import {
    type DESKTOP_OBJECTS,
    window,
    type WindowBounds,
    type WindowRow,
} from "../object/index.ts";
import type { Shell } from "../shell/index.ts";
import type { Dialog } from "./dialog.ts";

/** How long a window rests after a move before the desktop writes its place: half a second. */
const PLACE_DELAY_MILLISECONDS = 500;

/** The condition a window reports: shown. */
const READY = "Ready";

/** A shown window's ready condition. */
const SHOWN: Condition = { status: "true", reason: "Shown", message: "" };

/** One change a set of window rows asks of the shell and the native device. */
export type WindowStep =
    | {
          /** Launch a row's link and show it in a new window. */
          readonly kind: "show";
          /** The row. */
          readonly row: WindowRow;
      }
    | {
          /** Bring a shown window into focus. */
          readonly kind: "focus";
          /** The row. */
          readonly row: WindowRow;
      }
    | {
          /** Observe a shown window's new generation, which asks nothing more of the shell. */
          readonly kind: "settle";
          /** The row. */
          readonly row: WindowRow;
      }
    | {
          /** Close a row opening a link another window shows, and focus that window. */
          readonly kind: "merge";
          /** The duplicate row's identity. */
          readonly id: string;
          /** The identity of the row showing the link. */
          readonly into: string;
      }
    | {
          /** Close a window whose row is closing or gone, and remove a closing row. */
          readonly kind: "remove";
          /** The row's identity. */
          readonly id: string;
          /** Whether the row is still there, closing, to finalize. */
          readonly isClosing: boolean;
      };

/** Where a row's window stands: launching, asking the person to grant its view, or shown. */
export type WindowState =
    | {
          /** Launching the row's link, before the shell shows it. */
          readonly kind: "launching";
      }
    | {
          /** Asking the person to grant the row's view what it asks. */
          readonly kind: "asking";
          /** The dialog asking. */
          readonly dialog: Dialog;
      }
    | {
          /** Shown by the shell. */
          readonly kind: "shown";
          /** The native window. */
          readonly window: Window;
      };

/** What a window works with: the rows it observes, the shell it shows in and its failures. */
export interface WindowContext {
    /** The desktop's local objects, which store the window rows. */
    readonly objects: ObjectClient<typeof DESKTOP_OBJECTS>;
    /** The native shell. */
    readonly shell: Shell;
    /** Report a failure no event answers, such as a place written late. */
    readonly report: (error: unknown) => void;
}

/** One native window of this computer, showing a window row's link on its launched origin. */
export class Window {
    /** The row's identity. */
    readonly id: string;
    /** Whether it shows a view, or floats the command palette. */
    readonly kind: WindowRow["kind"];
    /** The link the window opened. */
    readonly link: string;
    /** The location the window loaded, on the origin its launch authorized. */
    readonly url: string;
    /** The view or object of the link. */
    readonly address: Address;
    /** The rows, the shell and the failures the window works with. */
    readonly #context: WindowContext;
    /** The focus requests of its row the window answered. */
    #focuses: number;
    /** The place waiting out a drag, absent while none waits. */
    #placing: AbortController | undefined;

    /** Take a window the shell shows for a row, answering the focus requests the row has. */
    constructor(
        fields: Pick<Window, "id" | "kind" | "link" | "url" | "address"> & {
            readonly focuses: number;
        },
        context: WindowContext,
    ) {
        // take what the window shows, and what it works with
        this.id = fields.id;
        this.kind = fields.kind;
        this.link = fields.link;
        this.url = fields.url;
        this.address = fields.address;
        this.#focuses = fields.focuses;
        this.#context = context;
    }

    /** Plan the steps that make the windows match the rows, past the launching or asking ones. */
    static plan(
        states: ReadonlyMap<string, WindowState>,
        rows: readonly WindowRow[],
    ): WindowStep[] {
        // close the windows of rows gone
        const steps: WindowStep[] = [];
        const ids = new Set<string>(rows.map((row) => row.id));
        for (const id of states.keys()) {
            if (!ids.has(id)) {
                steps.push({ kind: "remove", id, isClosing: false });
            }
        }

        // plan each row, merging a row into the earlier one of its kind opening its link
        const opening = new Map<string, string>();
        for (const row of rows.toSorted((left, right) => left.createdAt - right.createdAt)) {
            const state = states.get(row.id);
            const opened = `${row.kind} ${row.link}`;
            const earlier = opening.get(opened);
            // close a closing row's window and remove the row
            if (row.deletedAt !== null) {
                steps.push({ kind: "remove", id: row.id, isClosing: true });
            }
            // merge a row into the earlier one opening its link
            else if (earlier !== undefined) {
                steps.push({ kind: "merge", id: row.id, into: earlier });
            }
            // plan the row this window shows, or shows once it launched or the person answered
            else {
                opening.set(opened, row.id);
                steps.push(...Window.#step(state, row));
            }
        }

        return steps;
    }

    /** Plan the steps a row's window takes from where it stands, none while it stands right. */
    static #step(state: WindowState | undefined, row: WindowRow): WindowStep[] {
        // show a new row
        if (state === undefined) {
            return [{ kind: "show", row }];
        }
        // leave a window launching or asking, which shows once it launched or the person answered
        else if (state.kind !== "shown") {
            return [];
        }
        // focus a shown window asked into focus again
        else if (state.window.#focuses !== row.focuses) {
            return [{ kind: "focus", row }];
        }
        // observe a shown window's change the shell already shows, such as its place
        else if (!Observation.isCurrent(row, READY)) {
            return [{ kind: "settle", row }];
        }
        // leave a shown window as it is
        else {
            return [];
        }
    }

    /** Launch a row's view or object as the person, and show it under a title where it sat. */
    static async open(
        row: WindowRow,
        address: Address,
        title: string,
        session: Pick<Session, "launch">,
        context: WindowContext,
    ): Promise<Window> {
        // launch it on its origin
        const launched = await session.launch(address);
        const profile = Identifier.uuid(launched.target);

        // show it, and observe it shown with its title and profile
        const opened = new Window(
            {
                id: row.id,
                kind: row.kind,
                link: row.link,
                url: launched.url,
                address,
                focuses: row.focuses,
            },
            context,
        );
        await context.shell.send({
            action: "window.open",
            id: row.id,
            kind: row.kind,
            url: launched.url,
            profile,
            title,
            ...(row.bounds === null ? {} : { bounds: row.bounds }),
        });
        await opened.observe(row, { title, profile });

        return opened;
    }

    /** Bring the window into focus, answering its row's focus requests. */
    async focus(row: WindowRow): Promise<void> {
        // count the requests answered, then focus and observe it
        this.#focuses = row.focuses;
        await this.#context.shell.send({ action: "window.focus", id: this.id });
        await this.observe(row);
    }

    /** Stop writing a place waiting out a drag, as the window or its person goes. */
    stop(): void {
        this.#placing?.abort();
        this.#placing = undefined;
    }

    /** Close the window on screen, stopping a place waiting out a drag. */
    async close(): Promise<void> {
        // stop the place, then close it on screen
        this.stop();
        await this.#context.shell.send({ action: "window.close", id: this.id });
    }

    /** Write where the person placed the window once the drag rests, only the latest place. */
    place(bounds: WindowBounds): void {
        // wait out the drag, dropping an earlier place
        this.#placing?.abort();
        const resting = new AbortController();
        this.#placing = resting;

        // write the place once the window rests on the objects' clock
        const { objects, report } = this.#context;
        const write = () => {
            this.#placing = undefined;
            objects
                .mutate(window)
                .place({ id: this.id, bounds })
                .confirmed.catch((error: unknown) => report(error));
        };
        void objects.environment.clock
            .sleep(PLACE_DELAY_MILLISECONDS, resting.signal)
            .then(write, (error: unknown) => {
                // report a failure other than the stop of the wait
                if (!resting.signal.aborted) {
                    report(error);
                }
            });
    }

    /** Read the view a location the window navigated to shows beside its origin, else nothing. */
    beside(href: string): Address | undefined {
        return Link.beside(new URL(this.url), new URL(href));
    }

    /** Observe the window's ready condition at the row's generation, with what it shows. */
    async observe(
        row: WindowRow,
        fields?: { readonly title: string; readonly profile: string },
    ): Promise<void> {
        await this.#context.objects.mutate(window).observe({
            id: row.id,
            observedGeneration: row.generation,
            conditions: { [READY]: SHOWN },
            ...(fields === undefined ? {} : { fields }),
        }).confirmed;
    }
}
