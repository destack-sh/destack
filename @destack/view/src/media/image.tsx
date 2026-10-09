import * as style from "@destack/style";
import type { JSX } from "@solidjs/web";
import { For } from "../solid/flow.ts";
import { createSignal, omit } from "../solid/reactive.ts";
import type { ImageFormat, ImageMetadata } from "./asset.ts";

/** The formats a picture offers before its original, best first. */
const PICTURE_FORMATS: readonly ImageFormat[] = ["avif", "webp"];

/** The format an image prefers for its single source. */
const IMAGE_FORMAT: ImageFormat = "webp";

/** The styles of images: a picture and its sources laying the image out as a bare image would, and a blurred placeholder. */
const styles = style.create({
    picture: {
        display: "contents",
    },
    source: {
        display: "none",
    },
    placeholder: (placeholder: string) => ({
        backgroundImage: `url("${placeholder}")`,
        backgroundSize: "cover",
        backgroundPosition: "center",
    }),
});

/** The attributes of an image beside its source, an image element's attributes included. */
interface ImageAttributes extends Omit<
    JSX.ImgHTMLAttributes<HTMLImageElement>,
    "src" | "srcset" | "width" | "height" | "alt" | "style" | "onLoad"
> {
    /** The text standing in for the image, empty for a decorative one. */
    readonly alt: string;
    /** Show the image's blurred rendition until it loads, or nothing. */
    readonly placeholder?: "blur" | "empty";
    /** Call back once the image loads. */
    readonly onLoad?: (event: Event) => void;
    /** The StyleX styles applied last. */
    readonly xstyle?: style.Styles;
    /** The inline style applied last. */
    readonly style?: JSX.CSSProperties | string;
}

/** The properties of an image: a build's image with its own size, or an address with its width and height. */
export type ImageProperties = ImageAttributes &
    (
        | {
              /** The build's image. */
              readonly src: ImageMetadata;
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
    // offer each better format the image has before its original
    const files = () => (typeof properties.src === "string" ? [] : properties.src.srcSet);
    const sources = () =>
        PICTURE_FORMATS.map((format) => ({
            type: `image/${format}`,
            srcset: srcsetOf(files().filter((file) => file.format === format)),
        })).filter((source) => source.srcset !== "");

    return (
        <picture data-slot="picture" {...style.attrs(styles.picture)}>
            <For each={sources()}>
                {(source) => (
                    <source
                        type={source.type}
                        srcset={source.srcset}
                        sizes={properties.sizes}
                        {...style.attrs(styles.source)}
                    />
                )}
            </For>
            <ImageElement image={properties} choose={original} />
        </picture>
    );
}

/** Render an image element over the files a choice keeps, its blurred rendition shown until it loads. */
function ImageElement(properties: {
    readonly image: ImageProperties;
    readonly choose: (image: ImageMetadata) => ImageMetadata["srcSet"];
}): JSX.Element {
    // read the build's image, or the address and size given beside it
    const image = properties.image;
    const rest = omit(image, "src", "alt", "placeholder", "xstyle", "style", "onLoad");
    const [isLoaded, setLoaded] = createSignal(false);
    const metadata = () => (typeof image.src === "string" ? undefined : image.src);
    const size = () =>
        "width" in image
            ? { width: image.width, height: image.height }
            : { width: metadata()?.width, height: metadata()?.height };

    // keep the files the choice offers
    const chosen = () => {
        const found = metadata();

        return found === undefined ? [] : properties.choose(found);
    };

    // show the blurred rendition until the image loads
    const placeholder = () => {
        const found = metadata()?.blurDataURL;

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
            onLoad={(event) => {
                setLoaded(true);
                image.onLoad?.(event);
            }}
            {...style.attributes([placeholder(), image.xstyle], image.style)}
        />
    );
}

/** Keep an image's files in the preferred single format, or else its original's. */
function preferred(image: ImageMetadata): ImageMetadata["srcSet"] {
    const found = image.srcSet.filter((file) => file.format === IMAGE_FORMAT);

    return found.length > 0 ? found : original(image);
}

/** Keep an image's files in its original's format. */
function original(image: ImageMetadata): ImageMetadata["srcSet"] {
    return image.srcSet.filter((file) => file.format === image.format);
}

/** Write files as a width-described source set, empty for none. */
function srcsetOf(files: ImageMetadata["srcSet"]): string {
    return files.map((file) => `${file.src} ${file.width}w`).join(", ");
}
