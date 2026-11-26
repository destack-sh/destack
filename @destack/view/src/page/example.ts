import type { LocaleTag } from "@destack/locale";
import { createContext, useContext } from "solid-js";

/** An example or scenario a page shows, by its name and its kind. */
export interface IndexEntry {
    /** The name, as `<shown>.<member>`. */
    readonly name: string;
    /** Whether it is an example or a scenario. */
    readonly kind: "example" | "scenario";
}

/** The example page an example renders on: what it shows, and the languages it translates into. */
export interface ExamplePage {
    /** The examples and scenarios it shows. */
    readonly index: readonly IndexEntry[];
    /** The locales its catalogs translate into, the source locale aside. */
    readonly locales: readonly LocaleTag[];
}

/** The example page an example renders on, none outside one. */
export const ExamplePageContext = createContext<ExamplePage>({ index: [], locales: [] });

/** Read the example page an example renders on. */
export function useExamplePage(): ExamplePage {
    return useContext(ExamplePageContext);
}
