// One hidden key for all brands.
declare const __brand: unique symbol;

// Generic brand.
export type Brand<T, Name extends string> = T & { readonly [__brand]: Name };

// Generic brander factory.
export const brandAs =
    <T, Name extends string>() =>
    (v: T) =>
        v as Brand<T, Name>;

// Examples.
export type Uint8 = Brand<number, "uint8">;
export type Uint16 = Brand<number, "uint16">;

export const asUint8 = brandAs<number, "uint8">();
export const asUint16 = brandAs<number, "uint16">();

// Optional runtime guard + brander.
export function isUint8(n: number): n is Uint8 {
    return Number.isInteger(n) && n >= 0 && n <= 255;
}
export function uint8(n: number): Uint8 {
    if (!isUint8(n)) throw new RangeError("uint8 out of range");
    return n as Uint8;
}
