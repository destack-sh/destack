import { CopyButton } from "../layout/copy";

/** Copy a text, such as a code listing's source. */
export default function Copy(properties: { readonly value: string }) {
    return <CopyButton value={properties.value} />;
}
