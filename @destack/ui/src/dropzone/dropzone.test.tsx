import de from "../../locale/de.json" with { type: "json" };
import { Catalog, Localization } from "@destack/locale";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import {
    Dropzone,
    DropzoneHint,
    DropzoneOpen,
    DropzoneRefusal,
    DropzoneTitle,
    DropzoneTrigger,
} from "./index.ts";

/** The marks a locale isolates a value's direction with. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** Make a file of a name, a type and a size. */
function file(name: string, type: string, size = 1): File {
    return new File([new Uint8Array(size)], name, { type });
}

/** Render a zone, and read what it takes and shows. */
function open(properties: Omit<Parameters<typeof Dropzone>[0], "onFiles">) {
    const taken: string[][] = [];
    const { container } = render(() => (
        <Dropzone
            {...properties}
            onFiles={(files) => void taken.push(files.map(({ name }) => name))}
        />
    ));
    flush();
    const zone = present(container.querySelector("[data-slot=dropzone]"), "the zone");
    const give = (type: "drop" | "paste" | "choose", files: readonly File[]) => {
        // hand over files as the browser does
        const input = present(zone.querySelector("input"), "the picker's input");
        const target = type === "choose" ? input : zone;
        const event = new Event(type === "choose" ? "change" : type, {
            bubbles: true,
            cancelable: true,
        });
        Object.defineProperty(
            type === "choose" ? input : event,
            type === "paste" ? "clipboardData" : type === "drop" ? "dataTransfer" : "files",
            {
                configurable: true,
                value: type === "choose" ? files : { files, types: ["Files"] },
            },
        );
        target.dispatchEvent(event);
        flush();

        return zone
            .querySelector("[data-slot=dropzone-refusal]")
            ?.textContent.replace(ISOLATES, "");
    };

    return { taken, zone, give };
}

test("take files dropped, pasted and chosen, and say why it refuses one", () => {
    const { taken, give } = open({
        multiple: true,
        limits: { accept: ["image/*", "application/pdf"], size: 1000 },
    });
    const refusals = [
        give("drop", [file("a.png", "image/png"), file("b.zip", "application/zip")]),
        give("paste", [file("c.pdf", "application/pdf")]),
        give("choose", [file("d.png", "image/png", 1001)]),
        give("drop", []),
    ];

    expect({ taken, refusals }).toEqual({
        taken: [["a.png"], ["c.pdf"]],
        refusals: [
            "b.zip is another kind of file",
            "",
            "d.png is larger than 1 kB",
            "d.png is larger than 1 kB",
        ],
    });
});

test("take one file unless it takes several, and none while disabled", () => {
    const one = open({});
    const two = one.give("drop", [file("a.txt", "text/plain"), file("b.txt", "text/plain")]);
    one.give("drop", [file("c.txt", "text/plain")]);
    const none = open({ disabled: true });
    none.give("drop", [file("a.txt", "text/plain")]);

    expect({
        two,
        taken: [one.taken, none.taken],
        isDisabled: none.zone.querySelector("button")?.disabled,
    }).toEqual({ two: "Choose one file", taken: [[["c.txt"]], []], isDisabled: true });
});

test("ask for what its limits take, in its label and for the picker", () => {
    const asked = [
        { limits: { accept: ["image/*"], size: 20_000_000 } },
        { limits: { accept: ["application/pdf", "image/*", "image/svg+xml"] } },
        { limits: { size: 5000 }, multiple: true },
        {},
    ].map((properties) => {
        const { zone } = open(properties);

        return [
            zone.querySelector("button")?.getAttribute("aria-label")?.replace(ISOLATES, ""),
            zone.querySelector("input")?.accept,
        ];
    });

    expect(asked).toEqual([
        ["Drop a file here, or choose. Images, up to 20 MB", "image/*"],
        [
            "Drop a file here, or choose. PDF, images, or SVG",
            "application/pdf,image/*,image/svg+xml",
        ],
        ["Drop files here, or choose. Up to 5 kB", ""],
        ["Drop a file here, or choose", ""],
    ]);
});

test("take a caller's words, element and refusals in each part", () => {
    // build a zone of its parts, as a form's section
    const refused: string[][] = [];
    const { zone, give } = open({
        limits: { accept: ["image/*"] },
        onRefuse: (refusals) =>
            void refused.push(refusals.map(({ file: { name }, reason }) => `${name} ${reason}`)),
        render: (attributes) => <section {...attributes} />,
        get children() {
            return (
                <>
                    <DropzoneOpen aria-label="Add photos" />
                    <DropzoneTitle>Photos</DropzoneTitle>
                    <DropzoneHint>JPEG or PNG</DropzoneHint>
                    <DropzoneRefusal>
                        {(refusals) => <b>{refusals.map(({ file: { name } }) => name).join()}</b>}
                    </DropzoneRefusal>
                    <DropzoneTrigger>Browse</DropzoneTrigger>
                </>
            );
        },
    });
    give("drop", [file("a.txt", "text/plain")]);

    expect({
        element: zone.localName,
        text: zone.textContent,
        labels: [...zone.querySelectorAll("[aria-label]")].map((each) =>
            each.getAttribute("aria-label"),
        ),
        refused,
    }).toEqual({
        element: "section",
        text: "PhotosJPEG or PNGa.txtBrowse",
        labels: ["Add photos"],
        refused: [["a.txt type"]],
    });
});

test("ask for files and refuse one in the person's language", () => {
    const german = Localization.of("de", [Catalog.parse(de)]);
    const { container } = render(() => (
        <LocaleContext value={german}>
            <Dropzone
                limits={{ accept: ["application/pdf", "image/*"], size: 5_000_000 }}
                onFiles={() => {}}
            />
        </LocaleContext>
    ));
    flush();
    const zone = present(container.querySelector("[data-slot=dropzone]"), "the zone");
    const event = new Event("drop", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "dataTransfer", {
        value: { files: [file("a.txt", "text/plain")], types: ["Files"] },
    });
    zone.dispatchEvent(event);
    flush();

    expect(
        ["title", "hint", "refusal"].map((part) =>
            zone.querySelector(`[data-slot=dropzone-${part}]`)?.textContent.replace(ISOLATES, ""),
        ),
    ).toEqual([
        "Datei hierher ziehen oder auswählen",
        "PDF oder Bilder, bis zu 5 MB",
        "a.txt hat einen anderen Dateityp",
    ]);
});
