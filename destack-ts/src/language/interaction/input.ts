import type {
  Branch,
  Datetime,
  EventStatus,
  NodeReference,
  Snapshot,
  Space,
  UInt128,
  UUID,
} from "@destack/language/core";
import { type Entity, Event, NodeType } from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";

/* ==== DESTACK_GENERATED_START:NODE:2000000 ==== */
/**
 * An InputEvent is an Event that corresponds to some direct user input.
 */
export abstract class InputEvent extends Event {
  static metatype: NodeType = NodeType.INPUT_EVENT;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  abstract get precededBy(): Event | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  abstract get causedBy(): Event | null;
  declare readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system).
   */
  declare readonly createdAt: Datetime;

  /**
   * The logical time this Event was created (system).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Event.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

  /**
   * The Client that created this Event (client).
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference;

  /**
   * The nonce of the Client that created this Event (client).
   */
  declare readonly clientNonce: UUID;

  /**
   * The time in the Client when it created this Event (client).
   */
  declare readonly clientCreatedAt: Datetime;

  /**
   * The logical time in the Client when it created this Event (client).
   */
  declare readonly clientEpoch: UInt128;

  /**
   * The status of the Event (system).
   */
  declare readonly status: EventStatus;

  /**
   * InputEvent.node
   */
  abstract get node(): Entity | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INPUT_EVENT, InputEvent);
/* ==== DESTACK_GENERATED_END:NODE:2000000 ==== */
