import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { Match, Show, Switch } from "@destack/view";
import fileIcon from "@destack/icon/phosphor/file-zip";
import { FILM, sound } from "../example/media.ts";
import { landscape } from "../example/picture.ts";
import { FileItem } from "../file-item/index.ts";
import { Gallery, galleryContentStyle, GalleryItem, GridLayout } from "../gallery/index.ts";
import { Icon } from "../icon/index.ts";
import { AudioPlayer, VideoPlayer } from "../player/index.ts";
import { ViewerControl } from "./control.ts";
import {
    Viewer,
    ViewerActions,
    ViewerClose,
    ViewerCount,
    ViewerDownload,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerStrip,
    ViewerTitle,
    ViewerZoom,
    ViewerZoomToggle,
} from "./viewer.tsx";

/** The style of a file's card in its gallery. */
const styles = style.create({
    card: {
        height: "100%",
    },
    film: {
        width: "min(100cqw, 100cqh * 16 / 9)",
    },
    sound: {
        width: "min(90cqw, 32rem)",
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

/** The files of a trip: eight photos, a film, a sound, and a timetable no browser shows here. */
const FILES: readonly TripFile[] = [
    ...Array.from({ length: 8 }, (_photo, i) => {
        const ratio = [1.5, 0.75, 1, 16 / 9][i % 4] ?? 1;

        return {
            name: `Photo ${String(i + 1)}.svg`,
            type: "image/svg+xml",
            ratio,
            source: landscape(i + 30, ratio),
        };
    }),
    { name: "Pass.mp4", type: "video/mp4", ratio: 16 / 9, source: FILM },
    { name: "Bells.wav", type: "audio/wav", ratio: 1, source: sound([262, 330, 392], 8) },
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
                                <ViewerActions>
                                    <ViewerZoomToggle />
                                    <ViewerDownload href={file().source} name={file().name} />
                                    <ViewerClose />
                                </ViewerActions>
                            </ViewerHeader>
                            <ViewerStage control={viewer}>
                                {(shown) => (
                                    <Switch
                                        fallback={<FileItem file={{ ...shown(), size: 1200 }} />}
                                    >
                                        <Match when={shown().type.startsWith("video/")}>
                                            <VideoPlayer
                                                source={shown().source}
                                                title={shown().name}
                                                xstyle={styles.film}
                                            />
                                        </Match>
                                        <Match when={shown().type.startsWith("audio/")}>
                                            <AudioPlayer
                                                source={shown().source}
                                                title={shown().name}
                                                xstyle={styles.sound}
                                            />
                                        </Match>
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
                            <ViewerStrip control={viewer} label={(each) => each.name}>
                                {(each) => (
                                    <Show
                                        when={each().type.startsWith("image/")}
                                        fallback={<Icon icon={fileIcon} />}
                                    >
                                        <img
                                            alt=""
                                            src={each().source}
                                            {...style.attributes([galleryContentStyle()])}
                                        />
                                    </Show>
                                )}
                            </ViewerStrip>
                            <ViewerPrevious />
                            <ViewerNext />
                        </>
                    )}
                </Viewer>
            </>
        );
    },
});
