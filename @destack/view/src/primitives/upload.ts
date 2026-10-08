import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    createStore,
    onCleanup,
} from "solid-js";
import { TRANSPARENT } from "./utils.ts";

/** A chosen file, with an object URL showing it, which the primitive revokes once the file is dropped. */
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

/** Where an upload stands. */
export type UploadStatus = "idle" | "uploading" | "success" | "error" | "aborted";

/** How much of an upload has been sent. */
export type UploadProgress = {
    /** The bytes sent. */
    readonly loaded: number;
    /** The bytes to send. */
    readonly total: number;
    /** The share sent, in whole percent. */
    readonly percentage: number;
};

/** Send one file, reporting progress, rejecting with an `AbortError` when the signal aborts. */
export type SendFunction = (
    file: UploadFile,
    onProgress: (progress: UploadProgress) => void,
    signal: AbortSignal,
) => Promise<unknown>;

/** One file of an upload, with its progress, status, error and response. */
export type FileUploadEntry = {
    /** The file. */
    file: UploadFile;
    /** How much of it has been sent. */
    progress: UploadProgress;
    /** Where its upload stands. */
    status: UploadStatus;
    /** Why its upload failed, null unless it did. */
    error: unknown;
    /** The server's response, null until it succeeds. */
    response: unknown;
};

/** An uploader's upload action, its files, their combined progress and status, and how to abort and drop them. */
export interface FileUploader {
    /** Send each file in its own request, resolving once all settle. */
    readonly upload: (files: UploadFile[]) => Promise<unknown[]>;
    /** The files of the latest upload, each updating on its own. */
    readonly files: readonly FileUploadEntry[];
    /** The progress of all files together. */
    readonly progress: Accessor<UploadProgress>;
    /** The status of all files together: uploading, else error, else aborted, else success, else idle. */
    readonly status: Accessor<UploadStatus>;
    /** Abort every upload in flight. */
    readonly abort: () => void;
    /** Drop a file by name, aborting every upload in flight. */
    readonly removeFile: (fileName: string) => void;
    /** Drop every file, aborting every upload in flight. */
    readonly clearFiles: () => void;
}

/** How `fileSender` posts files. */
export type FileSenderOptions = {
    /** The form field each file is posted as, `file` by default. */
    readonly fieldName?: string;
    /** More request headers, never the content type, which the browser sets. */
    readonly headers?: Record<string, string>;
};

