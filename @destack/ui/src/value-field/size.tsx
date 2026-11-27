import { t } from "@destack/locale";
import {
    Size,
    SIZE_LIMIT_KINDS,
    SizeLimit,
    SizeLimitKind,
    SIZING_KINDS,
    Sizing,
    SizingKind,
} from "@destack/style";
import { For, type JSX } from "@destack/view";
import { KeywordParts } from "./dimension.tsx";
import { choiceType, ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The kinds of a sizing, and of a limit: none, or a length. */
const KINDS = {
    sizing: choiceType(SizingKind.options, SIZING_KINDS),
    limit: choiceType(SizeLimitKind.options, SIZE_LIMIT_KINDS),
};

/** No limit. */
const NO_LIMIT: SizeLimit = { kind: "none" };

/** The size an input starts at, around its content and within no limits. */
const START_SIZE: Size = {
    width: { kind: "fit" },
    height: { kind: "fit" },
    minWidth: NO_LIMIT,
    maxWidth: NO_LIMIT,
    minHeight: NO_LIMIT,
    maxHeight: NO_LIMIT,
};

/** The limits of a size an input shows, with what each is called. */
const LIMITS = [
    ["minWidth", t.context("label")`Least width`],
    ["maxWidth", t.context("label")`Greatest width`],
    ["minHeight", t.context("label")`Least height`],
    ["maxHeight", t.context("label")`Greatest height`],
] as const;

/** Render the input of a size: how it sizes across and down, and its limits. */
export function SizeInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the size, around its content where none is given
    const size = (): Size => Size.safeParse(properties.value).data ?? START_SIZE;
    const change = (next: Partial<Size>): void => properties.onValueChange({ ...size(), ...next });

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.group}>
            {/* Width and height */}
            <KeywordParts
                label={t.context("label")`Width`}
                kinds={KINDS.sizing}
                lengthKind="fixed"
                schema={Sizing}
                value={size().width}
                disabled={properties.disabled}
                onValueChange={(width) => change({ width })}
            />
            <KeywordParts
                label={t.context("label")`Height`}
                kinds={KINDS.sizing}
                lengthKind="fixed"
                schema={Sizing}
                value={size().height}
                disabled={properties.disabled}
                onValueChange={(height) => change({ height })}
            />

            {/* Limits */}
            <For each={LIMITS}>
                {([key, label]) => (
                    <KeywordParts
                        label={label}
                        kinds={KINDS.limit}
                        lengthKind="length"
                        schema={SizeLimit}
                        value={size()[key]}
                        disabled={properties.disabled}
                        onValueChange={(limit) => change({ [key]: limit })}
                    />
                )}
            </For>
        </ValueGroup>
    );
}
