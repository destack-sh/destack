import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { radius, space } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX, Show } from "@destack/view";
import { Button } from "../button/index.ts";
import { PICTURES } from "../example/picture.ts";
import { useCropper } from "./control.ts";
import {
    Cropper,
    CropperFlip,
    CropperGrid,
    CropperImage,
    CropperRotate,
    CropperSelection,
    CropperViewport,
    CropperZoom,
} from "./cropper.tsx";

/** The styles of the examples. */
const styles = style.create({
    cropper: {
        maxWidth: "24rem",
    },
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    result: {
        width: "6rem",
        height: "6rem",
        borderRadius: radius.full,
    },
});

/** Render the button that shows the picture a person cropped. */
function Save(): JSX.Element {
    // write the part once asked for
    const cropper = useCropper();
    const [result, setResult] = createSignal<string | undefined>(undefined);
    const save = async (): Promise<void> => {
        const blob = await cropper.toBlob({ type: "image/jpeg", quality: 0.9, width: 192 });
        setResult(URL.createObjectURL(blob));
    };

    return (
        <div {...style.attrs(styles.row)}>
            <Button onClick={() => void save()}>Save</Button>
            <Show when={result()}>
                {(source) => (
                    <img src={source()} alt="The picture" {...style.attrs(styles.result)} />
                )}
            </Show>
        </div>
    );
}

/** A profile picture, which moves below a round frame. */
export const cropperProfile = defineExample({
    of: Cropper,
    name: "profile",
    layout: "padded",
    description: t`A profile picture, which moves, zooms and turns below a round frame.`,
    render: () => (
        <Cropper fixedCropArea aspectRatio={1} xstyle={styles.cropper}>
            <CropperViewport>
                <CropperImage src={PICTURES.alfama} crossorigin="anonymous" alt="Roofs of Alfama" />
                <CropperSelection cropShape="circle" />
            </CropperViewport>
            <div {...style.attrs(styles.row)}>
                <CropperZoom />
                <CropperRotate />
                <CropperFlip />
            </div>
            <Save />
        </Cropper>
    ),
});

/** A part of a photo, its sides moved by handles. */
export const cropperPhoto = defineExample({
    of: Cropper,
    name: "photo",
    layout: "padded",
    description: t`A part of a photo, its sides moved by handles.`,
    render: () => (
        <Cropper defaultValue={{ x: 40, y: 30, width: 200, height: 120 }} xstyle={styles.cropper}>
            <CropperViewport>
                <CropperImage src={PICTURES.tram} alt="A tram on the hill" />
                <CropperSelection />
            </CropperViewport>
            <div {...style.attrs(styles.row)}>
                <CropperRotate direction={-1} />
                <CropperRotate />
                <CropperFlip />
                <CropperFlip axis="vertical" />
            </div>
        </Cropper>
    ),
});

/** A page's cover, wide and not tall. */
export const cropperCover = defineExample({
    of: Cropper,
    name: "cover",
    layout: "padded",
    description: t`A page's cover, whose handles keep its ratio.`,
    render: () => (
        <Cropper aspectRatio={3} xstyle={styles.cropper}>
            <CropperViewport>
                <CropperImage src={PICTURES.belem} alt="The tower of Belém" />
                <CropperSelection />
            </CropperViewport>
        </Cropper>
    ),
});

/** A profile picture below its frame. */
export const cropperSelectionProfile = defineExample({
    of: CropperSelection,
    name: "profile",
    description: t`A profile picture below its frame, round as an avatar shows it.`,
    layout: "padded",
    properties: {
        cropShape: "circle",
    },
    render: (properties) => (
        <Cropper fixedCropArea aspectRatio={1} xstyle={styles.cropper}>
            <CropperViewport>
                <CropperImage src={PICTURES.alfama} crossorigin="anonymous" alt="Roofs of Alfama" />
                <CropperSelection {...properties} />
            </CropperViewport>
        </Cropper>
    ),
});

/** A photo's thirds over its frame, to line up a horizon. */
export const cropperGridThirds = defineExample({
    of: CropperGrid,
    name: "thirds",
    description: t`A photo's thirds over its frame, to line up a horizon.`,
    layout: "padded",
    properties: {
        keepShown: true,
    },
    render: (properties) => (
        <Cropper aspectRatio={16 / 9} xstyle={styles.cropper}>
            <CropperViewport>
                <CropperImage src={PICTURES.tram} crossorigin="anonymous" alt="A tram on a hill" />
                <CropperSelection>
                    <CropperGrid {...properties} />
                </CropperSelection>
            </CropperViewport>
        </Cropper>
    ),
});
