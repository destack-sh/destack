import { schema } from "@destack/schema";
import {
    Color,
    ColorToken,
    CubicBezier,
    Dimension,
    Duration,
    FontFamily,
    FontWeight,
} from "@destack/style";
import tokens from "./tokens.json" with { type: "json" };

/** The prefix of every custom property a theme sets. */
export const VARIABLE_PREFIX = "--destack";

/** A custom property a theme sets. */
export type Variable = `${typeof VARIABLE_PREFIX}-${string}`;

/** The custom property scaling every duration, 0 under reduced motion. */
export const MOTION_SCALE: Variable = `${VARIABLE_PREFIX}-motion-scale`;

/** The `$extensions` namespace of Destack's token metadata, per the W3C Design Tokens format. */
export const EXTENSION = "app.destack";

/** The envelope fraction at which a spring counts as settled: 0.1% of its travel. */
const SPRING_SETTLED = 0.001;

/** The segments of a spring's `linear()` easing. */
const SPRING_SEGMENTS = 32;

/** The members of a typography token, each a custom property. */
const TYPOGRAPHY_MEMBERS = [
    "fontFamily",
    "fontSize",
    "fontWeight",
    "lineHeight",
    "letterSpacing",
    "fontSizeAdjust",
] as const;

/** The custom property a chosen face sizes its lowercase by, read by text in the text face. */
export const FONT_SIZE_ADJUST = "--destack-font-text-adjust";

/** The decimal places of every computed CSS number, below a thousandth of a pixel. */
const PRECISION = 3;

/** A length in pixels, the unit the theme scales. */
const Pixels = Dimension.extend({ unit: schema.literal("px") });

/** A token color in sRGB, the space the theme's palette works in. */
const ColorValue = ColorToken.extend({ colorSpace: schema.literal("srgb") });

/** A group of the token tree before its members are read. */
const Group = schema.jsonObject();

/** A reference to another token by its path, such as `{font.text}`. */
const Alias = schema.string().regex(/^\{[A-Za-z0-9.]+\}$/u);

/** One layer of a shadow. */
const ShadowLayer = schema.object({
    /** The shadow color, as an alias. */
    color: Alias,
    /** The horizontal offset. */
    offsetX: Pixels,
    /** The vertical offset. */
    offsetY: Pixels,
    /** The blur radius. */
    blur: Pixels,
    /** The spread distance. */
    spread: Pixels,
    /** Whether the shadow falls inside the box. */
    inset: schema.boolean().exactOptional(),
});

/** The physical parameters of a damped spring. */
const Spring = schema.object({
    /** The moving mass. */
    mass: schema.number().positive(),
    /** The spring stiffness. */
    stiffness: schema.number().positive(),
    /** The damping coefficient, below critical damping. */
    damping: schema.number().positive(),
});

/** The fields of every token beside its type and value. */
const common = {
    /** What the token is for. */
    $description: schema.string().min(1),
};

