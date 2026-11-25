import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { INDENT } from "../disclosure/index.ts";

/** The places of a timeline's parts. */
export const places = style.create({
    columns: (template: string) => ({ gridTemplateColumns: template }),
    span: (left: number, width: number) => ({
        insetInlineStart: `${String(left * 100)}%`,
        width: `${String(width * 100)}%`,
    }),
    at: (left: number) => ({ insetInlineStart: `${String(left * 100)}%` }),
    before: (left: number) => ({ insetInlineEnd: `${String((1 - left) * 100)}%` }),
    mark: (labelWidth: string, share: number) => ({
        insetInlineStart: `calc(${labelWidth} + (100% - ${labelWidth}) * ${String(share)})`,
    }),
    after: (labelWidth: string) => ({ insetInlineStart: labelWidth }),
    track: (labelWidth: string) => ({
        insetInlineStart: labelWidth,
        width: `calc(100% - ${labelWidth})`,
    }),
    top: (pixels: number) => ({ top: `${String(pixels)}px` }),
    markSpan: (labelWidth: string, share: number, width: number) => ({
        insetInlineStart: `calc(${labelWidth} + (100% - ${labelWidth}) * ${String(share)})`,
        width: `calc((100% - ${labelWidth}) * ${String(width)})`,
    }),
    row: (top: number, height: number) => ({
        transform: `translateY(${String(top)}px)`,
        height: `${String(height)}px`,
    }),
    height: (height: number) => ({ height: `${String(height)}px` }),
    inset: (gutter: string, depth: number) => ({
        insetInlineStart: `calc(${gutter} + ${String(depth)} * ${INDENT})`,
    }),
    trace: (left: number, width: number, top: number, mark: string) => ({
        insetInlineStart: `${String(left * 100)}%`,
        width: `max(${space[1]}, ${String(width * 100)}%)`,
        top: `${String(top * 100)}%`,
        backgroundColor: mark,
    }),
    fill: (background: string, label: string) => ({ backgroundColor: background, color: label }),
    fade: (toward: string) => ({
        maskImage: `linear-gradient(${toward}, black 40%, transparent)`,
    }),
    progress: (done: number, mark: string) => ({
        width: `${String(done * 100)}%`,
        backgroundColor: `color-mix(in oklab, ${mark} 35%, transparent)`,
    }),
});
