// styles
export { create, firstThatWorks, props } from "@stylexjs/stylex";
export type {
    CompiledStyles,
    CSSProperties,
    InlineStyles,
    StaticStyles,
    StaticStylesWithout,
    StyleXArray as StyleArray,
    StyleXClassNameFor as ClassNameFor,
    StyleXStylesWithout as StylesWithout,
} from "@stylexjs/stylex";

// variables and themes
export { createTheme, defineConsts, defineVars, types } from "@stylexjs/stylex";
export type { StyleXVar as Variable, Theme, VarGroup as Variables } from "@stylexjs/stylex";

// conditions
export { defaultMarker, defineMarker, env, when } from "@stylexjs/stylex";

// animations
export { keyframes, positionTry, viewTransitionClass } from "@stylexjs/stylex";
export type { Keyframes, PositionTry } from "@stylexjs/stylex";
