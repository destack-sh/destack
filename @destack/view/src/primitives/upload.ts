import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal, onCleanup } from "solid-js";
import { TRANSPARENT } from "./utils.ts";

/** A chosen file, with an object URL that shows it. */
export type UploadFile = {
    /** An object URL of the file. */
    readonly source: string;
    /** The file's name. */
    readonly name: string;
    /** The file's size in bytes. */
    readonly size: number;
    /** The file. */
    readonly file: File;
};

/** Which files a picker offers. */
export type FilePickerOptions = {
    /** The accepted file types, as the `accept` attribute lists them. */
    readonly accept?: string;
    /** Whether several files may be chosen. */
    readonly multiple?: boolean;
};

/** Handle the chosen files. */
export type UserCallback = (files: UploadFile[]) => void | Promise<void>;

/** Handle a drag moving over a drop zone, whose files the browser reveals only on drop. */
export type DragEventCallback = (event: DragEvent) => void | Promise<void>;

/** A file picker's files, the state of its callback, and how to choose and drop files. */
export interface FilePicker {
    /** The chosen files. */
    readonly files: Accessor<UploadFile[]>;
    /** The error of the last callback, null while none failed. */
    readonly error: Accessor<unknown>;
    /** Whether the callback runs. */
    readonly isLoading: Accessor<boolean>;
    /** Open the picker, then run the callback with the chosen files. */
    readonly selectFiles: (callback?: UserCallback) => void;
    /** Drop a file by name. */
    readonly removeFile: (fileName: string) => void;
    /** Drop every file. */
    readonly clearFiles: () => void;
}

/** The `fileUploader` ref factory's callback, file setter and error handler. */
export type FileUploaderDirective = {
    /** Handle the chosen files. */
    readonly userCallback: UserCallback;
    /** Hold the chosen files, whose object URLs the caller revokes. */
    readonly setFiles: (files: UploadFile[]) => void;
    /** Handle a failed callback, which rejects unhandled without one. */
    readonly onError?: (error: unknown) => void;
};

/** A drop zone's ref, its files, the state of its callback and the drag, and how to drop files. */
export interface Dropzone<Target extends HTMLElement = HTMLElement> {
    /** Make an element the drop zone. */
    readonly ref: (element: Target) => void;
    /** The dropped files. */
    readonly files: Accessor<UploadFile[]>;
    /** The error of the last drop callback, null while none failed. */
    readonly error: Accessor<unknown>;
    /** Whether the drop callback runs. */
    readonly isLoading: Accessor<boolean>;
    /** Whether a drag is over the zone. */
    readonly isDragging: Accessor<boolean>;
    /** Drop a file by name. */
    readonly removeFile: (fileName: string) => void;
    /** Drop every file. */
    readonly clearFiles: () => void;
}

/** What a drop zone does as a drag enters, moves, leaves and drops. */
export interface DropzoneOptions {
    /** Handle the dropped files. */
    readonly onDrop?: UserCallback;
    /** Handle a drag entering the zone. */
    readonly onDragEnter?: DragEventCallback;
    /** Handle a drag leaving the zone. */
    readonly onDragLeave?: DragEventCallback;
    /** Handle a drag moving over the zone. */
    readonly onDragOver?: DragEventCallback;
}

/** Open the device's file picker and follow the chosen files. */
export function createFilePicker(options: FilePickerOptions = {}): FilePicker {
    // choose nothing on the server
    if (isServer) {
        return {
            files: () => [],
            error: () => null,
            isLoading: () => false,
            selectFiles: () => {},
            removeFile: () => {},
            clearFiles: () => {},
        };
    }
    const { files, setFiles, removeFile, clearFiles } = createFileList();
    const [error, setError] = createSignal<unknown>(null, { ownedWrite: true });
    const [isLoading, setIsLoading] = createSignal(false, { ownedWrite: true });
    let callback: UserCallback = ignoreFiles;

    // open a fresh file input, then hold the chosen files and run the callback, recording its failure
    const selectFiles = (next?: UserCallback): void => {
        // make a fresh input of the chosen kinds
        callback = next ?? callback;
        const input = document.createElement("input");
        input.type = "file";
        input.accept = options.accept ?? "";
        input.multiple = options.multiple ?? false;
        input.addEventListener(
            "change",
            (event) => {
                // keep the event from the page
                event.preventDefault();
                event.stopPropagation();
                const chosen = readUploadFiles(input.files);
                input.remove();
                setFiles(chosen);
                void run(callback, chosen, setError, setIsLoading);
            },
            { once: true },
        );
        input.click();
    };

    return { files, error, isLoading, selectFiles, removeFile, clearFiles };
}

