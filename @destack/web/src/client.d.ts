/** A PNG image the web build turns into an asset with its variants. */
declare module "*.png" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A JPG image the web build turns into an asset with its variants. */
declare module "*.jpg" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A JPEG image the web build turns into an asset with its variants. */
declare module "*.jpeg" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A WEBP image the web build turns into an asset with its variants. */
declare module "*.webp" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A AVIF image the web build turns into an asset with its variants. */
declare module "*.avif" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A GIF image the web build turns into an asset with its variants. */
declare module "*.gif" {
    /** The image. */
    const image: import("@destack/view/media").ImageAsset;
    export default image;
}

/** A font file the web build reads with its metrics. */
declare module "*?font" {
    /** The font file. */
    const font: import("./build/font.ts").FontFile;
    export default font;
}
