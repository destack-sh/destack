import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    flush,
    untrack,
    useContext,
} from "@destack/view";

/** The smallest side of a crop, in the image's pixels. */
const LEAST = 16;

/** The most a person zooms in, as in react-easy-crop. */
const MOST_ZOOM = 3;

/** The share of the view around what it shows, where the handles of a frame find room. */
const FRAME_MARGIN = 0.06;

/** An image as it is stored, mirrored no way. */
const UPRIGHT: CropFlip = { horizontal: false, vertical: false };

/** A width and a height, in pixels. */
export interface CropSize {
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
}

/** A part of an image as it stands, in its own pixels. */
export interface CropArea extends CropSize {
    /** How far it is from the left side. */
    readonly x: number;
    /** How far it is from the top. */
    readonly y: number;
}

/** A quarter of a turn to the right, or several. */
export type CropRotation = 0 | 90 | 180 | 270;

/** The ways an image is mirrored. */
export interface CropFlip {
    /** Whether it is mirrored from left to right. */
    readonly horizontal: boolean;
    /** Whether it is mirrored from top to bottom. */
    readonly vertical: boolean;
}

/** The side or corner of a crop a handle moves. */
export type CropPosition =
    | "top"
    | "top-right"
    | "right"
    | "bottom-right"
    | "bottom"
    | "bottom-left"
    | "left"
    | "top-left";

/** How a cropper crops, and what it tells its owner. */
export interface CropperOptions {
    /** The part of the image, as it stands turned and mirrored, the most of it by default. */
    readonly value?: CropArea | undefined;
    /** The part at first while uncontrolled, the most of the image by default. */
    readonly defaultValue?: CropArea;
    /** Handle the part as it changes. */
    readonly onValueChange?: (value: CropArea) => void;
    /** Handle the part a person settles on. */
    readonly onValueCommitted?: (value: CropArea) => void;
    /** How much wider than tall the part is, any by default. */
    readonly aspectRatio?: number;
    /** Whether the frame stays still and the image moves below it, as for a profile picture. */
    readonly fixedCropArea?: boolean;
    /** How the image is turned. */
    readonly rotation?: CropRotation;
    /** How the image is turned at first while uncontrolled, not at all by default. */
    readonly defaultRotation?: CropRotation;
    /** Handle the image turning. */
    readonly onRotationChange?: (rotation: CropRotation) => void;
    /** How the image is mirrored. */
    readonly flip?: CropFlip;
    /** How the image is mirrored at first while uncontrolled, not at all by default. */
    readonly defaultFlip?: CropFlip;
    /** Handle the image being mirrored. */
    readonly onFlipChange?: (flip: CropFlip) => void;
    /** Handle how far the image is zoomed in. */
    readonly onZoomChange?: (zoom: number) => void;
    /** The least width of the part, in pixels of the image, 16 by default. */
    readonly minWidth?: number;
    /** The least height of the part, in pixels of the image, 16 by default. */
    readonly minHeight?: number;
    /** The most a person zooms in, three times by default. */
    readonly maxZoom?: number;
    /** Whether it ignores the person. */
    readonly disabled?: boolean;
}

/** How a picture is written. */
export interface CropBlobOptions {
    /** The kind of picture, PNG by default. */
    readonly type?: string;
    /** How much of it a lossy kind keeps, from 0 to 1. */
    readonly quality?: number;
    /** The width of the picture, the part's own by default. */
    readonly width?: number;
}

/** The part of an image a person crops, whichever way the image is turned and shown. */
export class CropperControl {
    /** The size of the image as it is stored, undefined until it loads. */
    readonly image: Accessor<CropSize | undefined>;
    /** How the image is turned. */
    readonly rotation: Accessor<CropRotation>;
    /** How the image is mirrored. */
    readonly mirror: Accessor<CropFlip>;
    /** The size of the view. */
    readonly view: Accessor<CropSize>;
    /** Whether a person moves the part or the image. */
    readonly isMoving: Accessor<boolean>;
    /** The options. */
    readonly options: CropperOptions;
    /** The element that shows the image. */
    element: HTMLImageElement | undefined;
    /** Read the part the owner or a person set. */
    readonly #value: Accessor<CropArea | undefined>;
    /** Set the part. */
    readonly #setValue: (value: CropArea | undefined) => void;
    /** Set the size of the image. */
    readonly #setImage: (image: CropSize) => void;
    /** Set how the image is turned. */
    readonly #setRotation: (rotation: CropRotation) => void;
    /** Set how the image is mirrored. */
    readonly #setMirror: (flip: CropFlip) => void;
    /** Set the size of the view. */
    readonly #setView: (view: CropSize) => void;
    /** Set whether a person moves something. */
    readonly #setMoving: (isMoving: boolean) => void;

