import { creditPrices, type currencies } from "../content/generated/pricing";

/** A currency Destack bills in. */
export type Currency = (typeof currencies)[number];

/** The regions that pay in a currency other than US dollars, by region code. */
const regionCurrencies: Readonly<Record<string, Currency>> = {
    AU: "AUD",
    CA: "CAD",
    CH: "CHF",
    LI: "CHF",
    GB: "GBP",
    AT: "EUR",
    BE: "EUR",
    CY: "EUR",
    DE: "EUR",
    EE: "EUR",
    ES: "EUR",
    FI: "EUR",
    FR: "EUR",
    GR: "EUR",
    HR: "EUR",
    IE: "EUR",
    IT: "EUR",
    LT: "EUR",
    LU: "EUR",
    LV: "EUR",
    MT: "EUR",
    NL: "EUR",
    PT: "EUR",
    SI: "EUR",
    SK: "EUR",
};

/** The regions of the time zones that pay in a currency other than US dollars, by IANA zone. */
const zoneRegions: Readonly<Record<string, string>> = {
    "Australia/Adelaide": "AU",
    "Australia/Brisbane": "AU",
    "Australia/Darwin": "AU",
    "Australia/Hobart": "AU",
    "Australia/Melbourne": "AU",
    "Australia/Perth": "AU",
    "Australia/Sydney": "AU",
    "America/Edmonton": "CA",
    "America/Halifax": "CA",
    "America/Regina": "CA",
    "America/St_Johns": "CA",
    "America/Toronto": "CA",
    "America/Vancouver": "CA",
    "America/Winnipeg": "CA",
    "Atlantic/Azores": "PT",
    "Atlantic/Canary": "ES",
    "Atlantic/Madeira": "PT",
    "Europe/Amsterdam": "NL",
    "Europe/Athens": "GR",
    "Europe/Berlin": "DE",
    "Europe/Bratislava": "SK",
    "Europe/Brussels": "BE",
    "Europe/Dublin": "IE",
    "Europe/Helsinki": "FI",
    "Europe/Lisbon": "PT",
    "Europe/Ljubljana": "SI",
    "Europe/London": "GB",
    "Europe/Luxembourg": "LU",
    "Europe/Madrid": "ES",
    "Europe/Malta": "MT",
    "Europe/Nicosia": "CY",
    "Europe/Paris": "FR",
    "Europe/Riga": "LV",
    "Europe/Rome": "IT",
    "Europe/Tallinn": "EE",
    "Europe/Vaduz": "LI",
    "Europe/Vienna": "AT",
    "Europe/Vilnius": "LT",
    "Europe/Zagreb": "HR",
    "Europe/Zurich": "CH",
};

/** Return the currency a reader's region pays in, by time zone, else by language, else dollars. */
export function currencyOf(zone: string, language: string): Currency {
    const region = zoneRegions[zone] ?? new Intl.Locale(language).maximize().region;

    return (region === undefined ? undefined : regionCurrencies[region]) ?? "USD";
}

/** Convert a rate in US dollars into a currency at its credit price. */
export function convert(dollars: number, currency: Currency): number {
    return dollars * creditPrices[currency];
}

/** Format an amount in a currency, with cents only where it has them or a rate needs them. */
export function money(amount: number, currency: Currency, isRate = false): string {
    return new Intl.NumberFormat("en-US", {
        style: "currency",
        currency,
        currencyDisplay: "symbol",
        minimumFractionDigits: isRate || !Number.isInteger(amount) ? 2 : 0,
        maximumFractionDigits: isRate ? 4 : 2,
    }).format(amount);
}
