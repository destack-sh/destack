import type { EventStatus, IsSubject, NodeReference, Snapshot } from "@destack/language/core";
import { Entity, Event, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:560000 ==== */
/**
 * An InputEvent is an Event that corresponds to some direct user input.
 */
export abstract class InputEvent extends Event {
  static metatype: NodeType = NodeType.INPUT_EVENT;

  /**
   * Event.parent
   */
  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  abstract get createdBy(): (Entity & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  declare readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  abstract get node(): View | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INPUT_EVENT, InputEvent);
/* ==== DESTACK_GENERATED_END:NODE:560000 ==== */
