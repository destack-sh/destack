import type { ObjectFile } from "@destack/object";
import type { UploadReason, UploadStatus } from "@destack/object/client";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    useContext,
} from "@destack/view";
import { FileKind } from "./kind.ts";
import type { Refusal } from "./limits.ts";
import { createProgress, type FollowedUpload } from "./upload.ts";

/** How a file is laid out: in a row, or as a card under its picture. */
export type FileItemLayout = "row" | "card";

/** The size of a file: the default, or small as beside a message's text. */
export type FileItemSize = "default" | "sm";

/** A file as it is shown: its name, size in bytes and media type. */
export type ShownFile = Pick<ObjectFile, "name" | "size" | "type">;

/** What a file is shown from, and what a person can do with it. */
export interface FileItemOptions {
    /** The file. */
    readonly file: ShownFile;
    /** The address of its bytes, as `ObjectFileClient` writes one, or the bytes themselves. */
    readonly source?: string | Blob | undefined;
    /** The upload sending it, absent once its object has it. */
    readonly upload?: FollowedUpload | undefined;
    /** Why it could not be sent: a refusal, or the app's own words, its upload's by default. */
    readonly error?: Refusal | string | undefined;
    /** How it is laid out. */
    readonly layout: FileItemLayout;
    /** Its size. */
    readonly size: FileItemSize;
    /** Open it in place, such as in a viewer. */
    readonly onOpen?: (() => void) | undefined;
    /** Send it again after a failure. */
    readonly onRetry?: (() => void) | undefined;
    /** Remove it, or stop sending it. */
    readonly onRemove?: (() => void) | undefined;
}

/** A file as its parts read it and act on it. */
export class FileItemControl {
    /** The address of its bytes, absent before it has one. */
    readonly source: Accessor<string | undefined>;
    /** The options, read as they change. */
    readonly #options: FileItemOptions;
    /** How far its upload is, absent without one. */
    readonly #progress: ReturnType<typeof createProgress>;

    /** Whether its buttons are Tab stops, as those of a group's one focused file are. */
    readonly #isActive: Accessor<boolean>;

    /** Follow a file and its upload. */
    constructor(options: FileItemOptions, isActive: Accessor<boolean> = () => true) {
        // follow its options and Tab stop
        this.#options = options;
        this.#isActive = isActive;
        this.source = createAddress(() => options.source);
        this.#progress = createProgress(() => options.upload);
    }

    /** The file. */
    get file(): ShownFile {
        return this.#options.file;
    }

    /** What it is, as far as its icon tells. */
    get kind(): FileKind {
        return FileKind.of(this.file.name, this.file.type);
    }

    /** How it is laid out. */
    get layout(): FileItemLayout {
        return this.#options.layout;
    }

    /** Its size. */
    get size(): FileItemSize {
        return this.#options.size;
    }

    /** Whether it is there, on its way, paused, cancelled or failed. */
    get status(): UploadStatus {
        return this.#options.error === undefined ? (this.#progress()?.status ?? "done") : "failed";
    }

    /** The share of its bytes that arrived, from 0 to 1. */
    get share(): number {
        return this.#progress()?.share ?? 1;
    }

    /** Why it could not be sent, absent for a failure its upload tells. */
    get error(): Refusal | string | undefined {
        return this.#options.error;
    }

    /** Why its upload failed, absent for one that did not. */
    get reason(): UploadReason | undefined {
        return this.#progress()?.reason;
    }

    /** Whether it did not arrive, as it failed or a person stopped it. */
    get isFailed(): boolean {
        return this.status === "failed" || this.status === "cancelled";
    }

    /** Whether it is on its way, paused or not. */
    get isWaiting(): boolean {
        return this.status === "sending" || this.status === "paused";
    }

    /** Whether a person can open it. */
    get isOpenable(): boolean {
        return (
            !this.isWaiting && (this.source() !== undefined || this.#options.onOpen !== undefined)
        );
    }

    /** Whether a person can send it again. */
    get isRetryable(): boolean {
        return this.isFailed && this.#options.onRetry !== undefined;
    }

    /** Whether a person can remove it. */
    get isRemovable(): boolean {
        return this.#options.onRemove !== undefined;
    }

    /** Whether its buttons are Tab stops, as those of a group's one focused file are. */
    get isActive(): boolean {
        return this.#isActive();
    }

    /** Open it in place, leaving a click that asks for a new tab or window to the browser. */
    open(event: MouseEvent): void {
        const isPlain = !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
        const { onOpen } = this.#options;
        if (onOpen !== undefined && isPlain) {
            event.preventDefault();
            onOpen();
        }
    }

    /** Send it again after a failure. */
    retry(): void {
        this.#options.onRetry?.();
    }

    /** Remove it, or stop sending it. */
    remove(): void {
        this.#options.onRemove?.();
    }
}

/** The nearest file, null outside one. */
export const FileItemContext = createContext<FileItemControl | null>(null);

/** Read the nearest file, refusing a part outside one. */
export function useFileItem(): FileItemControl {
    const item = useContext(FileItemContext);
    if (item === null) {
        throw new TypeError("a part of a file needs a file around it");
    }

    return item;
}

/** Address bytes the browser has for as long as they are shown, and pass on another address. */
function createAddress(source: () => string | Blob | undefined): () => string | undefined {
    // address the bytes until others come or the file goes
    const [made, setMade] = createSignal<string | undefined>(undefined, { ownedWrite: true });
    const bytes = (): Blob | undefined => {
        const given = source();

        return typeof given === "object" ? given : undefined;
    };
    createEffect(bytes, (given) => {
        const address = given === undefined ? undefined : URL.createObjectURL(given);
        setMade(address);

        return () => {
            if (address !== undefined) {
                URL.revokeObjectURL(address);
            }
        };
    });

    return () => {
        const given = source();

        return typeof given === "string" ? given : made();
    };
}
