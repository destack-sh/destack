import { afterEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import {
    createDropzone,
    createFilePicker,
    createFileUploader,
    dropzone,
    fileSender,
    fileUploader,
    type SendFunction,
    type UploadFile,
    type UploadProgress,
} from "./upload.ts";

afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    document.body.replaceChildren();
});

/** Make a file. */
function file(name = "picture.png"): File {
    return new File(["content"], name, { type: "image/png" });
}

/** A file list of chosen files. */
class StubFileList implements FileList {
    [index: number]: File;

    /** How many files the list holds. */
    readonly length: number;

    /** Hold the files by index. */
    constructor(files: readonly File[]) {
        this.length = files.length;
        for (const [index, item] of files.entries()) {
            this[index] = item;
        }
    }

    /** Read the file at an index. */
    item(index: number): File | null {
        return this[index] ?? null;
    }

    /** Iterate the files in order. */
    [Symbol.iterator](): ArrayIterator<File> {
        return Array.from({ length: this.length }, (_value, index) => this[index])
            .filter((item) => item !== undefined)
            .values();
    }
}

/** Make a file list of files. */
function fileList(...files: File[]): FileList {
    return new StubFileList(files);
}

/** Wrap a file as an upload file. */
function uploadFile(name = "picture.png"): UploadFile {
    const content = file(name);

    return { source: `blob:${name}`, name, size: content.size, file: content };
}

/** Choose files in an input. */
function choose(input: HTMLInputElement, files: FileList): void {
    Object.defineProperty(input, "files", { configurable: true, value: files });
    input.dispatchEvent(new Event("change"));
}

/** Dispatch a drag event, carrying files on drop. */
function drag(target: EventTarget, type: string, files?: FileList): DragEvent {
    const event = new DragEvent(type, { bubbles: true, cancelable: true });
    if (files !== undefined) {
        Object.defineProperty(event, "dataTransfer", { value: { files } });
    }
    target.dispatchEvent(event);
    flush();

    return event;
}

/** Wait for pending callbacks. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
    flush();
}

/** Catch the file inputs a picker clicks open. */
function catchClicks(): { input: HTMLInputElement | undefined } {
    const clicked: { input: HTMLInputElement | undefined } = { input: undefined };
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(function (
        this: HTMLInputElement,
    ) {
        clicked.input = this;
    });

    return clicked;
}

test("open a file picker of the chosen kinds, hold the chosen files and run the callback", async () => {
    // open the picker and catch the input it clicks
    const clicked = catchClicks();
    const received: string[][] = [];
    const picker = createRoot((disposeRoot) => ({
        ...createFilePicker({ multiple: true, accept: "image/*" }),
        dispose: disposeRoot,
    }));
    picker.selectFiles(async (files) => {
        received.push(files.map((item) => item.name));
    });
    const kinds = [clicked.input?.type, clicked.input?.accept, clicked.input?.multiple];

    // choose two files, then drop one and clear the rest
    if (clicked.input !== undefined) {
        choose(clicked.input, fileList(file("first.png"), file("second.png")));
    }
    await settle();
    const chosen = picker.files().map((item) => item.name);
    picker.removeFile("first.png");
    flush();
    const kept = picker.files().map((item) => item.name);
    picker.clearFiles();
    flush();
    const cleared = picker.files();
    picker.dispose();

    expect({
        kinds,
        received,
        chosen,
        kept,
        cleared,
        error: picker.error(),
        isLoading: picker.isLoading(),
    }).toEqual({
        kinds: ["file", "image/*", true],
        received: [["first.png", "second.png"]],
        chosen: ["first.png", "second.png"],
        kept: ["second.png"],
        cleared: [],
        error: null,
        isLoading: false,
    });
});

test("record a picker callback's progress and failure", async () => {
    const clicked = catchClicks();
    let finish: (() => void) | undefined;
    const picker = createRoot((disposeRoot) => ({ ...createFilePicker(), dispose: disposeRoot }));
    picker.selectFiles(
        () =>
            new Promise<void>((_resolve, reject) => {
                finish = () => reject(new Error("callback failed"));
            }),
    );
    if (clicked.input !== undefined) {
        choose(clicked.input, fileList(file()));
    }
    flush();
    const during = picker.isLoading();
    finish?.();
    await settle();
    picker.dispose();

    expect([during, picker.isLoading(), picker.error()]).toEqual([
        true,
        false,
        new Error("callback failed"),
    ]);
});

