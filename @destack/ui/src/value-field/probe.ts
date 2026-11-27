import { Duration, type JsonValue } from "@destack/schema";
import {
    CubicBezier,
    Dimension,
    FontFamily,
    FontWeight,
    Reference,
    type Shadow,
    ShadowStack,
    type TokenType,
    Typography,
} from "@destack/style";

/** The CSS property a browser computes each type of token's value in. */
const PROPERTIES = {
    color: "color",
    dimension: "width",
    fontFamily: "font-family",
    fontWeight: "font-weight",
    duration: "transition-duration",
    cubicBezier: "transition-timing-function",
    shadow: "box-shadow",
    typography: "font-size",
} satisfies Record<TokenType, string>;

/** The milliseconds of a second, as CSS computes durations in seconds. */
const MILLISECONDS_PER_SECOND = 1000;

/** The digits a line height keeps, as tokens write it. */
const LINE_HEIGHT_PRECISION = 10_000;

/** Reads what tokens draw where an element is, through the computed styles of a hidden element. */
export class TokenProbe {
    /** The element whose theme the tokens are read in. */
    readonly #anchor: Element;

    /** Read tokens where an element is. */
    constructor(anchor: Element) {
        this.#anchor = anchor;
    }

    /** Read the CSS a browser computes for tokens, such as `16px`, in one pass of styles. */
    css(type: TokenType, references: readonly Reference[]): string[] {
        return this.#within(type, references, (computed) =>
            computed.getPropertyValue(PROPERTIES[type]),
        );
    }

    /** Read the color a token draws now. */
    color(reference: Reference): string {
        return this.#value("color", reference, (computed) => {
            const css = computed.getPropertyValue(PROPERTIES.color);

            return css.length > 0 ? css : undefined;
        });
    }

    /** Read the length a token draws now. */
    dimension(reference: Reference): Dimension {
        return this.#value("dimension", reference, (computed) => Dimension.read(computed.width));
    }

    /** Read the fonts a token draws now. */
    fontFamily(reference: Reference): FontFamily {
        return this.#value("fontFamily", reference, (computed) =>
            FontFamily.read(computed.fontFamily),
        );
    }

    /** Read the weight a token draws now. */
    fontWeight(reference: Reference): FontWeight {
        return this.#value(
            "fontWeight",
            reference,
            (computed) => FontWeight.safeParse(Number(computed.fontWeight)).data,
        );
    }

    /** Read the duration a token draws now, as the person's motion setting scales it. */
    duration(reference: Reference): Duration {
        return this.#value("duration", reference, (computed) => {
            const seconds = Number.parseFloat(computed.transitionDuration);

            return Duration.of(Math.round(seconds * MILLISECONDS_PER_SECOND), "milliseconds");
        });
    }

    /** Read the easing a token draws now. */
    cubicBezier(reference: Reference): CubicBezier {
        return this.#value("cubicBezier", reference, (computed) =>
            CubicBezier.read(computed.transitionTimingFunction),
        );
    }

    /** Read the shadows a token draws now. */
    shadow(reference: Reference): Shadow[] {
        return this.#value("shadow", reference, (computed) => ShadowStack.read(computed.boxShadow));
    }

    /** Read how a typography token sets text now, its line height a multiple of its size. */
    typography(reference: Reference): Typography {
        return this.#value("typography", reference, (computed) => {
            // read each member, letter spacing 0 where CSS calls it normal
            const fontSize = Dimension.read(computed.fontSize);
            const spacing = computed.letterSpacing === "normal" ? "0px" : computed.letterSpacing;
            const letterSpacing = Dimension.read(spacing);
            const size = Number.parseFloat(computed.fontSize);
            const ratio = Number.parseFloat(computed.lineHeight) / size;
            const lineHeight = Math.round(ratio * LINE_HEIGHT_PRECISION) / LINE_HEIGHT_PRECISION;

            return Typography.safeParse({
                fontFamily: FontFamily.read(computed.fontFamily),
                fontSize,
                fontWeight: Number(computed.fontWeight),
                letterSpacing,
                lineHeight,
            }).data;
        });
    }

    /** Read the value a token of a type draws now, as JSON. */
    json(type: TokenType, reference: Reference): JsonValue {
        // read through the type's own reader
        if (type === "color") {
            return this.color(reference);
        } else if (type === "dimension") {
            return this.dimension(reference);
        } else if (type === "fontFamily") {
            return this.fontFamily(reference);
        } else if (type === "fontWeight") {
            return this.fontWeight(reference);
        } else if (type === "duration") {
            return this.duration(reference);
        } else if (type === "cubicBezier") {
            return this.cubicBezier(reference);
        } else if (type === "shadow") {
            return this.shadow(reference);
        } else {
            return this.typography(reference);
        }
    }

    /** Read one token's value from its computed styles, refusing what fails to read. */
    #value<Value>(
        type: TokenType,
        reference: Reference,
        reader: (computed: CSSStyleDeclaration) => Value | undefined,
    ): Value {
        const [value] = this.#within(type, [reference], reader);
        if (value === undefined) {
            throw new TypeError(`token '${reference}' draws no ${type}`);
        }

        return value;
    }

    /** Set each token's property on a hidden element, and read their computed styles meanwhile. */
    #within<Read>(
        type: TokenType,
        references: readonly Reference[],
        reader: (computed: CSSStyleDeclaration) => Read,
    ): Read[] {
        // draw one element per token out of the flow, so they move nothing
        const elements = references.map((reference) => {
            // hide the element out of the flow
            const element = document.createElement("span");
            element.style.position = "absolute";
            element.style.visibility = "hidden";

            // set the type's property, or each member of a typography token
            if (type === "typography") {
                Object.assign(element.style, Typography.css(reference));
            } else {
                element.style.setProperty(PROPERTIES[type], Reference.css(reference));
            }

            return element;
        });

        // read them all after one computation of styles, then remove them
        this.#anchor.append(...elements);
        const values = elements.map((element) => reader(getComputedStyle(element)));
        for (const element of elements) {
            element.remove();
        }

        return values;
    }
}
