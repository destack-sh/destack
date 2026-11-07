import { Failure } from "@destack/error";
import type { Identifier, JsonObject } from "@destack/schema";
import { type Accessor, createSignal, type Setter } from "@destack/view";
import { schemaFields } from "@destack/ui/schema-form";
import { type CommandEntry, PaletteFocus, type RegisteredEntry } from "@destack/view/palette";
import type { PaletteListing } from "./listing.ts";

/** A command the palette runs: one an installation declares, or one the page under it registers. */
export type RunEntry = CommandEntry | RegisteredEntry;

/** Where running a command stands, from the search to its outcome. */
export type PaletteStep =
    | { readonly kind: "search" }
    | { readonly kind: "confirm"; readonly command: RunEntry }
    | { readonly kind: "object"; readonly command: CommandEntry }
    | {
          readonly kind: "input";
          readonly command: RunEntry;
          readonly parameters: JsonObject;
          readonly targets: readonly string[];
      }
    | {
          readonly kind: "approve";
          readonly command: CommandEntry;
          readonly input: JsonObject;
          readonly targets: readonly string[];
      }
    | { readonly kind: "running"; readonly command: RunEntry }
    | { readonly kind: "done"; readonly command: RunEntry }
    | { readonly kind: "failed"; readonly command: RunEntry; readonly message: string };

/** The run of commands from a palette: its step and the space it narrows to. */
export class PaletteRun {
    /** The step the run stands at. */
    readonly step: Accessor<PaletteStep>;
    /** The space the search narrows to, absent for every space. */
    readonly scope: Accessor<Identifier<"space"> | undefined>;
    /** Move the run to another step. */
    readonly #setStep: Setter<PaletteStep>;
    /** Narrow the search to a space, or widen it. */
    readonly #setScope: Setter<Identifier<"space"> | undefined>;
    /** The entries of the person's spaces. */
    readonly #listing: PaletteListing;
    /** What the window the palette opens over shows. */
    readonly #focus: PaletteFocus;
    /** Close the palette. */
    readonly #close: () => void;
    /** Run a command of the page under the palette, which closes it. */
    readonly #runInPage: (entry: RegisteredEntry, input: JsonObject) => void;

    /** Start at the search over every space. */
    constructor(
        listing: PaletteListing,
        focus: PaletteFocus,
        page: {
            readonly close: () => void;
            readonly run: (entry: RegisteredEntry, input: JsonObject) => void;
        },
    ) {
        // start searching every space
        const [step, setStep] = createSignal<PaletteStep>({ kind: "search" });
        const [scope, setScope] = createSignal<Identifier<"space"> | undefined>(undefined);
        this.step = step;
        this.scope = scope;
        this.#setStep = setStep;
        this.#setScope = setScope;
        this.#listing = listing;
        this.#focus = focus;
        this.#close = page.close;
        this.#runInPage = page.run;
    }

    /** Offer a launched command, which runs once the person confirms it. */
    offer(command: RunEntry): void {
        this.#setStep({ kind: "confirm", command });
    }

    /** Narrow the search to a space's entries. */
    narrow(space: Identifier<"space">): void {
        this.#setScope(space);
    }

    /** Widen a narrowed search on Escape, else close the palette. */
    escape(): void {
        if (this.step().kind === "search" && this.scope() !== undefined) {
            this.#setScope(undefined);
        } else {
            this.#close();
        }
    }

    /** Run a command: on the focused object or the ones the person picks, else on its scope. */
    run(entry: RunEntry): void {
        // ask for a page command's input
        if (entry.kind === "registered") {
            this.#ask(entry, this.#listing.registered(entry).input ?? {}, []);

            return;
        }

        // read what the declared command acts on
        let target: "object" | "scope";
        try {
            target = this.#listing.catalog(entry).command(entry).target;
        } catch (error) {
            this.#fail(entry, error);

            return;
        }

        // act on the scope, on the focused objects of the command's type, or on one the person picks
        const focused = PaletteFocus.ids(this.#focus, entry);
        if (target === "scope") {
            this.pick(entry, []);
        } else if (focused.length > 0) {
            this.pick(entry, focused);
        } else {
            this.#setStep({ kind: "object", command: entry });
        }
    }

    /** Go on with the picked targets: ask for the command's input, or call it right away. */
    pick(entry: CommandEntry, targets: readonly string[]): void {
        try {
            this.#ask(entry, this.#listing.catalog(entry).command(entry).parameters, targets);
        } catch (error) {
            this.#fail(entry, error);
        }
    }

    /** Run a command with its input, once the person approves a command removing its targets. */
    submit(entry: RunEntry, input: JsonObject, targets: readonly string[]): void {
        // run a page command in the page, which closes the palette
        if (entry.kind === "registered") {
            this.#runInPage(entry, input);
        }
        // ask before removing objects
        else if (this.#listing.catalog(entry).command(entry).isDestructive) {
            this.#setStep({ kind: "approve", command: entry, input, targets });
        }
        // call it
        else {
            this.call(entry, input, targets);
        }
    }

    /** Call a declared command once per target, or once on its scope, then show the outcome. */
    call(entry: CommandEntry, input: JsonObject, targets: readonly string[]): void {
        // call it on each target, or on its scope without any
        this.#setStep({ kind: "running", command: entry });
        const catalog = this.#listing.catalog(entry);
        const calls =
            targets.length === 0
                ? [catalog.call(entry, input, undefined)]
                : targets.map((target) => catalog.call(entry, input, target));
        Promise.all(calls).then(
            () => this.#setStep({ kind: "done", command: entry }),
            (error: unknown) => this.#fail(entry, error),
        );
    }

    /** Ask for a command's input when it takes any, else submit it. */
    #ask(entry: RunEntry, parameters: JsonObject, targets: readonly string[]): void {
        if (schemaFields(parameters).length > 0) {
            this.#setStep({ kind: "input", command: entry, parameters, targets });
        } else {
            this.submit(entry, {}, targets);
        }
    }

    /** Show why a command failed. */
    #fail(entry: RunEntry, error: unknown): void {
        this.#setStep({ kind: "failed", command: entry, message: Failure.of(error).message });
    }
}

/** Read a step of one kind, absent for any other. */
export function stepOf<Kind extends PaletteStep["kind"]>(
    step: PaletteStep,
    kind: Kind,
): Extract<PaletteStep, { readonly kind: Kind }> | undefined {
    return isKind(step, kind) ? step : undefined;
}

/** Report whether a step is of one kind. */
function isKind<Kind extends PaletteStep["kind"]>(
    step: PaletteStep,
    kind: Kind,
): step is Extract<PaletteStep, { readonly kind: Kind }> {
    return step.kind === kind;
}
