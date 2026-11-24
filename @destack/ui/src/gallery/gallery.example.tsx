import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import { Dialog, DialogContent, DialogControl, DialogTitle } from "../dialog/index.ts";
import { OpenChange } from "../layer/index.ts";
import { Gallery, galleryContentStyle, GalleryItem } from "./gallery.tsx";
import { CollageLayout, GridLayout, type Layout, MasonryLayout, RowLayout } from "./layout.ts";
import * as style from "@destack/style";

/** A photo of a trip. */
interface Photo {
    /** What it shows. */
    readonly title: string;
    /** Its width divided by its height. */
    readonly ratio: number;
    /** Its address. */
    readonly source: string;
}

/** What the photos show. */
const TITLES = ["Lake", "Pass", "Tram", "Hut", "Village", "Glacier", "Station"];

/** The ratios of the photos, as a camera, a phone and a panorama take them. */
const RATIOS = [1.5, 4 / 3, 0.75, 1, 16 / 9, 9 / 16, 2.4, 1.5, 0.8];

/** The photos of a trip: a sky over two ridges, drawn in the colors of its hour. */
const PHOTOS: readonly Photo[] = Array.from({ length: 23 }, (_photo, i) => {
    // draw each in the colors of its hour
    const ratio = RATIOS[(i * 5) % RATIOS.length] ?? 1;
    const [width, hue] = [Math.round(400 * ratio), (200 + i * 37) % 360];
    const ridge = (top: number, light: number): string =>
        `<path d="M0 ${String(top)} L${String(width * 0.3)} ${String(top - 90)} L${String(width * 0.55)} ${String(top - 20)} L${String(width * 0.8)} ${String(top - 120)} L${String(width)} ${String(top - 40)} V400 H0Z" fill="hsl(${String(hue)} 30% ${String(light)}%)"/>`;
    const picture = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} 400" width="${String(width)}" height="400"><rect width="100%" height="100%" fill="hsl(${String(hue)} 55% 80%)"/><circle cx="${String(width * 0.7)}" cy="90" r="34" fill="hsl(${String((hue + 180) % 360)} 70% 88%)"/>${ridge(290, 52)}${ridge(370, 34)}</svg>`;

    return {
        title: `${TITLES[i % TITLES.length] ?? ""} ${String(i + 1)}`,
        ratio,
        source: `data:image/svg+xml,${encodeURIComponent(picture)}`,
    };
});

/** The photos of a trip in a layout, each opening in one dialog. */
function Photos(properties: {
    /** How they are laid out. */
    readonly layout: Layout;
    /** How many there are. */
    readonly count: number;
}) {
    const viewer = new DialogControl<Photo>();

    return (
        <>
            <Gallery
                aria-label="Trip"
                each={PHOTOS.slice(0, properties.count)}
                ratio={(photo) => photo.ratio}
                layout={properties.layout}
            >
                {(photo) => (
                    <GalleryItem
                        label={photo().title}
                        onOpen={(event) => {
                            const trigger = { element: event.currentTarget, payload: photo() };
                            viewer.open(new OpenChange("trigger-press", event, trigger));
                        }}
                    >
                        <img
                            alt=""
                            src={photo().source}
                            {...style.attributes([galleryContentStyle()])}
                        />
                    </GalleryItem>
                )}
            </Gallery>
            <Dialog control={viewer}>
                {(photo) => (
                    <DialogContent>
                        <DialogTitle>{photo()?.title}</DialogTitle>
                        <img alt="" src={photo()?.source ?? ""} />
                    </DialogContent>
                )}
            </Dialog>
        </>
    );
}

/** A trip's photos in rows that fill the page. */
export const galleryRows = defineExample({
    of: Gallery,
    name: "rows",
    description: "a trip's photos in rows that fill the page",
    layout: "padded",
    render: () => <Photos layout={new RowLayout({ height: 150 })} count={23} />,
});

/** A trip's photos as squares. */
export const galleryGrid = defineExample({
    of: Gallery,
    name: "grid",
    description: "a trip's photos as squares",
    layout: "padded",
    render: () => <Photos layout={new GridLayout({ minimum: 110 })} count={23} />,
});

/** A trip's photos in columns, each under the shortest. */
export const galleryMasonry = defineExample({
    of: Gallery,
    name: "masonry",
    description: "a trip's photos in columns, each under the shortest",
    layout: "padded",
    render: () => <Photos layout={new MasonryLayout({ minimum: 140 })} count={23} />,
});

/** The photos of a message: one to four of them, and the first five of many. */
export const galleryCollage = defineExample({
    of: Gallery,
    name: "collage",
    description: "the photos of a message: one to four of them, and the first five of many",
    layout: "padded",
    render: () => {
        const [count, setCount] = createSignal(23);

        return (
            <>
                <label>
                    Photos{" "}
                    <input
                        type="number"
                        min="1"
                        max="23"
                        value={count()}
                        onInput={(event) => setCount(Number(event.currentTarget.value))}
                    />
                </label>
                <Photos layout={new CollageLayout({ count: 5 })} count={count()} />
            </>
        );
    },
});