/** A design token in the W3C Design Tokens format. */
export const Token = schema.discriminatedUnion("$type", [
    schema.object({
        /** A color. */
        $type: schema.literal("color"),
        /** The light appearance's color. */
        $value: ColorValue,
        /** The dark appearance's color. */
        $extensions: schema.object({ [EXTENSION]: schema.object({ dark: ColorValue }) }),
        ...common,
    }),
    schema.object({
        /** A length. */
        $type: schema.literal("dimension"),
        /** The length at a scale of 1. */
        $value: Pixels,
        ...common,
    }),
    schema.object({
        /** A duration. */
        $type: schema.literal("duration"),
        /** The duration with full motion. */
        $value: Duration.extend({ unit: schema.literal("ms") }),
        ...common,
    }),
    schema.object({
        /** An easing curve. */
        $type: schema.literal("cubicBezier"),
        /** The control points, the portable form of a spring. */
        $value: CubicBezier,
        /** The spring a `linear()` easing samples instead. */
        $extensions: schema
            .object({ [EXTENSION]: schema.object({ spring: Spring }) })
            .exactOptional(),
        ...common,
    }),
    schema.object({
        /** A font stack. */
        $type: schema.literal("fontFamily"),
        /** The family names in order of preference. */
        $value: FontFamily,
        ...common,
    }),
    schema.object({
        /** A font weight. */
        $type: schema.literal("fontWeight"),
        /** The weight from 1 to 1000, as CSS font-weight takes it. */
        $value: FontWeight,
        ...common,
    }),
    schema.object({
        /** A text style. */
        $type: schema.literal("typography"),
        /** The family, size, weight, line height and letter spacing at a text scale of 1. */
        $value: schema.object({
            /** The font stack, as an alias. */
            fontFamily: Alias,
            /** The font size. */
            fontSize: Pixels,
            /** The font weight. */
            fontWeight: FontWeight,
            /** The line height as a multiple of the font size. */
            lineHeight: schema.number().positive(),
            /** The letter spacing. */
            letterSpacing: Pixels,
        }),
        ...common,
    }),
    schema.object({
        /** A box shadow. */
        $type: schema.literal("shadow"),
        /** The layers from top to bottom. */
        $value: schema.array(ShadowLayer).min(1),
        ...common,
    }),
]);
/** A design token in the W3C Design Tokens format. */
export type Token = schema.Infer<typeof Token>;

/** A token at its path in the token tree, such as `["text", "body"]`. */
export class TokenEntry {
    /** The group names and the token name, the family first. */
    readonly path: readonly string[];
    /** The token. */
    readonly token: Token;
    /** The custom property, such as `--destack-color-muted-foreground`. */
    readonly variable: Variable;
    /** The constant key within the family, such as `durationShort`. */
    readonly key: string;
    /** The CSS values computed so far, by the factor that scales their lengths. */
    readonly #values = new Map<number, readonly (readonly [Variable, string])[]>();

    /** Create a token at its path. */
    constructor(path: readonly string[], token: Token) {
        // store the token, and write its custom property and key once
        this.path = path;
        this.token = token;
        this.variable = variable(path);
        this.key = constantKey(path);
    }

    /** List the constant key and custom property of each member. */
    members(): readonly TokenMember[] {
        const paths =
            this.token.$type === "typography"
                ? TYPOGRAPHY_MEMBERS.map((name) => [...this.path, name])
                : [this.path];

        return paths.map((path) => ({ key: constantKey(path), variable: variable(path) }));
    }

    /** Read the custom property of one member of a typography token, such as its letter spacing. */
    member(name: (typeof TYPOGRAPHY_MEMBERS)[number]): Variable {
        return variable([...this.path, name]);
    }

    /** List the token paths this token's aliases name. */
    aliases(): readonly (readonly string[])[] {
        const token = this.token;
        if (token.$type === "typography") {
            return [aliasPath(token.$value.fontFamily)];
        } else if (token.$type === "shadow") {
            return token.$value.map((shadow) => aliasPath(shadow.color));
        }

        return [];
    }

    /** Refuse a color token whose hex forms differ from its channels. */
    requireHex(): void {
        const token = this.token;
        if (token.$type !== "color") {
            return;
        }
        for (const color of [token.$value, token.$extensions[EXTENSION].dark]) {
            const hex = Color.fromToken(color).with("alpha", 1).toString("hex");
            if (color.hex !== undefined && color.hex.toLowerCase() !== hex) {
                throw new RangeError(
                    `token '${this.path.join(".")}' has hex '${color.hex}', but its color is '${hex}'`,
                );
            }
        }
    }

    /** Compute the CSS values of the custom properties, scaling lengths by a factor. */
    values(factor: number): readonly (readonly [Variable, string])[] {
        // reuse the values at a factor, as a token never changes
        const known = this.#values.get(factor);
        if (known !== undefined) {
            return known;
        }
        const values = this.#compute(factor);
        this.#values.set(factor, values);

        return values;
    }

