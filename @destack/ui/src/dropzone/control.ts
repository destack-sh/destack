import {
    type Accessor,
    createContext,
    createSignal,
    flush,
    type Setter,
    useContext,
} from "@destack/view";
import { isAccepted } from "@destack/object";
import { present } from "@destack/schema";
import { type FileLimits, type Refusal, refusalOf } from "../file-item/index.ts";

/** Whether the files dragged over a zone fit, absent without a drag. */
export type DropzoneDrag = "accept" | "reject";

/** What a zone takes, and what becomes of it. */
export interface DropzoneOptions {
    /** Take the files that fit, reading each one's path in the folder it came with. */
    readonly onFiles: (files: readonly File[], pathOf: (file: File) => string) => void;
    /** Hear of the files that do not, such as to tell them elsewhere. */
    readonly onRefuse?: ((refusals: readonly Refusal[]) => void) | undefined;
    /** Hear of its form resetting, to forget the files. */
    readonly onReset?: (() => void) | undefined;
    /** Say in the app's own words why a file does not fit, absent for one that does. */
    readonly validate?: ((file: File) => string | undefined) | undefined;
    /** The files there are, which its form submits, those it took by default. */
    readonly files?: readonly File[] | undefined;
    /** The media types, sizes and number of files it takes. */
    readonly limits: FileLimits;
    /** Whether it takes several files. */
    readonly multiple: boolean;
    /** Whether it takes no file now. */
    readonly disabled: boolean;
    /** The name its form submits the files under, submitting none without one. */
    readonly name?: string | undefined;
}

/** A file handed to a zone. */
export interface DropzoneFile {
    /** The file. */
    readonly file: File;
    /** Its path in the folder it came with, its name without one. */
    readonly path: string;
    /** Whether it stands for a folder, whose bytes the browser does not read. */
    readonly isFolder: boolean;
}

/** A zone as its parts read it and hand it files. */
export class DropzoneControl {
    /** The options, read as they change. */
    readonly #options: DropzoneOptions;
    /** Whether a drag is over it. */
    readonly #isDragging: Accessor<boolean>;
    /** The media types of the files dragged over it. */
    readonly #offered: Accessor<readonly string[]>;
    /** The files it took, while its owner passes none. */
    readonly #taken: Accessor<readonly File[]>;
    /** Replace the files it took. */
    readonly #setTaken: Setter<readonly File[]>;
    /** The files it refused last, none while the last all fit. */
    readonly #refusals: Accessor<readonly Refusal[]>;
    /** Replace the files it refused. */
    readonly #setRefusals: Setter<readonly Refusal[]>;
    /** The browser's own picker, once it is there. */
    #input: HTMLInputElement | undefined;

    /** Follow what a zone takes and the drags over it. */
    constructor(
        options: DropzoneOptions,
        isDragging: Accessor<boolean>,
        offered: Accessor<readonly string[]>,
    ) {
        // take and refuse nothing yet
        this.#options = options;
        this.#isDragging = isDragging;
        this.#offered = offered;
        [this.#taken, this.#setTaken] = createSignal<readonly File[]>([], { ownedWrite: true });
        [this.#refusals, this.#setRefusals] = createSignal<readonly Refusal[]>([], {
            ownedWrite: true,
        });
    }

    /** The media types, sizes and number of files it takes. */
    get limits(): FileLimits {
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

    /** Whether the files dragged over it fit as far as the browser tells, absent without a drag. */
    get drag(): DropzoneDrag | undefined {
        if (!this.isDragging) {
            return undefined;
        }

        // read their number and the types the browser shows
        const offered = this.#offered();
        const isFitting =
            offered.length <= this.#room() &&
            offered.every((type) => type === "" || isAccepted(type, this.limits));

        return isFitting ? "accept" : "reject";
    }

    /** The files there are: its owner's, else those it took. */
    get files(): readonly File[] {
        return this.#options.files ?? this.#taken();
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
        // take them with their paths in a chosen folder
        this.take([...(this.#input?.files ?? [])].map((file) => fileOf(file)));

        // leave the picker with the files there are, as the form submits those
        flush();
        this.submit(this.files);
    }

    /** Give the picker the files its form submits, none without a name. */
    submit(files: readonly File[]): void {
        if (this.#input === undefined) {
            return;
        } else if (this.#options.name === undefined) {
            this.#input.value = "";
        } else {
            const transfer = new DataTransfer();
            for (const file of files) {
                transfer.items.add(file);
            }
            this.#input.files = transfer.files;
        }
    }

    /** Forget the files and refusals, as its form resets. */
    reset(): void {
        this.#setTaken([]);
        this.#setRefusals([]);
        this.#options.onReset?.();
    }

    /** Take the files that fit, and refuse the others. */
    take(given: readonly DropzoneFile[]): void {
        // leave files while it takes none
        if (this.isDisabled || given.length === 0) {
            return;
        }

        // refuse several where it takes one, else each that does not fit or is too many
        const { limits } = this;
        const [first] = given;
        const isSeveral = !this.isMultiple && given.length > 1;
        const refusals =
            isSeveral && first !== undefined
                ? [{ file: first.file, reason: "count" as const, limits: { ...limits, count: 1 } }]
                : given
                      .map((each) => this.#refuse(each))
                      .filter((refusal) => refusal !== undefined);
        const refused = new Set(refusals.map(({ file }) => file));
        const fitting = isSeveral ? [] : given.filter(({ file }) => !refused.has(file));
        const taken = fitting.slice(0, this.#room());
        const [over] = fitting.slice(taken.length);
        const told =
            over === undefined
                ? refusals
                : [...refusals, { file: over.file, reason: "count" as const, limits }];

        // hand on the files it takes with their paths, and tell those it refuses
        this.#setRefusals(told);
        if (taken.length > 0) {
            const files = taken.map(({ file }) => file);
            const paths = new Map(taken.map(({ file, path }) => [file, path]));
            this.#setTaken((current) => (this.isMultiple ? [...current, ...files] : files));
            this.#options.onFiles(files, (file) => present(paths.get(file), "the file's path"));
        }
        if (told.length > 0) {
            this.#options.onRefuse?.(told);
        }
    }

    /** Report why a file does not fit, by the limits and then the app. */
    #refuse({ file, isFolder }: DropzoneFile): Refusal | undefined {
        // refuse a folder, whose bytes the browser does not read
        const { limits } = this;
        if (isFolder) {
            return { file, reason: "folder", limits };
        }

        // ask the app about a file within the limits
        const refusal = refusalOf(file, limits);
        const message = refusal === undefined ? this.#options.validate?.(file) : undefined;

        return message === undefined ? refusal : { file, reason: "custom", limits, message };
    }

    /** Count the files it still takes, one that replaces another where it takes one. */
    #room(): number {
        // take one that replaces another
        const { count } = this.limits;
        if (!this.isMultiple) {
            return 1;
        }

        return count === undefined
            ? Number.POSITIVE_INFINITY
            : Math.max(count - this.files.length, 0);
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

/** Read a chosen or pasted file with its path. */
export function fileOf(file: File): DropzoneFile {
    const path = file.webkitRelativePath;

    return { file, path: path === "" ? file.name : path, isFolder: false };
}
