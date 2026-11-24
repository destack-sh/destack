import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import {
    Viewer,
    ViewerActions,
    ViewerClose,
    ViewerControl,
    ViewerCount,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerStrip,
    ViewerTitle,
    ViewerZoom,
    ViewerZoomToggle,
} from "../viewer/index.ts";
import { landscape } from "../example/picture.ts";
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

/** The photos of a trip, each at its hour. */
const PHOTOS: readonly Photo[] = Array.from({ length: 23 }, (_photo, i) => {
    const ratio = RATIOS[(i * 5) % RATIOS.length] ?? 1;

    return {
        title: `${TITLES[i % TITLES.length] ?? ""} ${String(i + 1)}`,
        ratio,
        source: landscape(i, ratio),
    };
});

/** The photos of a trip in a layout, each opening in one viewer. */
function Photos(properties: {
    /** How they are laid out. */
    readonly layout: Layout;
    /** How many there are. */
    readonly count: number;
}) {
    const photos = () => PHOTOS.slice(0, properties.count);
    const viewer = new ViewerControl({ items: photos, key: (photo) => photo.title });

    return (
        <>
            <Gallery
                aria-label="Trip"
                each={photos()}
                ratio={(photo) => photo.ratio}
                layout={properties.layout}
            >
                {(photo) => (
                    <GalleryItem
                        ref={viewer.thumbnail(photo())}
                        label={photo().title}
                        onOpen={(event) => viewer.open(photo(), event)}
                    >
                        <img
                            alt=""
                            src={photo().source}
                            {...style.attributes([galleryContentStyle()])}
                        />
                    </GalleryItem>
                )}
            </Gallery>
            <Viewer control={viewer}>
                {(photo) => (
                    <>
                        <ViewerHeader>
                            <ViewerTitle>{photo().title}</ViewerTitle>
                            <ViewerCount />
                            <ViewerActions>
                                <ViewerZoomToggle />
                                <ViewerClose />
                            </ViewerActions>
                        </ViewerHeader>
                        <ViewerStage control={viewer}>
                            {(shown) => (
                                <ViewerZoom ratio={shown().ratio}>
                                    <img
                                        alt={shown().title}
                                        src={shown().source}
                                        {...style.attributes([galleryContentStyle()])}
                                    />
                                </ViewerZoom>
                            )}
                        </ViewerStage>
                        <ViewerStrip control={viewer} label={(each) => each.title}>
                            {(each) => (
                                <img
                                    alt=""
                                    src={each().source}
                                    {...style.attributes([galleryContentStyle()])}
                                />
                            )}
                        </ViewerStrip>
                        <ViewerPrevious />
                        <ViewerNext />
                    </>
                )}
            </Viewer>
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