    /** Compute the CSS values of the custom properties at a factor. */
    #compute(factor: number): readonly (readonly [Variable, string])[] {
        const token = this.token;
        switch (token.$type) {
            case "color": {
                const light = Color.fromToken(token.$value).toString("hex");
                const dark = Color.fromToken(token.$extensions[EXTENSION].dark).toString("hex");

                return [[this.variable, `light-dark(${light}, ${dark})`]];
            }
            case "dimension":
                return [[this.variable, length(token.$value.value * factor)]];
            case "duration": {
                const scale = `var(${MOTION_SCALE}, 1)`;

                return [[this.variable, `calc(${token.$value.value}ms * ${scale})`]];
            }
            case "cubicBezier": {
                const spring = token.$extensions?.[EXTENSION].spring;
                const bezier = `cubic-bezier(${token.$value.join(", ")})`;

                return [[this.variable, spring === undefined ? bezier : linear(spring)]];
            }
            case "fontFamily":
                return [[this.variable, FontFamily.css(token.$value)]];
            case "fontWeight":
                return [[this.variable, String(token.$value)]];
            case "typography": {
                const style = token.$value;
                const member = (name: string) => variable([...this.path, name]);

                return [
                    [member("fontFamily"), `var(${variable(aliasPath(style.fontFamily))})`],
                    [member("fontSize"), length(style.fontSize.value * factor)],
                    [member("fontWeight"), String(style.fontWeight)],
                    [member("lineHeight"), String(style.lineHeight)],
                    [member("letterSpacing"), length(style.letterSpacing.value * factor)],
                    [member("fontSizeAdjust"), adjustOf(style.fontFamily)],
                ];
            }
            case "shadow":
                return [[this.variable, token.$value.map(layer).join(", ")]];
        }
    }
}

/** A custom property of a token and its constant key within the family. */
export interface TokenMember {
    /** The constant key, such as `bodyFontSize`. */
    readonly key: string;
    /** The custom property, such as `--destack-text-base-font-size`. */
    readonly variable: Variable;
}

/** A group of the token tree with its description. */
export interface TokenGroup {
    /** The group name. */
    readonly name: string;
    /** What is in the group. */
    readonly description: string;
    /** The tokens of the group and its nested groups in document order. */
    readonly entries: readonly TokenEntry[];
}

/** A tree of design tokens in the W3C Design Tokens format, read as top-level families. */
export class TokenTree {
    /** The top-level groups in document order. */
    readonly families: readonly TokenGroup[];
    /** Every entry by its dotted path. */
    readonly #entries: ReadonlyMap<string, TokenEntry>;

    /** Create a token tree of its families, indexing every entry by its path. */
    constructor(families: readonly TokenGroup[]) {
        this.families = families;
        this.#entries = new Map(
            families
                .flatMap((family) => family.entries)
                .map((entry) => [entry.path.join("."), entry]),
        );
    }

    /** Read a token tree from its JSON document. */
    static parse(document: unknown): TokenTree {
        // read each top-level group as a family with its description
        const root = Group.parse(document);
        const families = Object.entries(root)
            .filter(([name]) => !name.startsWith("$"))
            .map(([name, group]) => {
                const parsed = Group.parse(group);

                return {
                    name,
                    description: schema.string().min(1).parse(parsed["$description"]),
                    entries: entries([name], parsed),
                };
            });
        const tree = new TokenTree(families);

        // require every alias to name a token of the tree and every hex to match its channels
        for (const entry of families.flatMap((family) => family.entries)) {
            for (const path of entry.aliases()) {
                tree.entry(path);
            }
            entry.requireHex();
        }

        return tree;
    }

    /** Find a family by name. */
    family(name: string): TokenGroup {
        const family = this.families.find((candidate) => candidate.name === name);
        if (family === undefined) {
            throw new RangeError(`unknown token family: '${name}'`);
        }

        return family;
    }

    /** Find a token entry by its path. */
    entry(path: readonly string[]): TokenEntry {
        // look the dotted path up
        const key = path.join(".");
        const entry = this.#entries.get(key);
        if (entry === undefined) {
            throw new RangeError(`unknown token: '${key}'`);
        }

        return entry;
    }
}