    /** Follow how a cropper crops. */
    constructor(options: CropperOptions) {
        // follow the controlled part or its own
        const [value, setValue] = createControllableSignal<CropArea | undefined>({
            isControlled: () => "value" in options,
            value: () => options.value,
            defaultValue: untrack(() => options.defaultValue),
            onChange: (next) => {
                if (next !== undefined) {
                    options.onValueChange?.(next);
                }
            },
        });
        const owned = { ownedWrite: true };
        const [image, setImage] = createSignal<CropSize | undefined>(undefined, owned);
        const [rotation, setRotation] = createControllableSignal<CropRotation>({
            isControlled: () => "rotation" in options,
            value: () => options.rotation ?? 0,
            defaultValue: untrack(() => options.defaultRotation) ?? 0,
            onChange: (next) => options.onRotationChange?.(next),
        });
        const [mirror, setMirror] = createControllableSignal<CropFlip>({
            isControlled: () => "flip" in options,
            value: () => options.flip ?? UPRIGHT,
            defaultValue: untrack(() => options.defaultFlip) ?? UPRIGHT,
            onChange: (next) => options.onFlipChange?.(next),
        });
        const [view, setView] = createSignal<CropSize>({ width: 0, height: 0 }, owned);
        const [isMoving, setMoving] = createSignal(false, owned);
        this.options = options;
        this.image = image;
        this.rotation = rotation;
        this.mirror = mirror;
        this.view = view;
        this.isMoving = isMoving;
        this.#value = value;
        this.#setValue = setValue;
        this.#setImage = (next) => void setImage(next);
        this.#setRotation = setRotation;
        this.#setMirror = setMirror;
        this.#setView = (next) => void setView(next);
        this.#setMoving = (next) => void setMoving(next);
    }

    /** Take the image once it loads. */
    load(image: HTMLImageElement): void {
        this.element = image;
        this.#setImage({ width: image.naturalWidth, height: image.naturalHeight });
    }

    /** Take the size of the view. */
    resize(view: CropSize): void {
        this.#setView(view);
    }

    /** Say whether a person moves something, and that they settled once they do not. */
    setMoving(isMoving: boolean): void {
        this.#setMoving(isMoving);
        if (!isMoving) {
            this.commit();
        }
    }

    /** Tell the owner the part a person settled on. */
    commit(): void {
        // read it as just set
        flush();
        this.options.onValueCommitted?.(untrack(() => this.area()));
    }

    /** Read the size of the image as it stands turned. */
    turned(): CropSize {
        const { width, height } = this.image() ?? { width: 1, height: 1 };

        return this.rotation() % 180 === 0 ? { width, height } : { width: height, height: width };
    }

    /** Read the part, the most of the image where none is set. */
    area(): CropArea {
        const value = this.#value();

        return value === undefined ? this.#most() : this.#fit(value);
    }

    /** Read how far the image is zoomed in, once where the part is the most of it. */
    zoom(): number {
        return this.#most().width / this.area().width;
    }

    /** Read the most a person zooms in. */
    maxZoom(): number {
        // keep the least size too
        const most = this.#most();
        const { minWidth = LEAST, minHeight = LEAST, maxZoom = MOST_ZOOM } = this.options;
        const largest = Math.min(most.width / minWidth, most.height / minHeight);

        return Math.max(1, Math.min(maxZoom, largest));
    }

    /** Read the pixels of the screen one of the image takes. */
    scale(): number {
        // fit the part in the frame that stays, or the whole image in the view
        const { width, height } = this.turned();
        const view = this.view();

        return this.options.fixedCropArea === true
            ? this.frame().width / this.area().width
            : (1 - 2 * FRAME_MARGIN) * Math.min(view.width / width, view.height / height);
    }

    /** Read the place of the frame in the view. */
    frame(): CropArea {
        // follow the part over the image that stays
        const view = this.view();
        if (this.options.fixedCropArea !== true) {
            const area = this.area();
            const scale = this.scale();
            const origin = this.origin();

            return {
                x: origin.x + area.x * scale,
                y: origin.y + area.y * scale,
                width: area.width * scale,
                height: area.height * scale,
            };
        }

        // stay in the middle, as large as the view allows
        const ratio = this.#ratio();
        const room = 1 - 2 * FRAME_MARGIN;
        const width = Math.min(view.width * room, view.height * room * ratio);
        const height = width / ratio;

        return { x: (view.width - width) / 2, y: (view.height - height) / 2, width, height };
    }

