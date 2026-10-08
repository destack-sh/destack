import type { ShaderUniform, ShaderUniforms } from "../mount/mount.ts";

/** The shorter side an image is drawn at least at, in pixels, so vector images rasterize sharply. */
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

/** A value an effect passes to its fragment shader: a uniform, a color or list of colors, or an image. */
export type ShaderValue = ShaderUniform | ShaderColor | readonly ShaderColor[] | ShaderImage;

/** The values an effect passes to its fragment shader, by their GLSL names. */
export type ShaderValues = Readonly<Record<string, ShaderValue>>;

/** The images loading or loaded, by address, so effects sharing an image load it once. */
const images = new Map<string, Promise<HTMLImageElement>>();

/** Resolve colors to RGBA vectors as the element computes them and load images, returning the uniforms. */
export async function resolveValues(
    values: ShaderValues,
    element: Element,
): Promise<ShaderUniforms> {
    const entries = await Promise.all(
        Object.entries(values).map(async ([name, value]): Promise<[string, ShaderUniform]> => [
            name,
            await resolveValue(value, element),
        ]),
    );

    return Object.fromEntries(entries);
}

/** Resolve one value: a color to RGBA, a list of colors to a list of RGBA, an image to a loaded element. */
async function resolveValue(value: ShaderValue, element: Element): Promise<ShaderUniform> {
    // resolve colors and lists of colors
    if (typeof value === "string") {
        return colorOf(value, element);
    } else if (isColorList(value)) {
        return value.map((entry) => colorOf(entry, element));
    }
    // load images
    else if (typeof value === "object" && "image" in value) {
        return await loadImage(value.image);
    }

    return value;
}

/** Report whether a value is a list of colors, rather than a vector or a list of vectors. */
function isColorList(value: ShaderValue): value is readonly ShaderColor[] {
    return Array.isArray(value) && value.length > 0 && typeof value[0] === "string";
}

/** Compute a CSS color as the element draws it, as red, green, blue and alpha from 0 to 1. */
export function colorOf(value: string, element: Element): [number, number, number, number] {
    // refuse text that is no CSS color
    if (!CSS.supports("color", value)) {
        throw new TypeError(`${value} is not a CSS color`);
    }

    // let the browser compute the color on a probe inside the element
    const probe = element.ownerDocument.createElement("span");
    probe.style.color = value;
    element.append(probe);
    const computed = getComputedStyle(probe).color;
    probe.remove();

    // read rgb() channels, and paint any other syntax such as oklch() to read its pixel
    const channels = /^rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\)$/u.exec(
        computed,
    );
    if (channels === null) {
        return paintedColor(computed, element.ownerDocument);
    }
    const [, red = "0", green = "0", blue = "0", alpha = "1"] = channels;

    return [Number(red) / 255, Number(green) / 255, Number(blue) / 255, Number(alpha)];
}

/** Paint a CSS color on one pixel and read its sRGB channels. */
function paintedColor(color: string, document: Document): [number, number, number, number] {
    // paint the color on a one-pixel canvas
    const context = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    if (context === null) {
        throw new TypeError("this browser draws no 2D canvas to read colors with");
    }
    context.fillStyle = color;
    context.fillRect(0, 0, 1, 1);

    // read the pixel
    const [red = 0, green = 0, blue = 0, alpha = 0] = context.getImageData(0, 0, 1, 1).data;

    return [red / 255, green / 255, blue / 255, alpha / 255];
}

/** Load an image once per address, at least at the least side, cross-origin images anonymously. */
function loadImage(source: string | HTMLImageElement): Promise<HTMLImageElement> {
    // decode an element as it is
    if (typeof source !== "string") {
        return source.decode().then(() => raised(source));
    }

    // reuse a load of the same address
    const loading = images.get(source);
    if (loading !== undefined) {
        return loading;
    }

    // load the address
    const image = new Image();
    if (new URL(source, location.href).origin !== location.origin) {
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