/** The design tokens every theme sets, read from tokens.json. */
export const TOKENS = TokenTree.parse(tokens);

/** Name the custom property of a token path, such as `--destack-color-muted-foreground`. */
function variable(path: readonly string[]): Variable {
    const words = path.map((name) =>
        name.replaceAll(/[A-Z]/gu, (letter) => `-${letter.toLowerCase()}`),
    );

    return `${VARIABLE_PREFIX}-${words.join("-")}`;
}

/** Name the constant key of a token path within its family, such as `durationShort`. */
function constantKey(path: readonly string[]): string {
    const words = path
        .slice(1)
        .map((name, index) => (index === 0 ? name : name.charAt(0).toUpperCase() + name.slice(1)));

    return words.join("");
}

/** List the entries of a group and its nested groups. */
function entries(
    prefix: readonly string[],
    group: Readonly<Record<string, unknown>>,
): TokenEntry[] {
    return Object.entries(group)
        .filter(([name]) => !name.startsWith("$"))
        .flatMap(([name, member]) => {
            const parsed = Group.parse(member);
            const path = [...prefix, name];

            return "$value" in parsed
                ? [new TokenEntry(path, Token.parse(parsed))]
                : entries(path, parsed);
        });
}

/** Read the path an alias names. */
function aliasPath(alias: string): readonly string[] {
    return alias.slice(1, -1).split(".");
}

/** Format a length in pixels. */
function length(pixels: number): string {
    return `${number(pixels)}px`;
}

/** Format a number with at most three decimals. */
function number(value: number): string {
    const rounded = Number(value.toFixed(PRECISION));

    return String(rounded === 0 ? 0 : rounded);
}

/** Read the size adjustment of a text style's face: a chosen face's in the text face, else none. */
function adjustOf(font: string): string {
    return aliasPath(font).join(".") === "font.text" ? `var(${FONT_SIZE_ADJUST}, none)` : "none";
}

/** Format a shadow layer. */
function layer(shadow: schema.Infer<typeof ShadowLayer>): string {
    const lengths = [shadow.offsetX, shadow.offsetY, shadow.blur, shadow.spread].map((dimension) =>
        length(dimension.value),
    );
    const color = `var(${variable(aliasPath(shadow.color))})`;

    return [...(shadow.inset === true ? ["inset"] : []), ...lengths, color].join(" ");
}

/** Sample an underdamped spring from rest to settled as a CSS `linear()` easing. */
function linear(spring: schema.Infer<typeof Spring>): string {
    // derive the natural frequency and damping ratio
    const frequency = Math.sqrt(spring.stiffness / spring.mass);
    const ratio = spring.damping / (2 * Math.sqrt(spring.stiffness * spring.mass));
    if (ratio >= 1) {
        throw new RangeError("a spring easing needs damping below critical damping");
    }

    // derive the damped frequency and the time the envelope settles
    const damped = frequency * Math.sqrt(1 - ratio * ratio);
    const settled = Math.log(1 / SPRING_SETTLED) / (ratio * frequency);

    // sample the displacement over the settling time
    const points = Array.from({ length: SPRING_SEGMENTS + 1 }, (_, index) => {
        // displace the spring at this point in time, landing exactly on 1
        const time = (index / SPRING_SEGMENTS) * settled;
        const envelope = Math.exp(-ratio * frequency * time);
        const phase =
            Math.cos(damped * time) + ((ratio * frequency) / damped) * Math.sin(damped * time);

        return index === SPRING_SEGMENTS ? 1 : 1 - envelope * phase;
    });

    return `linear(${points.map(number).join(", ")})`;
}
