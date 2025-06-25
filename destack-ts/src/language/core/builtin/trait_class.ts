import { NodeClass, NodeType, TraitType, WithSubqueries, toSubqueries } from "@destack/language/core/builtin";
import {
  Aggregation,
  AggregationType,
  Condition,
  Expression,
  ExpressionIn,
  Join,
  PropertyDefinition,
  Query,
  QueryType,
  RelationReference,
  Sort,
} from "@destack/language/core/common";
import { Casing, toCasing } from "@destack/utils";

/** Internal base class for Trait companion objects.*/
export class TraitClass {
  readonly metatype: TraitType;
  readonly __traits__: TraitType[];
  readonly __properties__: Record<string, PropertyDefinition>;
  readonly __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: TraitType) {
    this.metatype = metatype;
    this.__traits__ = [];
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
