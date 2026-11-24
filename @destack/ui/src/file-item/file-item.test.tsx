import { expect, test } from "@destack/test";
import type { UploadReason, UploadStatus } from "@destack/object/client";
import { present } from "@destack/schema";
import { createSignal, flush, For } from "@destack/view";
import { render } from "@destack/view/test";
import {
    FileItem,
    FileItemActions,
    FileItemContent,
    FileItemDescription,
    FileItemGroup,
    FileItemMedia,
    FileItemOpen,
    FileItemPicture,
    FileItemRemove,
    FileItemRetry,
    FileItemTitle,
    FileKind,
    FileSize,
} from "./index.ts";

/** The marks a locale isolates a value's direction with. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** A photo. */
const PHOTO = { name: "tram.jpg", size: 1_100_000, type: "image/jpeg" };

/** An upload a test moves on. */
function testUpload() {
    const listeners = new Set<() => void>();
    const upload: {
        status: UploadStatus;
        reason: UploadReason | undefined;
        loaded: number;
        readonly size: number;
        readonly watch: (listener: () => void) => () => void;
    } = {
        status: "sending",
        reason: undefined,
        loaded: 0,
        size: 200,
        watch: (listener) => {
            listeners.add(listener);

            return () => void listeners.delete(listener);
        },
    };
    const move = (status: UploadStatus, loaded: number) => {
        Object.assign(upload, { status, loaded });
        for (const listener of listeners) {
            listener();
        }
        flush();
    };

    return { upload, move, listeners };
}

/** Write a number in English. */
function write(value: number, options: Intl.NumberFormatOptions): string {
    return new Intl.NumberFormat("en", options).format(value);
}

/** Read an element's label without its isolates. */
function labelOf(element: Element | null): string | undefined {
    return element?.getAttribute("aria-label")?.replace(ISOLATES, "");
}

/** Find the element of a label. */
function labelled(label: string): HTMLElement {
    const found = [...document.querySelectorAll<HTMLElement>("[aria-label]")].find(
        (each) => labelOf(each) === label,
    );

    return present(found, `the element '${label}'`);
}

/** Read a file's state, its description and how far it is sent. */
function shown() {
    const item = document.querySelector("[data-slot=file-item]");

    return [
        item?.getAttribute("data-state"),
        item?.querySelector("[data-slot=file-item-description]")?.textContent,
        item?.querySelector("[role=progressbar]")?.getAttribute("aria-valuenow"),
        item?.querySelector("[data-slot=file-item-open]")?.getAttribute("href"),
    ];
}

test("follow an upload until its file opens", () => {
    const { upload, move, listeners } = testUpload();
    const rendered = render(() => <FileItem file={PHOTO} source="/tram.jpg" upload={upload} />);
    const before = shown();
    move("sending", 128);
    const sending = shown();
    move("paused", 128);
    const paused = shown();
    move("failed", 128);
    const failed = shown();
    upload.reason = "connection";
    move("failed", 128);
    const lost = shown();
    upload.reason = undefined;
    move("cancelled", 128);
    const cancelled = shown();
    move("done", 200);
    const done = shown();
    rendered.unmount();

    expect({
        before,
        sending,
        paused,
        failed,
        lost,
        cancelled,
        done,
        listeners: listeners.size,
    }).toEqual({
        before: ["sending", "Sending", null, undefined],
        sending: ["sending", "Sending · 64%", "64", undefined],
        paused: ["paused", "Paused · 64%", "64", undefined],
        failed: ["failed", "Not sent", undefined, "/tram.jpg"],
        lost: ["failed", "Connection lost", undefined, "/tram.jpg"],
        cancelled: ["cancelled", "Not sent", undefined, "/tram.jpg"],
        done: ["done", "JPG · 1.1 MB", undefined, "/tram.jpg"],
        listeners: 0,
    });
});

test("open in place on a plain click, and leave a click for a new tab to the browser", () => {
    let count = 0;
    render(() => <FileItem file={PHOTO} source="/tram.jpg" onOpen={() => void (count += 1)} />);
    flush();
    const link = labelled("Open tram.jpg");
    const isLeft = [{}, { metaKey: true }, { ctrlKey: true }].map((keys) =>
        link.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, ...keys })),
    );

    expect({ count, isLeft }).toEqual({ count: 1, isLeft: [false, true, true] });
});