/** Receive files dragged onto an element, following the drag. */
export function createDropzone<Target extends HTMLElement = HTMLElement>(
    options: DropzoneOptions = {},
): Dropzone<Target> {
    // receive nothing on the server
    if (isServer) {
        return {
            ref: () => {},
            files: () => [],
            error: () => null,
            isLoading: () => false,
            isDragging: () => false,
            removeFile: () => {},
            clearFiles: () => {},
        };
    }
    const { files, setFiles, removeFile, clearFiles } = createFileList();
    const [error, setError] = createSignal<unknown>(null, { ownedWrite: true });
    const [isLoading, setIsLoading] = createSignal(false, { ownedWrite: true });
    const [isDragging, setIsDragging] = createSignal(false, { ownedWrite: true });
    const [zone, setZone] = createSignal<Target | undefined>(undefined, { ownedWrite: true });

    // allow drops over the zone
    const over = (event: DragEvent): void => {
        // leave a drag of text or of the page's own elements
        if (!hasFiles(event)) {
            return;
        }
        event.preventDefault();
        void options.onDragOver?.(event);
    };

    // follow drags over the zone, counting nested enters, and take the files on drop
    createEffect(
        zone,
        (element) => {
            if (element === undefined) {
                return undefined;
            }
            let depth = 0;
            const enter = (event: DragEvent): void => {
                // leave a drag without files
                if (!hasFiles(event)) {
                    return;
                }
                depth++;
                if (depth === 1) {
                    setIsDragging(true);
                    void options.onDragEnter?.(event);
                }
            };
            const leave = (event: DragEvent): void => {
                // leave a drag without files
                if (!hasFiles(event)) {
                    return;
                }
                depth = Math.max(depth - 1, 0);
                if (depth === 0) {
                    setIsDragging(false);
                    void options.onDragLeave?.(event);
                }
            };
            const drop = (event: DragEvent): void => {
                // leave a drag without files
                if (!hasFiles(event)) {
                    return;
                }

                // stop the drag
                event.preventDefault();
                depth = 0;
                setIsDragging(false);
                const dropped = readUploadFiles(event.dataTransfer?.files ?? null);
                setFiles(dropped);
                void run(options.onDrop, dropped, setError, setIsLoading);
            };
            element.addEventListener("dragenter", enter);
            element.addEventListener("dragleave", leave);
            element.addEventListener("dragover", over);
            element.addEventListener("drop", drop);

            return () => {
                // stop listening to the zone
                element.removeEventListener("dragenter", enter);
                element.removeEventListener("dragleave", leave);
                element.removeEventListener("dragover", over);
                element.removeEventListener("drop", drop);
            };
        },
        TRANSPARENT,
    );

    return {
        ref: (target) => setZone(() => target),
        files,
        error,
        isLoading,
        isDragging,
        removeFile,
        clearFiles,
    };
}

/** Make a drop zone that is its own ref. */
export function dropzone<Target extends HTMLElement = HTMLElement>(
    options?: DropzoneOptions,
): Dropzone<Target>["ref"] & Omit<Dropzone<Target>, "ref"> {
    const { ref, ...state } = createDropzone<Target>(options);

    return Object.assign(ref, state);
}

/** Hand the files chosen in a file input a ref receives to a callback. */
export function fileUploader(options: FileUploaderDirective): (element: HTMLInputElement) => void {
    // choose nothing on the server
    if (isServer) {
        return () => {};
    }

    // hold the chosen files, run the callback, and clear the input for the next choice
    let target: HTMLInputElement | undefined;
    const change = async (): Promise<void> => {
        if (target === undefined) {
            return;
        }
        const chosen = readUploadFiles(target.files);
        options.setFiles(chosen);
        try {
            await options.userCallback(chosen);
        } catch (error) {
            if (options.onError === undefined) {
                throw error;
            }
            options.onError(error);
        } finally {
            target.value = "";
        }
    };
    const listener = (): void => {
        void change();
    };
    onCleanup(() => target?.removeEventListener("change", listener));

    return (element) => {
        target = element;
        element.addEventListener("change", listener);
    };
}

/** Report whether a drag has files, which the browser tells before they are dropped. */
function hasFiles(event: DragEvent): boolean {
    return event.dataTransfer?.types.includes("Files") === true;
}

/** Hold a list of chosen files, revoking each file's object URL once it is dropped. */
function createFileList(): {
    files: Accessor<UploadFile[]>;
    setFiles: (files: UploadFile[]) => void;
    removeFile: (fileName: string) => void;
    clearFiles: () => void;
} {
    // keep the held files in plain state, which revocation reads right away, mirrored to a signal
    const [files, setStored] = createSignal<UploadFile[]>([], { ownedWrite: true });
    let held: UploadFile[] = [];
    const hold = (next: UploadFile[]): void => {
        held = next;
        setStored(next);
    };
    onCleanup(() => revoke(held));

    return {
        files,
        setFiles: (next) => {
            revoke(held);
            hold(next);
        },
        removeFile: (fileName) => {
            revoke(held.filter((file) => file.name === fileName));
            hold(held.filter((file) => file.name !== fileName));
        },
        clearFiles: () => {
            revoke(held);
            hold([]);
        },
    };
}

/** Wrap a file list's files, each with an object URL. */
function readUploadFiles(list: FileList | null): UploadFile[] {
    return [...(list ?? [])].map((file) => ({
        source: URL.createObjectURL(file),
        name: file.name,
        size: file.size,
        file,
    }));
}

/** Run a callback with files, recording whether it runs and its failure. */
async function run(
    callback: UserCallback | undefined,
    files: UploadFile[],
    setError: (error: unknown) => void,
    setIsLoading: (isLoading: boolean) => void,
): Promise<void> {
    setError(null);
    setIsLoading(true);
    try {
        await callback?.(files);
    } catch (error) {
        setError(error);
    } finally {
        setIsLoading(false);
    }
}

/** Take chosen files without handling them. */
function ignoreFiles(): void {}

/** Revoke the object URLs of dropped files. */
function revoke(dropped: readonly UploadFile[]): void {
    for (const file of dropped) {
        URL.revokeObjectURL(file.source);
    }
}
