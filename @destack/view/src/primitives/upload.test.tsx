import { afterEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createDropzone, createFilePicker, dropzone, fileUploader } from "./upload.ts";
import { wait } from "../test/dom.ts";

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

    /** How many files the list has. */
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

/** Choose files in an input. */
function choose(input: HTMLInputElement, files: FileList): void {
    Object.defineProperty(input, "files", { configurable: true, value: files });
    input.dispatchEvent(new Event("change"));
}

/** Dispatch a drag event of files, shown on drop, or of text. */
function drag(
    target: EventTarget,
    type: string,
    files: FileList = fileList(),
    types: readonly string[] = ["Files"],
): DragEvent {
    const event = new DragEvent(type, { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", { value: { files, types } });
    target.dispatchEvent(event);
    flush();

    return event;
}

/** Wait for pending callbacks. */
async function flushPending(): Promise<void> {
    await wait(0);
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
    await flushPending();
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
    await flushPending();
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
    await flushPending();
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
    await flushPending();
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

test("leave a drag of text to the page", async () => {
    const element = document.createElement("div");
    const zone = createRoot((dispose) => {
        const state = createDropzone();
        state.ref(element);

        return { ...state, dispose };
    });
    flush();
    const events = ["dragenter", "dragover", "drop"].map((type) =>
        drag(element, type, fileList(), ["text/plain"]),
    );
    const isDragging = zone.isDragging();
    zone.dispose();

    expect([isDragging, events.map((event) => event.defaultPrevented)]).toEqual([
        false,
        [false, false, false],
    ]);
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
    await flushPending();
    dispose();

    expect({ held, errors, value: succeeding.value }).toEqual({
        held: [["chosen.png"]],
        errors: [new Error("callback failed")],
        value: "",
    });
});
