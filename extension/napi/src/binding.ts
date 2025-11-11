import { createRequire } from "node:module";

export type DestackNapi = {
    fibonacci(n: number): number;
};

const requireNative = createRequire(import.meta.url);
const nativeBinding = requireNative("../index.js") as Partial<DestackNapi>;

if (typeof nativeBinding.fibonacci !== "function") {
    throw new Error("fibonacci binding is unavailable in the native module");
}

export const { fibonacci } = nativeBinding;
export default nativeBinding as DestackNapi;