test("revoke each file's object URL once it is dropped", async () => {
    const clicked = catchClicks();
    const revoked: string[] = [];
    vi.spyOn(URL, "revokeObjectURL").mockImplementation((url) => {
        revoked.push(url);
    });
    const picker = createRoot((disposeRoot) => ({
        ...createFilePicker({ multiple: true }),
        dispose: disposeRoot,
    }));
    picker.selectFiles();
    if (clicked.input !== undefined) {
        choose(clicked.input, fileList(file("first.png"), file("second.png")));
    }
    await settle();
    const sources = picker.files().map((item) => item.source);
    picker.removeFile("first.png");
    picker.dispose();

    expect(revoked).toEqual(sources);
});

test("follow drags over a drop zone and take the dropped files", async () => {
    // drag into a child and out again, telling the callbacks
    const element = document.createElement("div");
    const child = document.createElement("span");
    element.append(child);
    const seen: string[] = [];
    const zone = createRoot((disposeRoot) => {
        const state = createDropzone({
            onDragEnter: (event) => void seen.push(event.type),
            onDragLeave: (event) => void seen.push(event.type),
            onDragOver: (event) => void seen.push(event.type),
            onDrop: (files) => void seen.push(`drop ${files.map((item) => item.name).join(",")}`),
        });
        state.ref(element);

        return { ...state, dispose: disposeRoot };
    });
    flush();
    drag(element, "dragenter");
    drag(child, "dragenter");
    drag(child, "dragleave");
    const isOver = zone.isDragging();
    const over = drag(element, "dragover");

    // drop a file
    drag(element, "drop", fileList(file("dropped.png")));
    await settle();
    const dropped = zone.files().map((item) => item.name);
    zone.dispose();

    expect({
        seen,
        isOver,
        isOverPrevented: over.defaultPrevented,
        dropped,
        isDragging: zone.isDragging(),
    }).toEqual({
        seen: ["dragenter", "dragover", "drop dropped.png"],
        isOver: true,
        isOverPrevented: true,
        dropped: ["dropped.png"],
        isDragging: false,
    });
});

test("move a drop zone's listeners to the element its ref receives next", () => {
    const first = document.createElement("div");
    const second = document.createElement("div");
    const zone = createRoot((disposeRoot) => {
        const state = dropzone();
        state(first);

        return { state, dispose: disposeRoot };
    });
    flush();
    zone.state(second);
    flush();
    drag(first, "dragenter");
    const fromFirst = zone.state.isDragging();
    drag(second, "dragenter");
    const fromSecond = zone.state.isDragging();
    zone.dispose();

    expect([fromFirst, fromSecond]).toEqual([false, true]);
});

test("hand the files chosen in an input to a callback, clearing it, and report failures", async () => {
    // choose files in an input that succeeds and one that fails
    const succeeding = document.createElement("input");
    const failing = document.createElement("input");
    const held: string[][] = [];
    const errors: unknown[] = [];
    const dispose = createRoot((disposeRoot) => {
        fileUploader({
            setFiles: (files) => held.push(files.map((item) => item.name)),
            userCallback: () => {},
        })(succeeding);
        fileUploader({
            setFiles: () => {},
            userCallback: () => {
                throw new Error("callback failed");
            },
            onError: (error) => errors.push(error),
        })(failing);

        return disposeRoot;
    });
    choose(succeeding, fileList(file("chosen.png")));
    choose(failing, fileList(file()));
    await settle();
    dispose();

    expect({ held, errors, value: succeeding.value }).toEqual({
        held: [["chosen.png"]],
        errors: [new Error("callback failed")],
        value: "",
    });
});

test("upload each file in its own request, following each and all of them", async () => {
    // send two files whose requests the test settles
    const requests: {
        name: string;
        progress: (progress: UploadProgress) => void;
        settle: (outcome: unknown) => void;
    }[] = [];
    const send: SendFunction = (sent, onProgress) =>
        new Promise((resolve, reject) => {
            requests.push({
                name: sent.name,
                progress: onProgress,
                settle: (outcome) =>
                    outcome instanceof Error ? reject(outcome) : resolve(outcome),
            });
        });
    const uploader = createRoot((disposeRoot) => ({
        ...createFileUploader(send),
        dispose: disposeRoot,
    }));
    const idle = uploader.status();
    const uploaded = uploader.upload([uploadFile("first.png"), uploadFile("second.png")]);
    flush();
    const uploading = uploader.status();

    // report progress, then let one succeed and one fail
    requests[0]?.progress({ loaded: 50, total: 100, percentage: 50 });
    requests[1]?.progress({ loaded: 0, total: 100, percentage: 0 });
    flush();
    const progress = uploader.progress();
    requests[0]?.settle({ ok: true });
    requests[1]?.settle(new Error("upload failed"));
    const responses = await uploaded;
    flush();
    const entries = uploader.files.map((entry) => [
        entry.file.name,
        entry.status,
        entry.response,
        entry.error,
    ]);
    const status = uploader.status();
    uploader.dispose();

    expect({ idle, uploading, progress, responses, entries, status }).toEqual({
        idle: "idle",
        uploading: "uploading",
        progress: { loaded: 50, total: 200, percentage: 25 },
        responses: [{ ok: true }, undefined],
        entries: [
            ["first.png", "success", { ok: true }, null],
            ["second.png", "error", null, new Error("upload failed")],
        ],
        status: "error",
    });
});

