import { destackPlugin } from "@destack-sh/bun";
import { plugin } from "bun";

plugin(destackPlugin({}));

// @ts-ignore
export * from "./index.ds";
