import { ReactiveSupergraph } from "@destack-web/language/core/runtime/graph";
import {
  Client,
  EntityStore,
  Event,
  EventStore,
  IsSubject,
  Node,
  Oracle,
  Session,
  Space,
} from "destack";

/** A reactive variant of Session. */
export class ReactiveSession extends Session {
  constructor(options?: {
    oracle?: Oracle;
    space?: Space | null;
    client?: Client | null;
    clientNonce?: string | null;
    subject?: (Node & IsSubject) | null;
    store?: EventStore | EntityStore | null;
  }) {
    super({
      ...options,
      supergraphClass: ReactiveSupergraph,
    });
  }

  override flush(): void {
    super.flush();
  }

  override async stage(): Promise<void> {
    await super.stage();
  }

  override async commit(): Promise<Event[]> {
    const events = await super.commit();
    return events;
  }
}
