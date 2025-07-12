import { ReactiveGraph, ReactiveSupergraph } from "@destack-web/language/core/runtime/graph";
import { batch } from "@preact/signals-react";
import {
  Client,
  EditEvent,
  Entity,
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
  /** Unflushed Entities. */
  readonly _dirtyEntities: Map<string, Entity>;

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

  override archive(node: Entity): void {
    super.archive(node);
    this._dirtyEntities.set(node.id, node);
  }

  override unarchive(node: Entity): void {
    super.unarchive(node);
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
