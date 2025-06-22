import { Change, EnvironmentType, IsSubject, Node, Origin, QueryConnection, Space } from "@/language";
import { Temporal } from "temporal-polyfill";
import { Edit } from "../common";

export class Session {
  changes: Change[];
  closedAt: Temporal.ZonedDateTime | null;
  connections: QueryConnection[];
  dirty: Record<string, Node>;
  edits: Edit[];
  oracle: Oracle;
  origin: Origin | null;
  space: Space | null;
  store: Store | null;
  subject: (Node & IsSubject) | null;
}
