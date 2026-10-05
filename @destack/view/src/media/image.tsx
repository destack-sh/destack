import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import { For } from "../solid/flow.ts";
import { createSignal, omit } from "../solid/reactive.ts";
import type { ImageAsset } from "./asset.ts";

/** The formats a picture offers before its original, best first. */
const PICTURE_TYPES = ["image/avif", "image/webp"];

/** The format an image prefers for its single source. */
const IMAGE_TYPE = "image/webp";

/** The styles of images. */
const styles = style.create({
    placeholder: (placeholder: string) => ({
        backgroundImage: `url("${placeholder}")`,
        backgroundSize: "cover",
        backgroundPosition: "center",
    }),
});

/** The attributes of an image beside its source, an image element's attributes included. */
interface ImageAttributes extends Omit<
    JSX.ImgHTMLAttributes<HTMLImageElement>,
    "src" | "srcset" | "width" | "height" | "alt" | "style"
> {
    /** The text standing in for the image, empty for a decorative one. */
    readonly alt: string;
    /** Show the asset's blurred rendition until the image loads, or nothing. */
    readonly placeholder?: "blur" | "empty";
    /** The StyleX styles applied last. */
    readonly style?: style.Styles;
}

/** The properties of an image: a build's asset with its own size, or an address with its width and height. */
export type ImageProperties = ImageAttributes &
    (
        | {
              /** The build's asset. */
              readonly src: ImageAsset;
          }
        | {
              /** The image's address. */
              readonly src: string;
              /** The width in pixels. */
              readonly width: number;
              /** The height in pixels. */
              readonly height: number;
          }
    );

/** Show an image in its best single format at the width the layout needs, without shifting the layout. */
export function Image(properties: ImageProperties): JSX.Element {
    return <ImageElement image={properties} choose={preferred} />;
}

/** Show an image in the best format the browser reads, each format at the width the layout needs. */
export function Picture(properties: ImageProperties): JSX.Element {
    // offer each better format an asset has before its original
    const variants = () => (typeof properties.src === "string" ? [] : properties.src.variants);
    const sources = () =>
        PICTURE_TYPES.map((type) => ({
            type,
            srcset: srcsetOf(variants().filter((variant) => variant.type === type)),
        })).filter((source) => source.srcset !== "");

    return (
        <picture data-slot="picture">
            <For each={sources()}>
                {(source) => (
                    <source type={source.type} srcset={source.srcset} sizes={properties.sizes} />
                )}
            </For>
            <ImageElement image={properties} choose={original} />
        </picture>
    );
}

/** Render an image element over the variants a choice keeps, its blurred rendition shown until it loads. */
function ImageElement(properties: {
    readonly image: ImageProperties;
    readonly choose: (asset: ImageAsset) => ImageAsset["variants"];
}): JSX.Element {
    // read the asset, or the address and size given beside it
    const image = properties.image;
    const rest = omit(image, "src", "alt", "placeholder", "style", "onLoad");
    const [isLoaded, setLoaded] = createSignal(false);
    const asset = () => (typeof image.src === "string" ? undefined : image.src);
    const size = () =>
        "width" in image
            ? { width: image.width, height: image.height }
            : { width: asset()?.width, height: asset()?.height };

    // keep the variants the choice offers
    const chosen = () => {
        const found = asset();

        return found === undefined ? [] : properties.choose(found);
    };

    // show the blurred rendition until the image loads
    const placeholder = () => {
        const found = asset()?.placeholder;

        return image.placeholder === "empty" || isLoaded() || found === undefined
            ? undefined
            : styles.placeholder(found);
    };

    return (
        <img
            data-slot="image"
            loading="lazy"
            decoding="async"
            {...rest}
            src={typeof image.src === "string" ? image.src : image.src.src}
            srcset={srcsetOf(chosen()) || undefined}
            alt={image.alt}
            width={size().width}
            height={size().height}
            onLoad={() => setLoaded(true)}
            {...style.attrs(placeholder(), image.style)}
        />
    );
}

/** Keep an asset's variants in the preferred single format, or else its original's. */
function preferred(asset: ImageAsset): ImageAsset["variants"] {
    const found = asset.variants.filter((variant) => variant.type === IMAGE_TYPE);

    return found.length > 0 ? found : original(asset);
}

/** Keep an asset's variants in its original's format. */
function original(asset: ImageAsset): ImageAsset["variants"] {
    return asset.variants.filter((variant) => variant.type === asset.type);
}

/** Write variants as a width-described source set, empty for none. */
function srcsetOf(variants: ImageAsset["variants"]): string {
    return variants.map((variant) => `${variant.src} ${variant.width}w`).join(", ");
}
