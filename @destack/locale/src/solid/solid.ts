import { createContext, useContext } from "solid-js";
import { Localization } from "../localization/localization.ts";

/** The person's localization, which a mounted view or a website provides, the source language without one. */
export const LocaleContext = createContext<Localization>(Localization.of("en", []));

/** Read the person's localization to render messages and format values in. */
export function useLocale(): Localization {
    return useContext(LocaleContext);
}
