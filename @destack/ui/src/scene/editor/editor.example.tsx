import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Editor } from "./editor.tsx";

/** The package's examples beside their page and the properties of what each shows. */
export const editorDefault = defineExample({
    of: Editor,
    name: "default",
    description: t`The package's examples beside their page and the properties of what each shows.`,
    layout: "fullscreen",
    properties: { first: "button.default" },
    render: (properties) => <Editor first={properties.first ?? "button.default"} />,
});
