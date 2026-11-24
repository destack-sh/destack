import { parser } from "@lezer/yaml";
import { Language } from "../../language/index.ts";
import { LezerGrammar } from "../lezer.ts";

/** The grammar of YAML. */
export default new LezerGrammar(Language.of("yaml"), parser);
