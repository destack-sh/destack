import { Color } from "../color/index.ts";
import { schema } from "@destack/schema";

/** Colors in order, which whatever uses them spreads out, such as a shader's. */
export const Palette = schema.array(Color.schema).meta({ id: "style.palette", title: "Palette" });
/** Colors in order. */
export type Palette = schema.Infer<typeof Palette>;
