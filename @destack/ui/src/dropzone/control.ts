import type { ObjectFileLimits } from "@destack/object";
import { type Accessor, createContext, createSignal, type Setter, useContext } from "@destack/view";
import { type Refusal, refusalOf } from "../file-item/index.ts";

/** What a zone takes, and what becomes of it. */
export interface DropzoneOptions {
    /** Take the files that fit. */
    readonly onFiles: (files: readonly File[]) => void;
    /** Hear of the files that do not, such as to tell them elsewhere. */
    readonly onRefuse?: ((refusals: readonly Refusal[]) => void) | undefined;
    /** The media types and sizes it takes. */
    readonly limits: ObjectFileLimits;
    /** Whether it takes several files. */
    readonly multiple: boolean;
    /** Whether it takes no file now. */
    readonly disabled: boolean;
}

/** A zone as its parts read it and hand it files. */
export class DropzoneControl {
    /** The options, read as they change. */
    readonly #options: DropzoneOptions;
    /** Whether a drag is over it. */
    readonly #isDragging: Accessor<boolean>;
    /** The files it refused last, none while the last all fit. */
    readonly #refusals: Accessor<readonly Refusal[]>;
    /** Replace the files it refused. */
    readonly #setRefusals: Setter<readonly Refusal[]>;
    /** The browser's own picker, once it is there. */
    #input: HTMLInputElement | undefined;

    /** Follow what a zone takes and the drags over it. */
    constructor(options: DropzoneOptions, isDragging: Accessor<boolean>) {
        // refuse nothing yet
        this.#options = options;
        this.#isDragging = isDragging;
        [this.#refusals, this.#setRefusals] = createSignal<readonly Refusal[]>([], {
            ownedWrite: true,
        });
    }

    /** The media types and sizes it takes. */
    get limits(): ObjectFileLimits {
        return this.#options.limits;
    }

    /** Whether it takes several files. */
    get isMultiple(): boolean {
        return this.#options.multiple;
    }

    /** Whether it takes no file now. */
    get isDisabled(): boolean {
        return this.#options.disabled;
    }

    /** Whether files it may take are dragged over it. */
    get isDragging(): boolean {
        return this.#isDragging() && !this.isDisabled;
    }

    /** The files it refused last, none while the last all fit. */
    get refusals(): readonly Refusal[] {
        return this.#refusals();
    }

    /** Take the browser's own picker. */
    attach(input: HTMLInputElement): void {
        this.#input = input;
    }

    /** Open the picker. */
    choose(): void {
        this.#input?.click();
    }

    /** Take the files chosen in the picker, which may choose the same ones again. */
    read(): void {
        this.take([...(this.#input?.files ?? [])]);
        if (this.#input !== undefined) {
            this.#input.value = "";
        }
    }

    /** Take the files that fit, and refuse the others. */
    take(given: readonly File[]): void {
        // leave files while it takes none
        if (this.isDisabled || given.length === 0) {
            return;
        }

        // refuse several where it takes one, else each outside the limits
        const { limits } = this;
        const isTooMany = !this.isMultiple && given.length > 1;
        const [first] = given;
        const refusals =
            isTooMany && first !== undefined
                ? [{ file: first, reason: "count" as const, limits }]
                : given.map((file) => refusalOf(file, limits)).filter((each) => each !== undefined);
        const refused = new Set(refusals.map(({ file }) => file));
        const fitting = isTooMany ? [] : given.filter((file) => !refused.has(file));
        this.#setRefusals(refusals);

        // hand both on
        if (fitting.length > 0) {
            this.#options.onFiles(fitting);
        }
        if (refusals.length > 0) {
            this.#options.onRefuse?.(refusals);
        }
    }
}

/** The nearest zone, null outside one. */
export const DropzoneContext = createContext<DropzoneControl | null>(null);

/** Read the nearest zone, refusing a part outside one. */
export function useDropzone(): DropzoneControl {
    const zone = useContext(DropzoneContext);
    if (zone === null) {
        throw new TypeError("a part of a zone needs a zone around it");
    }

    return zone;
}
