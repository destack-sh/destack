import { ExpressionIn, NodeClass, WithSubqueries, toSubqueries } from "@destack/language";
import {
  Aggregation,
  AggregationType,
  Condition,
  Expression,
  Icon,
  Join,
  MaterializationType,
  NodeReference,
  PropertyDefinition,
  Query,
  QueryType,
  RelationReference,
  ResourceStatus,
  Sort,
  Value,
} from "@destack/language/core";
import { EnumType, Node, NodeType, TraitType } from "@destack/language/core/builtin";
import { Script } from "@destack/language/logic";
import { registerEnumClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import { Casing, toCasing } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/** Internal base class for Trait companion objects.*/
class TraitFacade {
  readonly metatype: TraitType;
  readonly __properties__: Record<string, PropertyDefinition>;
  readonly __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: TraitType) {
    this.metatype = metatype;
    this.__properties__ = {};
    this.__propertiesById__ = {};
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  property(name: string): PropertyDefinition {
    const prop = this.__properties__[name];
    if (!prop) {
      throw new Error(`Property ${name} not found on ${this.constructor.name}`);
    }
    return prop;
  }

  /** Make a get Query for this Node/Trait type. */
  get(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a search Query for this Node/Trait type. */
  search(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      having?: Condition;
      groupBy?: ExpressionIn[];
      sort?: Sort[];
      limit?: number;
      offset?: number;
    }>,
  ): Query {
    const { where, name, join, having, groupBy, sort, limit, offset, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_NODE : QueryType.NODE,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      sort,
      limit,
      offset,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make an exists Query for this Node/Trait type. */
  exists(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
    }>,
  ): Query {
    const { where, name, join, ...subqueries } = options;
    const query = new Query({
      type: QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      aggregation: Aggregation.of(AggregationType.EXISTS),
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a count Query for this Node/Trait type. */
  count(
    options: WithSubqueries<{
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(NodeType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.COUNT),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a min Query for this Node/Trait type. */
  min(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.MIN, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a max Query for this Node/Trait type. */
  max(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.MAX, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }

  /** Make a sum Query for this Node/Trait type. */
  sum(
    options: WithSubqueries<{
      expression: ExpressionIn;
      where?: Condition;
      name?: string;
      join?: Join;
      groupBy?: ExpressionIn[];
      having?: Condition;
      sort?: Sort[];
    }>,
  ): Query {
    const { expression, where, name, join, groupBy, having, sort, ...subqueries } = options;
    const query = new Query({
      type: groupBy ? QueryType.GROUPED_SCALAR : QueryType.SCALAR,
      relation: RelationReference.of(this as unknown as NodeClass),
      name: name ?? toCasing(TraitType[this.metatype], Casing.CAMEL),
      join,
      where,
      having,
      groupBy: groupBy?.map(Expression.of),
      aggregation: Aggregation.of(AggregationType.SUM, Expression.of(expression)),
      sort,
      subqueries: toSubqueries(subqueries),
    });
    return query;
  }
}

/* ==== DESTACK_GENERATED_START:ENUM:50101 ==== */
/**
 * JoinablePermission
 */
export enum JoinablePermission {
  INVITE = 1,
  REMOVE = 2,
  KICK = 3,
  BAN = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.JOINABLE_PERMISSION, JoinablePermission);
/* ==== DESTACK_GENERATED_END:ENUM:50101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:100 ==== */
/**
 * A Node with a plain name.
 */
export interface HasName {
  /**
   * HasName.name
   */
  name: string;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class HasName$Type extends TraitFacade {}
export const HasName = new HasName$Type(TraitType.HAS_NAME);
/* ==== DESTACK_GENERATED_END:TRAIT:100 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:101 ==== */
/**
 * A Node with a slug.
 */
export interface HasSlug {
  /**
   * HasSlug.slug
   */
  slug: string | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class HasSlug$Type extends TraitFacade {}
export const HasSlug = new HasSlug$Type(TraitType.HAS_SLUG);
/* ==== DESTACK_GENERATED_END:TRAIT:101 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:102 ==== */
/**
 * A Node with an icon.
 */
export interface HasIcon {
  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class HasIcon$Type extends TraitFacade {}
export const HasIcon = new HasIcon$Type(TraitType.HAS_ICON);
/* ==== DESTACK_GENERATED_END:TRAIT:102 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:51 ==== */
/**
 * A Node that is "tracked" on create/update.
 */
export interface IsTracked {
  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  get createdBy(): (Node & IsSubject) | null;
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  get updatedBy(): (Node & IsSubject) | null;
  readonly updatedByPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsTracked$Type extends TraitFacade {}
export const IsTracked = new IsTracked$Type(TraitType.TRACKED);
/* ==== DESTACK_GENERATED_END:TRAIT:51 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:9000 ==== */
/**
 * A Node that is a visual in some sense (views, styles, drawings, ...).
 */
export interface IsVisual extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsVisual$Type extends TraitFacade {}
export const IsVisual = new IsVisual$Type(TraitType.VISUAL);
/* ==== DESTACK_GENERATED_END:TRAIT:9000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:50 ==== */
/**
 * A Node that is frozen (read-only).
 * TODO :Cleanup: Nodes don't set 'real' frozen=True (like StructFrozen) :PretendFrozen
 *  (because that would require two separate inheritance chains for NodeMutable and NodeFrozen,
 *   which would have to include copies of every relevant trait and .. ughh no)
 */
export interface IsFrozen {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsFrozen$Type extends TraitFacade {}
export const IsFrozen = new IsFrozen$Type(TraitType.FROZEN);
/* ==== DESTACK_GENERATED_END:TRAIT:50 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:52 ==== */
/**
 * A Node that can be archived.
 */
export interface IsArchivable {
  /**
   * IsArchivable.archivedAt
   */
  readonly archivedAt: Temporal.ZonedDateTime | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsArchivable$Type extends TraitFacade {}
export const IsArchivable = new IsArchivable$Type(TraitType.ARCHIVABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:52 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:53 ==== */
/**
 * A Node that can be deleted.
 */
export interface IsDeletable {
  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsDeletable$Type extends TraitFacade {}
export const IsDeletable = new IsDeletable$Type(TraitType.DELETABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:53 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:23 ==== */
/**
 * A Node that defines a Custom Node type.
 */
export interface IsCustomNodeDefinition {
  get prototype(): (Node & IsCustomNode) | null;
  set prototype(value: (Node & IsCustomNode) | null);
  prototypePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsCustomNodeDefinition$Type extends TraitFacade {}
export const IsCustomNodeDefinition = new IsCustomNodeDefinition$Type(TraitType.CUSTOM_NODE_DEFINITION);
/* ==== DESTACK_GENERATED_END:TRAIT:23 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:24 ==== */
/**
 * A Node that is asome Custom Node.
 */
export interface IsCustomNode {
  get definition(): (Node & IsCustomNodeDefinition) | null;
  readonly definitionPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsCustomNode$Type extends TraitFacade {}
export const IsCustomNode = new IsCustomNode$Type(TraitType.CUSTOM_NODE);
/* ==== DESTACK_GENERATED_END:TRAIT:24 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:55 ==== */
/**
 * A Node that can be extended with custom Values (one Value per Field).
 */
export interface IsExtensible {
  /**
   * IsExtensible.value
   */
  value: Map<string, Value>;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsExtensible$Type extends TraitFacade {}
export const IsExtensible = new IsExtensible$Type(TraitType.EXTENSIBLE);
/* ==== DESTACK_GENERATED_END:TRAIT:55 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:56 ==== */
/**
 * A Node that can be ordered.
 */
export interface IsOrdered {
  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsOrdered$Type extends TraitFacade {}
export const IsOrdered = new IsOrdered$Type(TraitType.ORDERED);
/* ==== DESTACK_GENERATED_END:TRAIT:56 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5532 ==== */
/**
 * A Node that can be reacted to (with Reactions).
 */
export interface IsReactable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsReactable$Type extends TraitFacade {}
export const IsReactable = new IsReactable$Type(TraitType.REACTABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:5532 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5530 ==== */
/**
 * A Node that can be starred (with Stars).
 */
export interface IsStarable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsStarable$Type extends TraitFacade {}
export const IsStarable = new IsStarable$Type(TraitType.STARABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:5530 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5534 ==== */
/**
 * A Node that can be followed (with Follows).
 */
export interface IsFollowable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsFollowable$Type extends TraitFacade {}
export const IsFollowable = new IsFollowable$Type(TraitType.FOLLOWABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:5534 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3003 ==== */
/**
 * A Node that can be sourced from / defined by a Script.
 */
export interface IsSourceable extends IsOrdered {
  get source(): Script | null;
  readonly sourcePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsSourceable$Type extends TraitFacade {}
export const IsSourceable = new IsSourceable$Type(TraitType.SOURCEABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:3003 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3002 ==== */
/**
 * A Node that can be scripted.
 */
export interface IsScriptable {
  get script(): Script | null;
  set script(value: Script | null);
  scriptPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsScriptable$Type extends TraitFacade {}
export const IsScriptable = new IsScriptable$Type(TraitType.SCRIPTABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:3002 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3001 ==== */
/**
 * A Node that can be run (with Runs).
 */
export interface IsRunnable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsRunnable$Type extends TraitFacade {}
export const IsRunnable = new IsRunnable$Type(TraitType.RUNNABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:3001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:3000 ==== */
/**
 * A Node that can define an Action.
 */
export interface IsActionable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsActionable$Type extends TraitFacade {}
export const IsActionable = new IsActionable$Type(TraitType.ACTIONABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:3000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:500 ==== */
/**
 * A Node that can be owned by another Node.
 */
export interface IsOwnable {
  get ownedBy(): (Node & IsOwner) | null;
  set ownedBy(value: (Node & IsOwner) | null);
  ownedByPtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsOwnable$Type extends TraitFacade {}
export const IsOwnable = new IsOwnable$Type(TraitType.OWNABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:500 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5000 ==== */
/**
 * A Node that defines Settings.
 */
export interface IsSettings {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsSettings$Type extends TraitFacade {}
export const IsSettings = new IsSettings$Type(TraitType.SETTINGS);
/* ==== DESTACK_GENERATED_END:TRAIT:5000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:502 ==== */
/**
 * A Node that can be joined by Subjects.
 */
export interface IsJoinable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsJoinable$Type extends TraitFacade {}
export const IsJoinable = new IsJoinable$Type(TraitType.JOINABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:502 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:505 ==== */
/**
 * A Node that can be a Subject.
 */
export interface IsSubject {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsSubject$Type extends TraitFacade {}
export const IsSubject = new IsSubject$Type(TraitType.SUBJECT);
/* ==== DESTACK_GENERATED_END:TRAIT:505 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:506 ==== */
/**
 * A Node that can be an Owner.
 */
export interface IsOwner {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsOwner$Type extends TraitFacade {}
export const IsOwner = new IsOwner$Type(TraitType.OWNER);
/* ==== DESTACK_GENERATED_END:TRAIT:506 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1000 ==== */
/**
 * A Node that can be tagged (with a Tag).
 */
export interface IsTaggable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class IsTaggable$Type extends TraitFacade {}
export const IsTaggable = new IsTaggable$Type(TraitType.TAGGABLE);
/* ==== DESTACK_GENERATED_END:TRAIT:1000 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:510 ==== */
/**
 * A Node that represents a Membership.
 */
export interface LikeMembership {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class LikeMembership$Type extends TraitFacade {}
export const LikeMembership = new LikeMembership$Type(TraitType.MEMBERSHIP);
/* ==== DESTACK_GENERATED_END:TRAIT:510 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:511 ==== */
/**
 * A Node that represents an Invite.
 */
export interface LikeInvite {
  get member(): (Node & IsSubject) | null;
  set member(value: Node & IsSubject);
  memberPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class LikeInvite$Type extends TraitFacade {}
export const LikeInvite = new LikeInvite$Type(TraitType.INVITE);
/* ==== DESTACK_GENERATED_END:TRAIT:511 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1001 ==== */
/**
 * A Node that represents a Tag.
 */
export interface LikeTag {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class LikeTag$Type extends TraitFacade {}
export const LikeTag = new LikeTag$Type(TraitType.TAG);
/* ==== DESTACK_GENERATED_END:TRAIT:1001 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:5535 ==== */
/**
 * A Node that represents a Follow.
 */
export interface LikeFollow {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class LikeFollow$Type extends TraitFacade {}
export const LikeFollow = new LikeFollow$Type(TraitType.FOLLOW);
/* ==== DESTACK_GENERATED_END:TRAIT:5535 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:1 ==== */
/**
 * A Node that is global.
 */
export interface Global {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Global$Type extends TraitFacade {}
export const Global = new Global$Type(TraitType.GLOBAL);
/* ==== DESTACK_GENERATED_END:TRAIT:1 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:2 ==== */
/**
 * A Node in a Space.
 */
export interface Spatial {
  get space(): Space | null;
  readonly spacePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Spatial$Type extends TraitFacade {}
export const Spatial = new Spatial$Type(TraitType.SPATIAL);
/* ==== DESTACK_GENERATED_END:TRAIT:2 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:10 ==== */
/**
 * An Entity is a versioned Node in primary relational storage (OLTP).
 */
export interface Entity extends IsTracked {
  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Entity$Type extends TraitFacade {}
export const Entity = new Entity$Type(TraitType.ENTITY);
/* ==== DESTACK_GENERATED_END:TRAIT:10 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:11 ==== */
/**
 * A Particle is a forward-only Node in primary document storage (OLTP, high volume).
 */
export interface Particle extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Particle$Type extends TraitFacade {}
export const Particle = new Particle$Type(TraitType.PARTICLE);
/* ==== DESTACK_GENERATED_END:TRAIT:11 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:12 ==== */
/**
 * An Analytic is a read-only Node in primary or secondary warehouse storage (OLAP, bulk).
 */
export interface Analytic extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Analytic$Type extends TraitFacade {}
export const Analytic = new Analytic$Type(TraitType.ANALYTIC);
/* ==== DESTACK_GENERATED_END:TRAIT:12 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:13 ==== */
/**
 * A Node that is indexed in secondary search storage (OLTP).
 */
export interface Indexed extends IsTracked {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Indexed$Type extends TraitFacade {}
export const Indexed = new Indexed$Type(TraitType.INDEXED);
/* ==== DESTACK_GENERATED_END:TRAIT:13 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:21 ==== */
/**
 * A Resource represents an external asset.
 * The lifecycle of a Resource may be managed by some provisioner.
 */
export interface Resource extends Entity {
  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Resource$Type extends TraitFacade {}
export const Resource = new Resource$Type(TraitType.RESOURCE);
/* ==== DESTACK_GENERATED_END:TRAIT:21 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4010 ==== */
/**
 * An Entity that represents a Metric.
 */
export interface Metric extends Entity, IsCustomNodeDefinition, IsSourceable {
  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Metric$Type extends TraitFacade {}
export const Metric = new Metric$Type(TraitType.METRIC);
/* ==== DESTACK_GENERATED_END:TRAIT:4010 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:4011 ==== */
/**
 * An Analytic that represents a Measurement.
 */
export interface Measurement extends Analytic, IsCustomNode {
  get definition(): (Node & Metric) | null;
  readonly definitionPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Measurement$Type extends TraitFacade {}
export const Measurement = new Measurement$Type(TraitType.MEASUREMENT);
/* ==== DESTACK_GENERATED_END:TRAIT:4011 ==== */

/* ==== DESTACK_GENERATED_START:TRAIT:22 ==== */
/**
 * An Event is a Node that represents an Event.
 * Events always belong to a specific Space.
 */
export interface Event extends Spatial, Particle, Analytic, Indexed, IsFrozen {
  get node(): Node | null;
  set node(value: Node | null);
  nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}

class Event$Type extends TraitFacade {}
export const Event = new Event$Type(TraitType.EVENT);
/* ==== DESTACK_GENERATED_END:TRAIT:22 ==== */
