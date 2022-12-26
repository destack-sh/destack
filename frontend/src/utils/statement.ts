import type { FragmentType } from "@/gql";
import type { StatementHeaderType } from "@/utils/fragments";

export type StatementProps<T> = {
  statement: FragmentType<typeof StatementHeaderType>;
  content: T;
  focused: boolean;
  editing: boolean;
  readonly: boolean;
  lineNumberBase: number;
  xOffset: number;
};

export type StatementEmits = {
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
};
