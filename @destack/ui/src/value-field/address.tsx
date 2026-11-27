import { Url } from "@destack/schema";
import { type JSX } from "@destack/view";
import { Input } from "../input/index.ts";
import type { NamedValueField } from "./value-field.tsx";

/** The properties of the input of a named JSON value. */
type NamedProperties = Parameters<NamedValueField>[0];

/** Render the input of a web address, giving it once it parses as one and none once cleared. */
export function UrlInput(properties: NamedProperties): JSX.Element {
    return (
        <Input
            type="url"
            inputmode="url"
            placeholder="https://"
            aria-label={properties["aria-label"]}
            aria-describedby={properties["aria-describedby"]}
            disabled={properties.disabled}
            value={typeof properties.value === "string" ? properties.value : ""}
            onChange={(event) => {
                // give the address once it parses, and none for an empty field
                const text = event.currentTarget.value.trim();
                const address = Url.safeParse(text).data;
                if (text.length === 0 || address !== undefined) {
                    properties.onValueChange(address);
                }
            }}
        />
    );
}