test("abort uploads, and drop files while aborting what is in flight", async () => {
    // abort a request that listens to its signal
    const signals: AbortSignal[] = [];
    const send: SendFunction = (_file, _onProgress, signal) =>
        new Promise((_resolve, reject) => {
            signals.push(signal);
            signal.addEventListener("abort", () =>
                reject(new DOMException("upload aborted", "AbortError")),
            );
        });
    const uploader = createRoot((disposeRoot) => ({
        ...createFileUploader(send),
        dispose: disposeRoot,
    }));
    const first = uploader.upload([uploadFile()]);
    uploader.abort();
    await first;
    flush();
    const aborted = uploader.status();

    // upload two files, then drop one and clear the rest
    void uploader.upload([uploadFile("first.png"), uploadFile("second.png")]);
    flush();
    uploader.removeFile("first.png");
    flush();
    const kept = uploader.files.map((entry) => entry.file.name);
    uploader.clearFiles();
    flush();
    uploader.dispose();

    expect({
        aborted,
        signals: signals.map((signal) => signal.aborted),
        kept,
        files: uploader.files.length,
        status: uploader.status(),
    }).toEqual({
        aborted: "aborted",
        signals: [true, true, true],
        kept: ["second.png"],
        files: 0,
        status: "idle",
    });
});

test("post files as a form with headers and progress, parsing JSON responses", async () => {
    // stand in for the request, recording what it is told
    const calls: string[] = [];
    const listeners = new Map<string, () => void>();
    const progressListeners: ((event: ProgressEvent) => void)[] = [];
    let type = "application/json";
    let status = 200;
    class StubRequest {
        /** The status line. */
        statusText = "OK";
        /** The response body. */
        responseText = '{"ok":true}';
        /** The upload whose progress the test reports. */
        readonly upload = {
            addEventListener: (_type: string, listener: (event: ProgressEvent) => void) =>
                progressListeners.push(listener),
        };
        /** Read the status code. */
        get status(): number {
            return status;
        }
        /** Record the method and address. */
        open(method: string, url: string): void {
            calls.push(`${method} ${url}`);
        }
        /** Record a header. */
        setRequestHeader(name: string, value: string): void {
            calls.push(`${name}: ${value}`);
        }
        /** Record the body's kind. */
        send(body: unknown): void {
            calls.push(body instanceof FormData ? "form" : "other");
        }
        /** Abort nothing. */
        abort(): void {}
        /** Read the content type. */
        getResponseHeader(): string {
            return type;
        }
        /** Keep a listener. */
        addEventListener(name: string, listener: () => void): void {
            listeners.set(name, listener);
        }
    }
    vi.stubGlobal("XMLHttpRequest", StubRequest);
    const send = fileSender("/upload", {
        headers: { "X-Token": "secret", "Content-Type": "ignored" },
    });

    // send, report progress, and load as JSON, as text, and as a failure
    const progress: UploadProgress[] = [];
    const json = send(
        uploadFile(),
        (reported) => progress.push(reported),
        new AbortController().signal,
    );
    for (const listener of progressListeners) {
        listener(
            new ProgressEvent("progress", { lengthComputable: true, loaded: 256, total: 1024 }),
        );
    }
    listeners.get("load")?.();
    type = "text/plain";
    const text = send(uploadFile(), () => {}, new AbortController().signal);
    listeners.get("load")?.();
    status = 413;
    const failed = send(uploadFile(), () => {}, new AbortController().signal).catch(
        (error: unknown) => error,
    );
    listeners.get("load")?.();

    expect({
        calls: calls.slice(0, 3),
        progress,
        json: await json,
        text: await text,
        failed: await failed,
    }).toEqual({
        calls: ["POST /upload", "X-Token: secret", "form"],
        progress: [{ loaded: 256, total: 1024, percentage: 25 }],
        json: { ok: true },
        text: '{"ok":true}',
        failed: new Error("upload failed with HTTP 413 OK"),
    });
});