/** No progress yet. */
const NO_PROGRESS: UploadProgress = { loaded: 0, total: 0, percentage: 0 };

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
                const chosen = uploadFilesOf(input.files);
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

    // follow drags over the zone, counting nested enters, and take the files on drop
    createEffect(
        zone,
        (element) => {
            if (element === undefined) {
                return undefined;
            }
            let depth = 0;
            const enter = (event: DragEvent): void => {
                depth++;
                if (depth === 1) {
                    setIsDragging(true);
                    void options.onDragEnter?.(event);
                }
            };
            const leave = (event: DragEvent): void => {
                depth = Math.max(depth - 1, 0);
                if (depth === 0) {
                    setIsDragging(false);
                    void options.onDragLeave?.(event);
                }
            };
            const over = (event: DragEvent): void => {
                event.preventDefault();
                void options.onDragOver?.(event);
            };
            const drop = (event: DragEvent): void => {
                // stop the drag
                event.preventDefault();
                depth = 0;
                setIsDragging(false);
                const dropped = uploadFilesOf(event.dataTransfer?.files ?? null);
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

/** Upload files through a send function, each file in its own request, following each and all of them. */
export function createFileUploader(send: SendFunction): FileUploader {
    // upload nothing on the server
    if (isServer) {
        return {
            upload: async () => [],
            files: [],
            progress: () => NO_PROGRESS,
            status: () => "idle",
            abort: () => {},
            removeFile: () => {},
            clearFiles: () => {},
        };
    }

    // hold each file's state, ignoring writes from uploads a later one replaced
    const [files, setFiles] = createStore<FileUploadEntry[]>([]);
    let controllers: AbortController[] = [];
    let generation = 0;
    const update = (
        index: number,
        current: number,
        change: (entry: FileUploadEntry) => void,
    ): void => {
        if (generation === current) {
            setFiles((draft) => {
                const entry = draft[index];
                if (entry !== undefined) {
                    change(entry);
                }
            });
        }
    };
    const abort = (): void => {
        for (const controller of controllers) {
            controller.abort();
        }
    };

    // send every file, recording its progress, response, failure or abort
    const upload = async (uploadFiles: UploadFile[]): Promise<unknown[]> => {
        // abort the last upload, starting every file anew with a controller of its own
        abort();
        const sends = uploadFiles.map((file) => ({ file, controller: new AbortController() }));
        controllers = sends.map((entry) => entry.controller);
        const current = ++generation;
        setFiles(() =>
            uploadFiles.map((file) => ({
                file,
                progress: NO_PROGRESS,
                status: "uploading",
                error: null,
                response: null,
            })),
        );

        return Promise.all(
            sends.map(async ({ file, controller }, index) => {
                const signal = controller.signal;
                try {
                    const response = await send(
                        file,
                        (progress) =>
                            update(index, current, (entry) => (entry.progress = progress)),
                        signal,
                    );
                    update(index, current, (entry) => {
                        entry.status = "success";
                        entry.response = response;
                    });

                    return response;
                } catch (error) {
                    const isAborted = error instanceof DOMException && error.name === "AbortError";
                    update(index, current, (entry) => {
                        entry.status = isAborted ? "aborted" : "error";
                        entry.error = isAborted ? null : error;
                    });

                    return undefined;
                }
            }),
        );
    };

    // drop files, aborting what is in flight
    const removeFile = (fileName: string): void => {
        // abort and forget what is in flight
        abort();
        controllers = [];
        generation++;
        setFiles((draft) => {
            const index = draft.findIndex((entry) => entry.file.name === fileName);
            if (index !== -1) {
                draft.splice(index, 1);
            }
        });
    };
    const clearFiles = (): void => {
        // abort and forget what is in flight
        abort();
        controllers = [];
        generation++;
        setFiles(() => []);
    };

    // combine the files' progress and status
    const progress = createMemo((): UploadProgress => {
        const loaded = files.reduce((sum, entry) => sum + entry.progress.loaded, 0);
        const total = files.reduce((sum, entry) => sum + entry.progress.total, 0);

        return { loaded, total, percentage: total === 0 ? 0 : Math.round((loaded / total) * 100) };
    }, TRANSPARENT);
    const status = createMemo((): UploadStatus => {
        if (files.length === 0) {
            return "idle";
        }
        for (const candidate of ["uploading", "error", "aborted"] as const) {
            if (files.some((entry) => entry.status === candidate)) {
                return candidate;
            }
        }

        return "success";
    }, TRANSPARENT);
    onCleanup(abort);

    return { upload, files, progress, status, abort, removeFile, clearFiles };
}

/** Make a send function that posts each file in a form through a request reporting upload progress. */
export function fileSender(url: string, options: FileSenderOptions = {}): SendFunction {
    const { fieldName = "file", headers = {} } = options;

    return async (file, onProgress, signal) =>
        new Promise((resolve, reject) => {
            // post the file as a form field
            const form = new FormData();
            form.append(fieldName, file.file, file.name);
            const request = new XMLHttpRequest();
            signal.addEventListener("abort", () => request.abort());

            // report progress, and settle with the response, parsed as JSON when it says it is
            request.upload.addEventListener("progress", (event) => {
                if (event.lengthComputable) {
                    onProgress({
                        loaded: event.loaded,
                        total: event.total,
                        percentage: Math.round((event.loaded / event.total) * 100),
                    });
                }
            });
            request.addEventListener("load", () => {
                if (request.status < 200 || request.status >= 300) {
                    reject(
                        new Error(
                            `upload failed with HTTP ${request.status} ${request.statusText}`,
                        ),
                    );

                    return;
                }
                const isJson = request.getResponseHeader("content-type")?.includes("json") === true;
                resolve(isJson ? JSON.parse(request.responseText) : request.responseText);
            });
            request.addEventListener("error", () =>
                reject(new Error("upload failed on the network")),
            );
            request.addEventListener("abort", () =>
                reject(new DOMException("upload aborted", "AbortError")),
            );

            // send with the headers, leaving the content type to the browser
            request.open("POST", url);
            for (const [name, value] of Object.entries(headers)) {
                if (name.toLowerCase() !== "content-type") {
                    request.setRequestHeader(name, value);
                }
            }
            request.send(form);
        });
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
        const chosen = uploadFilesOf(target.files);
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
function uploadFilesOf(list: FileList | null): UploadFile[] {
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
