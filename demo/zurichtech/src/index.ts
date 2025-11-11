import { destackPlugin } from "@destack/bun";
import { plugin } from "bun";

plugin(destackPlugin);

// @ts-ignore
export * from "./index.ds";
