import { t } from "@destack/locale";
import downloadSimple from "@destack/icon/phosphor/download-simple";
import play from "@destack/icon/phosphor/play";
import waveform from "@destack/icon/phosphor/waveform";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import * as style from "@destack/style";
import { PICTURES } from "../example/picture.ts";
import { galleryContentStyle } from "../gallery/index.ts";
import { Icon } from "../icon/index.ts";
import { Stack } from "../stack/index.ts";
import {
    FileItem,
    FileItemAction,
    FileItemActions,
    FileItemContent,
    FileItemDescription,
    FileItemGroup,
    FileItemMedia,
    FileItemOpen,
    FileItemRemove,
    FileItemTitle,
} from "./file-item.tsx";
import {
    Viewer,
    ViewerActions,
    ViewerClose,
    ViewerControl,
    ViewerDownload,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerTitle,
    ViewerZoom,
} from "../viewer/index.ts";

/** A trip's files. */
const FILES = [
    { name: "itinerary.pdf", size: 2_400_000, type: "application/pdf" },
    { name: "Budget for the trip in spring, with every quote we got.xlsx", size: 48_120, type: "" },
    { name: "packing.ts", size: 812, type: "text/typescript" },
    { name: "fado.mp3", size: 4_100_000, type: "audio/mpeg" },
    { name: "tram-28.mov", size: 81_000_000, type: "video/quicktime" },
    { name: "tickets.zip", size: 120_400, type: "application/zip" },
] as const;

/** A trip's photos. */
const PHOTOS = [
    { name: "alfama.jpg", size: 820_000, type: "image/jpeg", source: PICTURES.alfama },
    { name: "tram-28-on-the-hill.jpg", size: 1_100_000, type: "image/jpeg", source: PICTURES.tram },
    { name: "pastries.jpg", size: 940_000, type: "image/jpeg", source: PICTURES.belem },
] as const;

/** An upload with two thirds of its bytes sent. */
const SENDING = {
    status: "sending",
    reason: undefined,
    loaded: 64,
    size: 100,
    watch: () => () => {},
} as const;

/** An upload a person paused. */
const PAUSED = { ...SENDING, status: "paused", loaded: 30 } as const;

/** An upload whose connection was lost. */
const LOST = { ...SENDING, status: "failed", reason: "connection" } as const;

/** A photo larger than its field takes. */
const REFUSED = {
    file: new File([], "tram-28-on-the-hill.jpg"),
    reason: "size",
    limits: { size: 1_000_000 },
} as const;

/** A trip's files in rows, which open and download. */
export const fileItemRows = defineExample({
    of: FileItem,
    name: "rows",
    description: t`A trip's files in rows, which open and download.`,
    layout: "padded",
    render: (properties) => (
        <FileItemGroup aria-label="Files">
            <For each={FILES}>
                {(file) => (
                    <FileItem {...properties} file={file} onOpen={() => {}}>
                        <FileItemOpen />
                        <FileItemMedia />
                        <FileItemContent>
                            <FileItemTitle />
                            <FileItemDescription />
                        </FileItemContent>
                        <FileItemActions>
                            <FileItemAction aria-label="Download">
                                <Icon icon={downloadSimple} />
                            </FileItemAction>
                        </FileItemActions>
                    </FileItem>
                )}
            </For>
        </FileItemGroup>
    ),
});

/** A trip's photos as cards, which a person opens and removes. */
export const fileItemGroupCards = defineExample({
    of: FileItemGroup,
    name: "cards",
    description: t`A trip's photos as cards, which a person opens and removes.`,
    layout: "padded",
    properties: {
        layout: "card",
    },
    render: (properties) => {
        const [photos, setPhotos] = createSignal<readonly (typeof PHOTOS)[number][]>(PHOTOS);
        const viewer = new ViewerControl({ items: photos, key: (photo) => photo.name });

        return (
            <>
                <FileItemGroup {...properties} aria-label="Photos">
                    <For each={photos()}>
                        {(photo) => (
                            <FileItem
                                ref={viewer.thumbnail(photo)}

                                file={photo}
                                source={photo.source}
                                onOpen={() => viewer.open(photo)}
                                onRemove={() =>
                                    setPhotos((left) =>
                                        left.filter(({ name }) => name !== photo.name),
                                    )
                                }
                            />
                        )}
                    </For>
                </FileItemGroup>
                <Viewer control={viewer}>
                    {(photo) => (
                        <>
                            <ViewerHeader>
                                <ViewerTitle>{photo().name}</ViewerTitle>
                                <ViewerActions>
                                    <ViewerDownload href={photo().source} name={photo().name} />
                                    <ViewerClose />
                                </ViewerActions>
                            </ViewerHeader>
                            <ViewerStage control={viewer}>
                                {(shown) => (
                                    <ViewerZoom ratio={4 / 3}>
                                        <img
                                            alt={shown().name}
                                            src={shown().source}
                                            {...style.attributes([galleryContentStyle()])}
                                        />
                                    </ViewerZoom>
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

/** A trip's recordings of other parts: a drawn wave, who sent it and when. */
export const fileItemParts = defineExample({
    of: FileItem,
    name: "parts",
    description: t`A trip's recordings of other parts: a drawn wave, who sent it and when.`,
    layout: "padded",
    render: (properties) => (
        <FileItemGroup aria-label="Recordings">
            <FileItem {...properties} file={FILES[3]} onOpen={() => {}} onRemove={() => {}}>
                <FileItemOpen aria-label="Play fado.mp3" />
                <FileItemMedia>
                    <Icon icon={waveform} />
                </FileItemMedia>
                <FileItemContent>
                    <FileItemTitle>Fado at Tasca do Chico</FileItemTitle>
                    <FileItemDescription>Ada · 3:12</FileItemDescription>
                </FileItemContent>
                <FileItemActions>
                    <FileItemAction aria-label="Play">
                        <Icon icon={play} />
                    </FileItemAction>
                    <FileItemRemove />
                </FileItemActions>
            </FileItem>
        </FileItemGroup>
    ),
});

/** A message's files on their way: sent, paused and failed, in rows and as cards. */
export const fileItemSending = defineExample({
    of: FileItem,
    name: "sending",
    description: t`A message's files on their way: sent, paused and failed, in rows and as cards.`,
    layout: "padded",
    render: (properties) => (
        <Stack space="4">
            <FileItemGroup aria-label="Files">
                <FileItem {...properties} file={FILES[0]} upload={SENDING} onRemove={() => {}} />
                <FileItem file={FILES[4]} upload={PAUSED} onRemove={() => {}} />
                <FileItem file={FILES[5]} upload={LOST} onRetry={() => {}} onRemove={() => {}} />
            </FileItemGroup>
            <FileItemGroup layout="card" aria-label="Photos">
                <FileItem
                    file={PHOTOS[0]}
                    source={PICTURES.alfama}
                    upload={SENDING}
                    onRemove={() => {}}
                />
                <FileItem
                    file={PHOTOS[1]}
                    source={PICTURES.tram}
                    error={REFUSED}
                    onRemove={() => {}}
                />
                <FileItem file={FILES[0]} upload={SENDING} onRemove={() => {}} />
            </FileItemGroup>
        </Stack>
    ),
});
