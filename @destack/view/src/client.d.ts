/// <reference types="vite/client" />

/** A PNG image the web build turns into image metadata with its source set. */
declare module "*.png" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** A JPG image the web build turns into image metadata with its source set. */
declare module "*.jpg" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** A JPEG image the web build turns into image metadata with its source set. */
declare module "*.jpeg" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** A WEBP image the web build turns into image metadata with its source set. */
declare module "*.webp" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** An AVIF image the web build turns into image metadata with its source set. */
declare module "*.avif" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** A GIF image the web build turns into image metadata with its source set. */
declare module "*.gif" {
    /** The image. */
    const image: import("@destack/view/media").ImageMetadata;
    export default image;
}

/** A font file the web build reads with its metrics. */
declare module "*?font" {
    /** The font file. */
    const font: import("./build/font.ts").FontFile;
    export default font;
}
