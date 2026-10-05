import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import type { JSX } from "@solidjs/web";
import { Image, Picture } from "./image.tsx";
import type { ImageAsset } from "./asset.ts";

/** A hero image the build turned into two widths in three formats, with a blurred rendition. */
const hero: ImageAsset = {
    src: "/_assets/hero.png",
    width: 1600,
    height: 900,
    type: "image/png",
    variants: ["image/avif", "image/webp", "image/png"].flatMap((type) =>
        [640, 1280].map((width) => ({
            src: `/_assets/hero-${width}.${type.slice(6)}`,
            width,
            type,
        })),
    ),
    placeholder: "data:image/webp;base64,UklGRg==",
};

/** Render an element into the document and remove it after the test. */
function mount(element: () => JSX.Element): HTMLElement {
    const host = document.createElement("main");
    document.body.append(host);
    const dispose = render(element, host);
    onTestFinished(() => {
        dispose();
        host.remove();
    });

    return host;
}

test("show an image in its preferred format at its intrinsic size, lazily", () => {
    // render the asset as a single image
    const image = mount(() => <Image src={hero} alt="The lattice" sizes="100vw" />).querySelector(
        "img",
    );

    // offer the WebP widths, keep the intrinsic size, and load lazily over the blurred rendition
    expect({
        src: image?.getAttribute("src"),
        srcset: image?.getAttribute("srcset"),
        sizes: image?.getAttribute("sizes"),
        width: image?.getAttribute("width"),
        height: image?.getAttribute("height"),
        loading: image?.getAttribute("loading"),
        alt: image?.getAttribute("alt"),
        isBlurred: image?.getAttribute("class") !== null,
    }).toEqual({
        src: "/_assets/hero.png",
        srcset: "/_assets/hero-640.webp 640w, /_assets/hero-1280.webp 1280w",
        sizes: "100vw",
        width: "1600",
        height: "900",
        loading: "lazy",
        alt: "The lattice",
        isBlurred: true,
    });
});

test("offer a picture's better formats before its original", () => {
    // render the asset as a picture
    const picture = mount(() => <Picture src={hero} alt="The lattice" loading="eager" />);

    // list AVIF and WebP sources, and fall back to the original's widths
    expect({
        sources: [...picture.querySelectorAll("source")].map((source) => [
            source.getAttribute("type"),
            source.getAttribute("srcset"),
        ]),
        fallback: picture.querySelector("img")?.getAttribute("srcset"),
        loading: picture.querySelector("img")?.getAttribute("loading"),
    }).toEqual({
        sources: [
            ["image/avif", "/_assets/hero-640.avif 640w, /_assets/hero-1280.avif 1280w"],
            ["image/webp", "/_assets/hero-640.webp 640w, /_assets/hero-1280.webp 1280w"],
        ],
        fallback: "/_assets/hero-640.png 640w, /_assets/hero-1280.png 1280w",
        loading: "eager",
    });
});

test("show an image given by its address at its given dimensions", () => {
    // render an address with its size and no variants
    const image = mount(() => (
        <Image src="/logo.svg" alt="" width={32} height={32} />
    )).querySelector("img");
    expect([image?.getAttribute("width"), image?.getAttribute("srcset")]).toEqual(["32", null]);
});
