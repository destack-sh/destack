import { Url } from "@destack/schema";
import type { JSX } from "@destack/view";
import { Input } from "../input/index.ts";
import type { NamedValueInputProperties } from "./value-field.tsx";

/** Render the input of a web address, giving it once it parses as one and none once cleared. */
export function UrlInput(properties: NamedValueInputProperties): JSX.Element {
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
