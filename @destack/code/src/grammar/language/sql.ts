import { pgSQL } from "@codemirror/legacy-modes/mode/sql";
import { Language } from "../../language/index.ts";
import { StreamGrammar } from "../stream.ts";

/** The grammar of SQL. */
export default new StreamGrammar(Language.of("sql"), pgSQL);