test("move the focus to the next file as one is removed, and send a failed one again", async () => {
    const [names, setNames] = createSignal(["a.txt", "b.txt", "c.txt"]);
    let retried = "";
    render(() => (
        <FileItemGroup aria-label="Files">
            <For each={names()}>
                {(name) => (
                    <FileItem
                        file={{ name, size: 1, type: "text/plain" }}
                        error={name === "c.txt" ? "Too large" : undefined}
                        onRetry={() => void (retried = name)}
                        onRemove={() => setNames((left) => left.filter((each) => each !== name))}
                    />
                )}
            </For>
        </FileItemGroup>
    ));
    flush();
    const removing = labelled("Remove b.txt");
    removing.focus();
    removing.click();
    flush();
    await Promise.resolve();
    labelled("Send c.txt again").click();

    expect({
        listed: document.querySelectorAll("[role=listitem]").length,
        focused: labelOf(document.activeElement),
        alert: document.querySelector("[role=alert]")?.textContent,
        retried,
    }).toEqual({ listed: 2, focused: "Remove c.txt", alert: "Too large", retried: "c.txt" });
});

test("show the kind of a picture the browser cannot show", () => {
    render(() => <FileItem file={PHOTO} source="/broken.heic" />);
    const picture = document.querySelector("img");
    present(picture, "the picture").dispatchEvent(new Event("error"));
    flush();

    expect([
        document.querySelector("img"),
        document.querySelectorAll("[data-slot=file-item-media] svg").length,
    ]).toEqual([null, 1]);
});

test("tell the kinds of files by their extensions and types", () => {
    const kinds = [
        ["a.pdf", ""],
        ["a.xlsx", ""],
        ["a.ts", "video/mp2t"],
        ["a", "image/png"],
        ["a", "audio/mpeg"],
        ["a.mov", "video/quicktime"],
        ["a.zip", "application/zip"],
        ["a.docx", ""],
        ["a", ""],
    ].map(([name = "", type]) => FileKind.of(name, type));

    expect(kinds).toEqual([
        "pdf",
        "sheet",
        "code",
        "image",
        "audio",
        "video",
        "archive",
        "text",
        "other",
    ]);
});

test("write sizes in the largest unit they have one of once rounded", () => {
    expect(
        [0, 1, 999, 1000, 999_949, 999_950, 5_000_000_000, 2e15].map((size) =>
            FileSize.write(size, write),
        ),
    ).toEqual(["0 bytes", "1 byte", "999 bytes", "1 kB", "999.9 kB", "1 MB", "5 GB", "2,000 TB"]);
});

test("take a caller's words, icons, handlers and element in each part", () => {
    // build a file of its parts, as a list's cell
    const heard: string[] = [];
    const { container } = render(() => (
        <FileItem
            file={PHOTO}
            error="Too large"
            onOpen={() => {}}
            onRetry={() => heard.push("retry")}
            onRemove={() => heard.push("remove")}
            render={(attributes) => <li {...attributes} />}
        >
            <FileItemOpen aria-label="View it" />
            <FileItemMedia>
                <FileItemPicture fallback={<b>JPG</b>} />
            </FileItemMedia>
            <FileItemContent>
                <FileItemTitle>Tram 28</FileItemTitle>
                <FileItemDescription>Ada, yesterday</FileItemDescription>
            </FileItemContent>
            <FileItemActions>
                <FileItemRetry aria-label="Try again" onClick={() => heard.push("clicked")}>
                    Again
                </FileItemRetry>
                <FileItemRemove aria-label="Drop it">Drop</FileItemRemove>
            </FileItemActions>
        </FileItem>
    ));
    flush();
    labelled("Try again").click();
    labelled("Drop it").click();

    expect({
        element: container.firstElementChild?.localName,
        text: container.textContent,
        labels: [...container.querySelectorAll("[aria-label]")].map((each) =>
            each.getAttribute("aria-label"),
        ),
        heard,
    }).toEqual({
        element: "li",
        text: "JPGTram 28Ada, yesterdayAgainDrop",
        labels: ["View it", "Try again", "Drop it"],
        heard: ["clicked", "retry", "remove"],
    });
});

test("tell why a file was refused beside its name", () => {
    const limits = { accept: ["image/*"], size: 1_000_000 };
    const file = new File([], "tram.jpg");
    const told = (["type", "size", "count"] as const).map((reason) => {
        const { container } = render(() => (
            <FileItem file={PHOTO} error={{ file, reason, limits }} />
        ));
        flush();

        return container
            .querySelector("[data-slot=file-item-description]")
            ?.textContent.replace(ISOLATES, "");
    });

    expect(told).toEqual(["Another kind of file", "Larger than 1 MB", "Choose one file"]);
});
