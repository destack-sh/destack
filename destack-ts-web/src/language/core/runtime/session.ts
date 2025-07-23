import { ReactiveGraph } from "@destack-web/language/core/runtime/graph";
import { batch } from "@preact/signals-react";
import { EditEvent, Entity, Event, Graph, NodeReference, Oracle, Session } from "destack";

/** A reactive variant of Session. */
export class ReactiveSession extends Session {
  /** Unflushed Entities. */
  readonly _dirtyEntities: Map<string, Entity>;

  constructor(options?: {
    oracle?: Oracle;
    clientPtr?: NodeReference | null;
    clientNonce?: string | null;
    actorPtr?: NodeReference | null;
    graph?: Graph | null;
    epoch?: number;
  }) {
    super({
      ...options
    });
    this._dirtyEntities = new Map();
  }

  override create(node: Entity): void {
    super.create(node);
    this._dirtyEntities.set(node.id, node);
  }

  override upsert(node: Entity): void {
    super.upsert(node);
    this._dirtyEntities.set(node.id, node);
  }

  override update(node: Entity, edit: EditEvent): void {
    super.update(node, edit);
    this._dirtyEntities.set(node.id, node);
  }

  override move(node: Entity, parent: Entity): void {
    super.move(node, parent);
    this._dirtyEntities.set(node.id, node);
  }

  override delete(node: Entity): void {
    super.delete(node);
    this._dirtyEntities.delete(node.id);
  }

  override restore(node: Entity): void {
    super.restore(node);
    this._dirtyEntities.set(node.id, node);
  }

  override _onFlush(): void {
    batch(() => {
      for (const node of this._dirtyEntities.values()) {
        if ("touch" in node._graph) {
          (node._graph as ReactiveGraph).touch(node.id);
        }
      }
    });
    this._dirtyEntities.clear();
  }

  override async flush(): Promise<void> {
    await super.flush();
  }

  override async commit(): Promise<Event[]> {
    const events = await super.commit();
    return events;
  }
}