    /** Read the place of the image's corner in the view. */
    origin(): { readonly x: number; readonly y: number } {
        // put the part below the frame that stays, or the image in the middle
        const scale = this.scale();
        if (this.options.fixedCropArea === true) {
            const frame = this.frame();
            const area = this.area();

            return { x: frame.x - area.x * scale, y: frame.y - area.y * scale };
        }
        const view = this.view();
        const { width, height } = this.turned();

        return { x: (view.width - width * scale) / 2, y: (view.height - height * scale) / 2 };
    }

    /** Move the part by pixels of the image. */
    moveBy(x: number, y: number, from: CropArea = this.area()): void {
        this.#set({ ...from, x: from.x + x, y: from.y + y });
    }

    /** Zoom the part about a point of the image, its middle by default. */
    zoomTo(zoom: number, point?: { readonly x: number; readonly y: number }): void {
        // keep the point where it is in the part
        const from = this.area();
        const most = this.#most();
        const bounded = Math.min(Math.max(zoom, 1), this.maxZoom());
        this.options.onZoomChange?.(bounded);
        const width = most.width / bounded;
        const height = most.height / bounded;
        const at = point ?? { x: from.x + from.width / 2, y: from.y + from.height / 2 };
        const across = (at.x - from.x) / from.width;
        const down = (at.y - from.y) / from.height;
        this.#set({ x: at.x - across * width, y: at.y - down * height, width, height });
    }

