import { defineSchema, schema } from "@destack/schema";
import { Color, HexColor } from "../palette/color.ts";
import tokens from "./tokens.json" with { type: "json" };

/** The prefix of every custom property a theme sets. */
export const VARIABLE_PREFIX = "--destack";

/** A custom property a theme sets. */
export type Variable = `${typeof VARIABLE_PREFIX}-${string}`;

/** The custom property scaling every duration, 0 under reduced motion. */
export const MOTION_VARIABLE: Variable = `${VARIABLE_PREFIX}-motion-scale`;

/** The `$extensions` namespace of Destack's token metadata, per the W3C Design Tokens format. */
export const EXTENSION = "app.destack";

/** The envelope fraction at which a spring counts as settled: 0.1% of its travel. */
const SPRING_SETTLED = 0.001;

/** The segments of a spring's `linear()` easing, smooth at 60 Hz for durations up to half a second. */
const SPRING_SEGMENTS = 32;

/** The members of a typography token, each its own custom property. */
const TYPOGRAPHY_MEMBERS = [
    "fontFamily",
    "fontSize",
    "fontWeight",
    "lineHeight",
    "letterSpacing",
] as const;

/** The decimal places of every computed CSS number, below a thousandth of a pixel. */
const PRECISION = 3;

/** A length in pixels. */
const Dimension = schema.object({
    /** The amount. */
    value: schema.number(),
    /** The unit. */
    unit: schema.literal("px"),
});

/** A sRGB color with optional alpha, per the W3C Design Tokens color format. */
const ColorValue = schema.object({
    /** The color space. */
    colorSpace: schema.literal("srgb"),
    /** The red, green and blue channels from 0 to 1. */
    components: schema.tuple([
        schema.number().min(0).max(1),
        schema.number().min(0).max(1),
        schema.number().min(0).max(1),
    ]),
    /** The opacity from 0 to 1, opaque when absent. */
    alpha: schema.number().min(0).max(1).exactOptional(),
    /** The six-digit hex form of the channels, which the token tree checks against them. */
    hex: HexColor.exactOptional(),
});

/** A group of the token tree before its members are read. */
const Group = schema.record(schema.string(), schema.json());

/** A reference to another token by its path, such as `{font.text}`. */
const Alias = schema.string().regex(/^\{[A-Za-z0-9.]+\}$/u);

/** One layer of a shadow. */
const ShadowLayer = schema.object({
    /** The shadow color, as an alias. */
    color: Alias,
    /** The horizontal offset. */
    offsetX: Dimension,
    /** The vertical offset. */
    offsetY: Dimension,
    /** The blur radius. */
    blur: Dimension,
    /** The spread distance. */
    spread: Dimension,
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

/** The fields every token carries beside its type and value. */
const common = {
    /** What the token is for. */
    $description: schema.string().min(1),
};

/** A design token in the W3C Design Tokens format. */
export const Token = defineSchema(
    schema.discriminatedUnion("$type", [
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
            $value: Dimension,
            ...common,
        }),
        schema.object({
            /** A duration. */
            $type: schema.literal("duration"),
            /** The duration with full motion. */
            $value: schema.object({ value: schema.number().min(0), unit: schema.literal("ms") }),
            ...common,
        }),
        schema.object({
            /** An easing curve. */
            $type: schema.literal("cubicBezier"),
            /** The control points, the portable form of a spring. */
            $value: schema.tuple([
                schema.number(),
                schema.number(),
                schema.number(),
                schema.number(),
            ]),
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
            $value: schema.array(schema.string().min(1)).min(1),
            ...common,
        }),
        schema.object({
            /** A font weight. */
            $type: schema.literal("fontWeight"),
            /** The weight from 1 to 1000, as CSS font-weight takes it. */
            $value: schema.number().int().min(1).max(1000),
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
                fontSize: Dimension,
                /** The font weight. */
                fontWeight: schema.number().int().min(1).max(1000),
                /** The line height as a multiple of the font size. */
                lineHeight: schema.number().positive(),
                /** The letter spacing. */
                letterSpacing: Dimension,
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
    ]),
);
/** A design token in the W3C Design Tokens format. */
export type Token = schema.Infer<typeof Token>;

/** A token at its path in the token tree, such as `["text", "body"]`. */
export class TokenEntry {
    /** The group names and the token name, the family first. */
    readonly path: readonly string[];
    /** The token. */
    readonly token: Token;

    /** Hold a token at its path. */
    constructor(path: readonly string[], token: Token) {
        this.path = path;
        this.token = token;
    }

    /** The custom property, such as `--destack-color-muted-foreground`. */
    get variable(): Variable {
        return variable(this.path);
    }

    /** The constant key within the family, such as `durationShort`. */
    get key(): string {
        return constantKey(this.path);
    }

    /** List the constant key and custom property of each member, the token itself unless composite. */
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
            const hex = Color.format(color.components);
            if (color.hex !== undefined && color.hex.toLowerCase() !== hex) {
                throw new RangeError(
                    `token ${this.path.join(".")} has hex ${color.hex} for ${hex}`,
                );
            }
        }
    }

    /** Compute the CSS values of the custom properties, scaling lengths by a factor. */
    values(factor: number): readonly (readonly [Variable, string])[] {
        const token = this.token;
        switch (token.$type) {
            case "color": {
                const light = rgba(token.$value);
                const dark = rgba(token.$extensions[EXTENSION].dark);

                return [[this.variable, `light-dark(${light}, ${dark})`]];
            }
            case "dimension":
                return [[this.variable, length(token.$value.value * factor)]];
            case "duration": {
                const scale = `var(${MOTION_VARIABLE}, 1)`;

                return [[this.variable, `calc(${token.$value.value}ms * ${scale})`]];
            }
            case "cubicBezier": {
                const spring = token.$extensions?.[EXTENSION].spring;
                const bezier = `cubic-bezier(${token.$value.join(", ")})`;

                return [[this.variable, spring === undefined ? bezier : linear(spring)]];
            }
            case "fontFamily":
                return [[this.variable, fontStack(token.$value)]];
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
    /** The custom property, such as `--destack-text-body-font-size`. */
    readonly variable: Variable;
}

/** A group of the token tree with its description. */
export interface TokenGroup {
    /** The group name. */
    readonly name: string;
    /** What the group holds. */
    readonly description: string;
    /** The tokens of the group and its nested groups in document order. */
    readonly entries: readonly TokenEntry[];
}

/** A tree of design tokens in the W3C Design Tokens format, read as top-level families. */
export class TokenTree {
    /** The top-level groups in document order. */
    readonly families: readonly TokenGroup[];

    /** Hold the families of a token tree. */
    constructor(families: readonly TokenGroup[]) {
        this.families = families;
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
            throw new RangeError(`unknown token family: ${name}`);
        }

        return family;
    }

    /** Find a token entry by its path. */
    entry(path: readonly string[]): TokenEntry {
        // search every family for the dotted path
        const key = path.join(".");
        const entry = this.families
            .flatMap((family) => family.entries)
            .find((candidate) => candidate.path.join(".") === key);
        if (entry === undefined) {
            throw new RangeError(`unknown token: ${key}`);
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

/** Format a token color as a hex color, with alpha when translucent. */
function rgba(color: schema.Infer<typeof ColorValue>): string {
    const hex = Color.format(color.components);

    return color.alpha === undefined ? hex : Color.translucent(hex, color.alpha);
}

/** Format a font stack, quoting family names with spaces. */
function fontStack(names: readonly string[]): string {
    return names.map((name) => (name.includes(" ") ? `"${name}"` : name)).join(", ");
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
