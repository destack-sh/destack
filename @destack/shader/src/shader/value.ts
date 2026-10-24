import type { ShaderUniform, ShaderUniforms } from "../mount/mount.ts";
import { ShaderError } from "../error/index.ts";

/** The shorter side an image is drawn at least at, in pixels. */
const MIN_IMAGE_SIDE = 1024;

/** A transparent 1 by 1 image, which an image effect samples until it has an image of its own. */
export const TRANSPARENT_PIXEL =
    "data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==";

/** A CSS color, such as `#3e63dd`, `oklch(0.6 0.2 260)` or a theme token like `color.primary`. */
export type ShaderColor = string;

/** An image a shader samples, by its address or as a loaded element. */
export interface ShaderImage {
    /** The image's address or element. */
    readonly image: string | HTMLImageElement;
}

/** A value an effect passes to its fragment shader. */
export type ShaderValue = ShaderUniform | ShaderColor | readonly ShaderColor[] | ShaderImage;

/** The values an effect passes to its fragment shader, by their GLSL names. */
export type ShaderValues = Readonly<Record<string, ShaderValue>>;

/** The images loading or loaded, by address, so effects sharing an image load it once. */
const images = new Map<string, Promise<HTMLImageElement>>();

/** The colors computed for one theme, by their CSS text. */
export type ColorCache = Map<string, Rgba>;

/** Resolve colors to RGBA vectors as the element computes them, and load images. */
export async function resolveValues(
    values: ShaderValues,
    element: Element,
    colors: ColorCache,
): Promise<ShaderUniforms> {
    // compute only the colors not computed for this theme yet, in one style pass
    const texts = Object.values(values).flatMap((value) =>
        typeof value === "string" ? [value] : isColorList(value) ? value : [],
    );
    const missing = texts.filter((text) => !colors.has(text));
    if (missing.length > 0) {
        for (const [text, color] of colorsOf(missing, element)) {
            colors.set(text, color);
        }
    }
    const entries = await Promise.all(
        Object.entries(values).map(async ([name, value]): Promise<[string, ShaderUniform]> => [
            name,
            await resolveValue(value, colors, element),
        ]),
    );

    return Object.fromEntries(entries);
}

/** Resolve one value: its colors from those computed, its images from the element's page. */
async function resolveValue(
    value: ShaderValue,
    colors: ReadonlyMap<string, Rgba>,
    element: Element,
): Promise<ShaderUniform> {
    // resolve colors and lists of colors
    if (typeof value === "string") {
        return present(colors, value);
    } else if (isColorList(value)) {
        return value.map((entry) => present(colors, entry));
    }
    // load images
    else if (typeof value === "object" && "image" in value) {
        return await loadImage(value.image, element);
    }

    return value;
}

/** Report whether a value is a list of colors, rather than a vector or a list of vectors. */
function isColorList(value: ShaderValue): value is readonly ShaderColor[] {
    return Array.isArray(value) && value.length > 0 && typeof value[0] === "string";
}

/** A color as red, green, blue and alpha from 0 to 1. */
type Rgba = [number, number, number, number];

/** Return a computed color, which every color text has. */
function present(colors: ReadonlyMap<string, Rgba>, text: string): Rgba {
    const color = colors.get(text);
    if (color === undefined) {
        throw new TypeError(`color ${text} is not computed`);
    }

    return color;
}

/** Compute CSS colors as the element draws them, all in one style pass. */
export function colorsOf(values: readonly string[], element: Element): Map<string, Rgba> {
    // refuse text that is no CSS color
    const texts = [...new Set(values)];
    for (const text of texts) {
        if (!CSS.supports("color", text)) {
            throw ShaderError.of("INVALID_COLOR", `${text} is not a CSS color`);
        }
    }

    // let the browser compute every color on probes inside the element, then read them together
    const probes = texts.map((text) => {
        const probe = element.ownerDocument.createElement("span");
        probe.style.color = text;

        return { text, probe };
    });
    element.append(...probes.map(({ probe }) => probe));
    const computed = probes.map(({ text, probe }) => ({
        text,
        color: getComputedStyle(probe).color,
    }));
    for (const { probe } of probes) {
        probe.remove();
    }

    return new Map(computed.map(({ text, color }) => [text, rgbaOf(color, element)]));
}

/** Read a computed color's channels, painting syntax such as oklch() to read its pixel. */
function rgbaOf(computed: string, element: Element): Rgba {
    // read rgb() channels, painting any other color syntax
    const channels = /^rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\)$/u.exec(
        computed,
    );
    if (channels === null) {
        return paintedColor(computed, element.ownerDocument);
    }

    // scale the channels to fractions
    const [, red = "0", green = "0", blue = "0", alpha = "1"] = channels;

    return [Number(red) / 255, Number(green) / 255, Number(blue) / 255, Number(alpha)];
}

/** Paint a CSS color on one pixel and read its sRGB channels. */
function paintedColor(color: string, document: Document): Rgba {
    // paint the color on a one-pixel canvas
    const context = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    if (context === null) {
        throw ShaderError.of(
            "CANVAS_UNSUPPORTED",
            "this browser has no 2D canvas to read colors with",
        );
    }
    context.fillStyle = color;
    context.fillRect(0, 0, 1, 1);

    // read the pixel
    const [red = 0, green = 0, blue = 0, alpha = 0] = context.getImageData(0, 0, 1, 1).data;

    return [red / 255, green / 255, blue / 255, alpha / 255];
}

/** Load an image once per address, at least at the least side, cross-origin images anonymously. */
function loadImage(source: string | HTMLImageElement, element: Element): Promise<HTMLImageElement> {
    // decode an element as it is
    if (typeof source !== "string") {
        return source.decode().then(() => raised(source));
    }

    // reuse a load of the same address
    const loading = images.get(source);
    if (loading !== undefined) {
        return loading;
    }

    // load the address, anonymously from another origin than the element's document
    const image = new Image();
    const page = new URL(element.baseURI);
    if (new URL(source, page).origin !== page.origin) {
        image.crossOrigin = "anonymous";
    }
    image.src = source;
    const loaded = image.decode().then(() => raised(image));
    images.set(source, loaded);

    return loaded;
}

/** Draw a small image at the least side, keeping its aspect ratio. */
function raised(image: HTMLImageElement): HTMLImageElement {
    // keep an image already at least the least side
    if (image.naturalWidth >= MIN_IMAGE_SIDE || image.naturalHeight >= MIN_IMAGE_SIDE) {
        return image;
    }

    // raise the shorter side
    const aspect = image.naturalWidth / image.naturalHeight;
    image.width = Math.round(aspect > 1 ? MIN_IMAGE_SIDE * aspect : MIN_IMAGE_SIDE);
    image.height = Math.round(aspect > 1 ? MIN_IMAGE_SIDE : MIN_IMAGE_SIDE / aspect);

    return image;
}