    /** Move a side or a corner of the part by pixels of the image. */
    resizeBy(position: CropPosition, x: number, y: number, from: CropArea = this.area()): void {
        // move each side it has, as far as the image and the least size let it
        const turned = this.turned();
        const isLeft = position.includes("left");
        const isRight = position.includes("right");
        const isTop = position.includes("top");
        const isBottom = position.includes("bottom");
        const right = from.x + from.width;
        const bottom = from.y + from.height;
        const narrowest = Math.min(this.options.minWidth ?? LEAST, from.width);
        const shortest = Math.min(this.options.minHeight ?? LEAST, from.height);
        const left = isLeft ? within(from.x + x, 0, right - narrowest) : from.x;
        const top = isTop ? within(from.y + y, 0, bottom - shortest) : from.y;
        const width =
            (isRight ? within(right + x, from.x + narrowest, turned.width) : right) - left;
        const height =
            (isBottom ? within(bottom + y, from.y + shortest, turned.height) : bottom) - top;
        const ratio = this.options.aspectRatio;
        if (ratio === undefined) {
            this.#set({ x: left, y: top, width, height });

            return;
        }

        // keep the ratio, from the side opposite to the one that moves
        const isTall = isTop || isBottom;
        const isWide = isLeft || isRight;
        const asked =
            isWide && isTall ? Math.max(width, height * ratio) : isWide ? width : height * ratio;
        const roomX = isLeft ? right : isRight ? turned.width - from.x : turned.width;
        const roomY = isTop ? bottom : isBottom ? turned.height - from.y : turned.height;
        const smallest = Math.max(narrowest, shortest * ratio);
        const fitted = Math.min(Math.max(asked, smallest), roomX, roomY * ratio);
        const size = { width: fitted, height: fitted / ratio };
        this.#set({
            x: isLeft
                ? right - size.width
                : isWide
                  ? from.x
                  : from.x + (from.width - size.width) / 2,
            y: isTop
                ? bottom - size.height
                : isTall
                  ? from.y
                  : from.y + (from.height - size.height) / 2,
            ...size,
        });
    }

    /** Turn the image a quarter to the right, or to the left, the same part in view. */
    rotate(direction: 1 | -1 = 1): void {
        // turn the part with the image
        const from = this.area();
        const { width, height } = this.turned();

        // turn a mirrored image the other way, as it is mirrored after it is turned
        const { horizontal, vertical } = this.mirror();
        const isRight = (direction === 1) === (horizontal === vertical);
        const next = (this.rotation() + (isRight ? 90 : 270)) % 360;
        const rotation = ([0, 90, 180, 270] as const).find((each) => each === next) ?? 0;
        const turned =
            direction === 1
                ? { x: height - from.y - from.height, y: from.x }
                : { x: from.y, y: width - from.x - from.width };
        this.#setRotation(rotation);
        const area = { ...turned, width: from.height, height: from.width };
        this.#setValue(this.#fitIn(area, { width: height, height: width }));
        this.commit();
    }

    /** Mirror the image along an axis, from left to right by default, the same part in view. */
    flip(axis: keyof CropFlip = "horizontal"): void {
        // mirror the part with the image
        const from = this.area();
        const turned = this.turned();
        const mirror = this.mirror();
        this.#setMirror({ ...mirror, [axis]: !mirror[axis] });
        this.#set(
            axis === "horizontal"
                ? { ...from, x: turned.width - from.x - from.width }
                : { ...from, y: turned.height - from.y - from.height },
        );
        this.commit();
    }

    /** Take the most of the image as it is stored. */
    reset(): void {
        // turn and mirror it back
        this.#setRotation(0);
        this.#setMirror(UPRIGHT);
        this.#setValue(this.#mostOf(this.image() ?? { width: 1, height: 1 }));
        this.commit();
    }

    /** Write the part as a picture. */
    async toBlob({ type = "image/png", quality, width }: CropBlobOptions = {}): Promise<Blob> {
        // draw the image as it stands, only the part on the canvas
        const image = this.element;
        const size = this.image();
        if (image === undefined || size === undefined) {
            throw new TypeError("the image to crop is not there yet");
        }
        const area = this.area();
        const scale = (width ?? area.width) / area.width;
        const canvas = image.ownerDocument.createElement("canvas");
        canvas.width = Math.max(1, Math.round(area.width * scale));
        canvas.height = Math.max(1, Math.round(area.height * scale));
        const context = canvas.getContext("2d");
        if (context === null) {
            throw new TypeError("the browser draws no pictures");
        }
        const turned = this.turned();
        context.scale(scale, scale);
        context.translate(turned.width / 2 - area.x, turned.height / 2 - area.y);
        const { horizontal, vertical } = this.mirror();
        context.scale(horizontal ? -1 : 1, vertical ? -1 : 1);
        context.rotate((this.rotation() * Math.PI) / 180);
        context.drawImage(image, -size.width / 2, -size.height / 2);

        // write it, refusing a picture the browser may not read
        return new Promise((resolve, reject) => {
            canvas.toBlob(
                (blob) => {
                    if (blob === null) {
                        reject(new TypeError("the browser wrote no picture"));
                    } else {
                        resolve(blob);
                    }
                },
                type,
                quality,
            );
        });
    }

    /** Set the part, within the image. */
    #set(area: CropArea): void {
        if (this.options.disabled !== true) {
            this.#setValue(this.#fit(area));
        }
    }

    /** Read how much wider than tall the part is. */
    #ratio(): number {
        const { width, height } = this.#value() ?? this.turned();

        return this.options.aspectRatio ?? width / height;
    }

    /** Read the largest part in the middle of the image. */
    #most(): CropArea {
        return this.#mostOf(this.turned());
    }

    /** Read the largest part in the middle of an image. */
    #mostOf(size: CropSize): CropArea {
        // take the owner's ratio, else the image's
        const ratio = this.options.aspectRatio ?? size.width / size.height;
        const width = Math.min(size.width, size.height * ratio);
        const height = width / ratio;

        return { x: (size.width - width) / 2, y: (size.height - height) / 2, width, height };
    }

    /** Keep a part within the image as it stands. */
    #fit(area: CropArea): CropArea {
        return this.#fitIn(area, this.turned());
    }

    /** Keep a part within an image, its ratio kept about its middle. */
    #fitIn(area: CropArea, size: CropSize): CropArea {
        // take the ratio, as large as fits the part's longer side
        const ratio = this.options.aspectRatio;
        const wide = Math.min(area.width, size.width);
        const tall = Math.min(area.height, size.height);
        const width =
            ratio === undefined
                ? wide
                : Math.min(Math.max(wide, tall * ratio), size.width, size.height * ratio);
        const height = ratio === undefined ? tall : width / ratio;

        // push it back inside
        const x = area.x + (area.width - width) / 2;
        const y = area.y + (area.height - height) / 2;

        return {
            x: Math.min(Math.max(x, 0), size.width - width),
            y: Math.min(Math.max(y, 0), size.height - height),
            width,
            height,
        };
    }
}

/** Keep a value between two others. */
function within(value: number, low: number, high: number): number {
    return Math.min(Math.max(value, low), high);
}

/** The nearest cropper, null outside one. */
export const CropperContext = createContext<CropperControl | null>(null);

/** Read the nearest cropper, refusing a part outside one. */
export function useCropper(): CropperControl {
    const control = useContext(CropperContext);
    if (control === null) {
        throw new TypeError("a cropper part needs a cropper around it");
    }

    return control;
}
