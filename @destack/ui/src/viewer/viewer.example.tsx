import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { Match, Show, Switch } from "@destack/view";
import { FileItem } from "../file-item/index.ts";
import { Gallery, galleryContentStyle, GalleryItem, GridLayout } from "../gallery/index.ts";
import { ViewerControl } from "./control.ts";
import {
    Viewer,
    ViewerClose,
    ViewerCount,
    ViewerDownload,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerTitle,
    ViewerZoom,
    ViewerZoomToggle,
} from "./viewer.tsx";

/** The style of a file's card in its gallery. */
const styles = style.create({
    card: {
        height: "100%",
    },
});

/** A file of a trip. */
interface TripFile {
    /** Its name. */
    readonly name: string;
    /** Its media type. */
    readonly type: string;
    /** Its width divided by its height, one for a file that is no picture. */
    readonly ratio: number;
    /** Its address. */
    readonly source: string;
}

/** The files of a trip: eight photos, and a timetable no browser shows here. */
const FILES: readonly TripFile[] = [
    ...Array.from({ length: 8 }, (_photo, i) => {
        // draw a ridge in the colors of its hour
        const ratio = [1.5, 0.75, 1, 16 / 9][i % 4] ?? 1;
        const width = Math.round(800 * ratio);
        const hue = String((200 + i * 47) % 360);
        const picture = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} 800" width="${String(width)}" height="800"><rect width="100%" height="100%" fill="hsl(${hue} 55% 80%)"/><path d="M0 640 L${String(width * 0.3)} 380 L${String(width * 0.55)} 560 L${String(width * 0.8)} 320 L${String(width)} 520 V800 H0Z" fill="hsl(${hue} 30% 45%)"/><text x="50%" y="30%" font-size="120" text-anchor="middle" fill="hsl(${hue} 40% 30%)">${String(i + 1)}</text></svg>`;

        return {
            name: `Photo ${String(i + 1)}.svg`,
            type: "image/svg+xml",
            ratio,
            source: `data:image/svg+xml,${encodeURIComponent(picture)}`,
        };
    }),
    { name: "Timetable.zip", type: "application/zip", ratio: 1, source: "data:application/zip," },
];

/** A trip's files, opened large over the page. */
export const viewerTrip = defineExample({
    of: Viewer,
    name: "trip",
    description: "a trip's files, opened large over the page",
    layout: "padded",
    render: () => {
        const viewer = new ViewerControl({
            items: () => FILES,
            key: (file) => file.name,
        });

        return (
            <>
                <Gallery each={FILES} layout={new GridLayout({ minimum: 120 })}>
                    {(file) => (
                        <GalleryItem
                            ref={viewer.thumbnail(file())}
                            label={file().name}
                            onOpen={(event) => viewer.open(file(), event)}
                        >
                            <Show
                                when={file().type.startsWith("image/")}
                                fallback={
                                    <FileItem
                                        layout="card"
                                        file={{ ...file(), size: 1200 }}
                                        xstyle={styles.card}
                                    />
                                }
                            >
                                <img
                                    alt=""
                                    src={file().source}
                                    {...style.attributes([galleryContentStyle()])}
                                />
                            </Show>
                        </GalleryItem>
                    )}
                </Gallery>
                <Viewer control={viewer}>
                    {(file) => (
                        <>
                            <ViewerHeader>
                                <ViewerTitle>{file().name}</ViewerTitle>
                                <ViewerCount />
                                <ViewerZoomToggle />
                                <ViewerDownload href={file().source} name={file().name} />
                                <ViewerClose />
                            </ViewerHeader>
                            <ViewerStage control={viewer}>
                                {(shown) => (
                                    <Switch
                                        fallback={<FileItem file={{ ...shown(), size: 1200 }} />}
                                    >
                                        <Match when={shown().type.startsWith("image/")}>
                                            <ViewerZoom ratio={shown().ratio}>
                                                <img
                                                    alt={shown().name}
                                                    src={shown().source}
                                                    {...style.attributes([galleryContentStyle()])}
                                                />
                                            </ViewerZoom>
                                        </Match>
                                    </Switch>
                                )}
                            </ViewerStage>
                            <ViewerPrevious />
                            <ViewerNext />
                        </>
                    )}
                </Viewer>
            </>
        );
    },
});
