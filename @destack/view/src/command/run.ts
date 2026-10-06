import type { Identifier, JsonObject } from "@destack/schema";
import type { Accessor, Setter } from "solid-js";
import { fieldsOf } from "../form/schema.tsx";
import { createSignal } from "../solid/reactive.ts";
import type { CommandEntry, CommandShape, PaletteClient, PaletteFocus } from "../palette/entry.ts";

/** Where running a command stands: searching, confirming a launched command, picking its object, filling its input, calling it, or its outcome. */
export type PaletteStep =
    | { readonly kind: "search" }
    | { readonly kind: "confirm"; readonly command: CommandEntry }
    | { readonly kind: "object"; readonly command: CommandEntry; readonly shape: CommandShape }
    | {
          readonly kind: "input";
          readonly command: CommandEntry;
          readonly shape: CommandShape;
          readonly object: string | undefined;
      }
    | { readonly kind: "running"; readonly command: CommandEntry }
    | { readonly kind: "done"; readonly command: CommandEntry }
    | { readonly kind: "failed"; readonly command: CommandEntry; readonly message: string };

/** The run of commands from a palette: the step it stands at, the space it narrows to, and where each choice leads. */
export class PaletteRun {
    /** The step the run stands at. */
    readonly step: Accessor<PaletteStep>;
    /** The space the search narrows to, absent for every space. */
    readonly scope: Accessor<Identifier<"space"> | undefined>;
    /** Move the run to another step. */
    readonly #setStep: Setter<PaletteStep>;
    /** Narrow the search to a space, or widen it. */
    readonly #setScope: Setter<Identifier<"space"> | undefined>;
    /** What the palette reaches as the person. */
    readonly #client: PaletteClient;
    /** What the window the palette opens over shows. */
    readonly #focus: PaletteFocus;

    /** Start at the search over every space. */
    constructor(client: PaletteClient, focus: PaletteFocus) {
        // start searching every space
        const [step, setStep] = createSignal<PaletteStep>({ kind: "search" });
        const [scope, setScope] = createSignal<Identifier<"space"> | undefined>(undefined);
        this.step = step;
        this.scope = scope;
        this.#setStep = setStep;
        this.#setScope = setScope;
        this.#client = client;
        this.#focus = focus;
    }

    /** Offer a launched command for the person to confirm, as only their own choice inside the palette calls it. */
    offer(command: CommandEntry): void {
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
            this.#client.close();
        }
    }

    /** Run a command on the focused window's object of its type, or on one the person picks. */
    run(command: CommandEntry): void {
        this.#client.shape(command).then(
            (shape) => {
                const object = focusedObject(command, this.#focus);
                if (shape.isTargeted && object === undefined) {
                    this.#setStep({ kind: "object", command, shape });
                } else {
                    this.proceed(command, shape, shape.isTargeted ? object : undefined);
                }
            },
            (error: unknown) => this.#fail(command, error),
        );
    }

    /** Fill the input a command takes beside its object, or call it right away. */
    proceed(command: CommandEntry, shape: CommandShape, object: string | undefined): void {
        if (fieldsOf(shape.input).length > 0) {
            this.#setStep({ kind: "input", command, shape, object });
        } else {
            this.call(command, {}, object);
        }
    }

    /** Call a command with its input, then show the outcome. */
    call(command: CommandEntry, input: JsonObject, object: string | undefined): void {
        this.#setStep({ kind: "running", command });
        this.#client.call(command, object === undefined ? input : { ...input, id: object }).then(
            () => this.#setStep({ kind: "done", command }),
            (error: unknown) => this.#fail(command, error),
        );
    }

    /** Show why a command failed. */
    #fail(command: CommandEntry, error: unknown): void {
        const message = error instanceof Error ? error.message : String(error);
        this.#setStep({ kind: "failed", command, message });
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

/** Read the identifier of the focused window's object when a command acts on its type. */
function focusedObject(command: CommandEntry, focus: PaletteFocus): string | undefined {
    const { object } = focus;

    return object?.packageId === command.typePackageId && object.type === command.type
        ? object.id
        : undefined;
}
