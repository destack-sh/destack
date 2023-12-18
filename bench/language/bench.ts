/* eslint-disable */
import * as _m0 from "protobufjs/minimal";
import { Struct } from "../../google/protobuf/struct";
import { Timestamp } from "../../google/protobuf/timestamp";
import Long = require("long");

export const protobufPackage = "";

export enum AggregationOp {
  AGGREGATION_OP_UNSET = 0,
  AGGREGATION_OP_COUNT = 1,
  AGGREGATION_OP_SUM = 2,
  AGGREGATION_OP_AVERAGE = 3,
  AGGREGATION_OP_MIN = 4,
  AGGREGATION_OP_MAX = 5,
  AGGREGATION_OP_MEDIAN = 6,
  AGGREGATION_OP_HISTOGRAM = 7,
  UNRECOGNIZED = -1,
}

export function aggregationOpFromJSON(object: any): AggregationOp {
  switch (object) {
    case 0:
    case "AGGREGATION_OP_UNSET":
      return AggregationOp.AGGREGATION_OP_UNSET;
    case 1:
    case "AGGREGATION_OP_COUNT":
      return AggregationOp.AGGREGATION_OP_COUNT;
    case 2:
    case "AGGREGATION_OP_SUM":
      return AggregationOp.AGGREGATION_OP_SUM;
    case 3:
    case "AGGREGATION_OP_AVERAGE":
      return AggregationOp.AGGREGATION_OP_AVERAGE;
    case 4:
    case "AGGREGATION_OP_MIN":
      return AggregationOp.AGGREGATION_OP_MIN;
    case 5:
    case "AGGREGATION_OP_MAX":
      return AggregationOp.AGGREGATION_OP_MAX;
    case 6:
    case "AGGREGATION_OP_MEDIAN":
      return AggregationOp.AGGREGATION_OP_MEDIAN;
    case 7:
    case "AGGREGATION_OP_HISTOGRAM":
      return AggregationOp.AGGREGATION_OP_HISTOGRAM;
    case -1:
    case "UNRECOGNIZED":
    default:
      return AggregationOp.UNRECOGNIZED;
  }
}

export function aggregationOpToJSON(object: AggregationOp): string {
  switch (object) {
    case AggregationOp.AGGREGATION_OP_UNSET:
      return "AGGREGATION_OP_UNSET";
    case AggregationOp.AGGREGATION_OP_COUNT:
      return "AGGREGATION_OP_COUNT";
    case AggregationOp.AGGREGATION_OP_SUM:
      return "AGGREGATION_OP_SUM";
    case AggregationOp.AGGREGATION_OP_AVERAGE:
      return "AGGREGATION_OP_AVERAGE";
    case AggregationOp.AGGREGATION_OP_MIN:
      return "AGGREGATION_OP_MIN";
    case AggregationOp.AGGREGATION_OP_MAX:
      return "AGGREGATION_OP_MAX";
    case AggregationOp.AGGREGATION_OP_MEDIAN:
      return "AGGREGATION_OP_MEDIAN";
    case AggregationOp.AGGREGATION_OP_HISTOGRAM:
      return "AGGREGATION_OP_HISTOGRAM";
    case AggregationOp.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum BlobStatus {
  BLOB_STATUS_UNSET = 0,
  BLOB_STATUS_PREPARED = 1,
  BLOB_STATUS_UPLOADING = 2,
  BLOB_STATUS_AVAILABLE = 3,
  UNRECOGNIZED = -1,
}

export function blobStatusFromJSON(object: any): BlobStatus {
  switch (object) {
    case 0:
    case "BLOB_STATUS_UNSET":
      return BlobStatus.BLOB_STATUS_UNSET;
    case 1:
    case "BLOB_STATUS_PREPARED":
      return BlobStatus.BLOB_STATUS_PREPARED;
    case 2:
    case "BLOB_STATUS_UPLOADING":
      return BlobStatus.BLOB_STATUS_UPLOADING;
    case 3:
    case "BLOB_STATUS_AVAILABLE":
      return BlobStatus.BLOB_STATUS_AVAILABLE;
    case -1:
    case "UNRECOGNIZED":
    default:
      return BlobStatus.UNRECOGNIZED;
  }
}

export function blobStatusToJSON(object: BlobStatus): string {
  switch (object) {
    case BlobStatus.BLOB_STATUS_UNSET:
      return "BLOB_STATUS_UNSET";
    case BlobStatus.BLOB_STATUS_PREPARED:
      return "BLOB_STATUS_PREPARED";
    case BlobStatus.BLOB_STATUS_UPLOADING:
      return "BLOB_STATUS_UPLOADING";
    case BlobStatus.BLOB_STATUS_AVAILABLE:
      return "BLOB_STATUS_AVAILABLE";
    case BlobStatus.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ConditionalOp {
  CONDITIONAL_OP_UNSET = 0,
  CONDITIONAL_OP_TRUE = 1,
  CONDITIONAL_OP_FALSE = 2,
  CONDITIONAL_OP_NOT = 3,
  CONDITIONAL_OP_AND = 4,
  CONDITIONAL_OP_OR = 5,
  CONDITIONAL_OP_EQUALS = 6,
  CONDITIONAL_OP_NOT_EQUALS = 7,
  CONDITIONAL_OP_GREATER_THAN = 8,
  CONDITIONAL_OP_GREATER_THAN_OR_EQUALS = 9,
  CONDITIONAL_OP_LESS_THAN = 10,
  CONDITIONAL_OP_LESS_THAN_OR_EQUALS = 11,
  CONDITIONAL_OP_MATCHES = 12,
  CONDITIONAL_OP_STARTS_WITH = 13,
  CONDITIONAL_OP_CONTAINS = 14,
  CONDITIONAL_OP_NOT_CONTAINS = 15,
  CONDITIONAL_OP_IN = 16,
  CONDITIONAL_OP_NOT_IN = 17,
  CONDITIONAL_OP_EXISTS = 18,
  CONDITIONAL_OP_NOT_EXISTS = 19,
  CONDITIONAL_OP_NEAR = 20,
  UNRECOGNIZED = -1,
}

export function conditionalOpFromJSON(object: any): ConditionalOp {
  switch (object) {
    case 0:
    case "CONDITIONAL_OP_UNSET":
      return ConditionalOp.CONDITIONAL_OP_UNSET;
    case 1:
    case "CONDITIONAL_OP_TRUE":
      return ConditionalOp.CONDITIONAL_OP_TRUE;
    case 2:
    case "CONDITIONAL_OP_FALSE":
      return ConditionalOp.CONDITIONAL_OP_FALSE;
    case 3:
    case "CONDITIONAL_OP_NOT":
      return ConditionalOp.CONDITIONAL_OP_NOT;
    case 4:
    case "CONDITIONAL_OP_AND":
      return ConditionalOp.CONDITIONAL_OP_AND;
    case 5:
    case "CONDITIONAL_OP_OR":
      return ConditionalOp.CONDITIONAL_OP_OR;
    case 6:
    case "CONDITIONAL_OP_EQUALS":
      return ConditionalOp.CONDITIONAL_OP_EQUALS;
    case 7:
    case "CONDITIONAL_OP_NOT_EQUALS":
      return ConditionalOp.CONDITIONAL_OP_NOT_EQUALS;
    case 8:
    case "CONDITIONAL_OP_GREATER_THAN":
      return ConditionalOp.CONDITIONAL_OP_GREATER_THAN;
    case 9:
    case "CONDITIONAL_OP_GREATER_THAN_OR_EQUALS":
      return ConditionalOp.CONDITIONAL_OP_GREATER_THAN_OR_EQUALS;
    case 10:
    case "CONDITIONAL_OP_LESS_THAN":
      return ConditionalOp.CONDITIONAL_OP_LESS_THAN;
    case 11:
    case "CONDITIONAL_OP_LESS_THAN_OR_EQUALS":
      return ConditionalOp.CONDITIONAL_OP_LESS_THAN_OR_EQUALS;
    case 12:
    case "CONDITIONAL_OP_MATCHES":
      return ConditionalOp.CONDITIONAL_OP_MATCHES;
    case 13:
    case "CONDITIONAL_OP_STARTS_WITH":
      return ConditionalOp.CONDITIONAL_OP_STARTS_WITH;
    case 14:
    case "CONDITIONAL_OP_CONTAINS":
      return ConditionalOp.CONDITIONAL_OP_CONTAINS;
    case 15:
    case "CONDITIONAL_OP_NOT_CONTAINS":
      return ConditionalOp.CONDITIONAL_OP_NOT_CONTAINS;
    case 16:
    case "CONDITIONAL_OP_IN":
      return ConditionalOp.CONDITIONAL_OP_IN;
    case 17:
    case "CONDITIONAL_OP_NOT_IN":
      return ConditionalOp.CONDITIONAL_OP_NOT_IN;
    case 18:
    case "CONDITIONAL_OP_EXISTS":
      return ConditionalOp.CONDITIONAL_OP_EXISTS;
    case 19:
    case "CONDITIONAL_OP_NOT_EXISTS":
      return ConditionalOp.CONDITIONAL_OP_NOT_EXISTS;
    case 20:
    case "CONDITIONAL_OP_NEAR":
      return ConditionalOp.CONDITIONAL_OP_NEAR;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ConditionalOp.UNRECOGNIZED;
  }
}

export function conditionalOpToJSON(object: ConditionalOp): string {
  switch (object) {
    case ConditionalOp.CONDITIONAL_OP_UNSET:
      return "CONDITIONAL_OP_UNSET";
    case ConditionalOp.CONDITIONAL_OP_TRUE:
      return "CONDITIONAL_OP_TRUE";
    case ConditionalOp.CONDITIONAL_OP_FALSE:
      return "CONDITIONAL_OP_FALSE";
    case ConditionalOp.CONDITIONAL_OP_NOT:
      return "CONDITIONAL_OP_NOT";
    case ConditionalOp.CONDITIONAL_OP_AND:
      return "CONDITIONAL_OP_AND";
    case ConditionalOp.CONDITIONAL_OP_OR:
      return "CONDITIONAL_OP_OR";
    case ConditionalOp.CONDITIONAL_OP_EQUALS:
      return "CONDITIONAL_OP_EQUALS";
    case ConditionalOp.CONDITIONAL_OP_NOT_EQUALS:
      return "CONDITIONAL_OP_NOT_EQUALS";
    case ConditionalOp.CONDITIONAL_OP_GREATER_THAN:
      return "CONDITIONAL_OP_GREATER_THAN";
    case ConditionalOp.CONDITIONAL_OP_GREATER_THAN_OR_EQUALS:
      return "CONDITIONAL_OP_GREATER_THAN_OR_EQUALS";
    case ConditionalOp.CONDITIONAL_OP_LESS_THAN:
      return "CONDITIONAL_OP_LESS_THAN";
    case ConditionalOp.CONDITIONAL_OP_LESS_THAN_OR_EQUALS:
      return "CONDITIONAL_OP_LESS_THAN_OR_EQUALS";
    case ConditionalOp.CONDITIONAL_OP_MATCHES:
      return "CONDITIONAL_OP_MATCHES";
    case ConditionalOp.CONDITIONAL_OP_STARTS_WITH:
      return "CONDITIONAL_OP_STARTS_WITH";
    case ConditionalOp.CONDITIONAL_OP_CONTAINS:
      return "CONDITIONAL_OP_CONTAINS";
    case ConditionalOp.CONDITIONAL_OP_NOT_CONTAINS:
      return "CONDITIONAL_OP_NOT_CONTAINS";
    case ConditionalOp.CONDITIONAL_OP_IN:
      return "CONDITIONAL_OP_IN";
    case ConditionalOp.CONDITIONAL_OP_NOT_IN:
      return "CONDITIONAL_OP_NOT_IN";
    case ConditionalOp.CONDITIONAL_OP_EXISTS:
      return "CONDITIONAL_OP_EXISTS";
    case ConditionalOp.CONDITIONAL_OP_NOT_EXISTS:
      return "CONDITIONAL_OP_NOT_EXISTS";
    case ConditionalOp.CONDITIONAL_OP_NEAR:
      return "CONDITIONAL_OP_NEAR";
    case ConditionalOp.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ExpressionKind {
  EXPRESSION_KIND_UNSET = 0,
  EXPRESSION_KIND_CONDITIONAL = 1,
  EXPRESSION_KIND_SORT = 2,
  EXPRESSION_KIND_AGGREGATION = 3,
  UNRECOGNIZED = -1,
}

export function expressionKindFromJSON(object: any): ExpressionKind {
  switch (object) {
    case 0:
    case "EXPRESSION_KIND_UNSET":
      return ExpressionKind.EXPRESSION_KIND_UNSET;
    case 1:
    case "EXPRESSION_KIND_CONDITIONAL":
      return ExpressionKind.EXPRESSION_KIND_CONDITIONAL;
    case 2:
    case "EXPRESSION_KIND_SORT":
      return ExpressionKind.EXPRESSION_KIND_SORT;
    case 3:
    case "EXPRESSION_KIND_AGGREGATION":
      return ExpressionKind.EXPRESSION_KIND_AGGREGATION;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ExpressionKind.UNRECOGNIZED;
  }
}

export function expressionKindToJSON(object: ExpressionKind): string {
  switch (object) {
    case ExpressionKind.EXPRESSION_KIND_UNSET:
      return "EXPRESSION_KIND_UNSET";
    case ExpressionKind.EXPRESSION_KIND_CONDITIONAL:
      return "EXPRESSION_KIND_CONDITIONAL";
    case ExpressionKind.EXPRESSION_KIND_SORT:
      return "EXPRESSION_KIND_SORT";
    case ExpressionKind.EXPRESSION_KIND_AGGREGATION:
      return "EXPRESSION_KIND_AGGREGATION";
    case ExpressionKind.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ExpressionOp {
  EXPRESSION_OP_UNSET = 0,
  EXPRESSION_OP_TRUE = 1,
  EXPRESSION_OP_FALSE = 2,
  EXPRESSION_OP_NOT = 3,
  EXPRESSION_OP_AND = 4,
  EXPRESSION_OP_OR = 5,
  EXPRESSION_OP_EQUALS = 6,
  EXPRESSION_OP_NOT_EQUALS = 7,
  EXPRESSION_OP_GREATER_THAN = 8,
  EXPRESSION_OP_GREATER_THAN_OR_EQUALS = 9,
  EXPRESSION_OP_LESS_THAN = 10,
  EXPRESSION_OP_LESS_THAN_OR_EQUALS = 11,
  EXPRESSION_OP_MATCHES = 12,
  EXPRESSION_OP_STARTS_WITH = 13,
  EXPRESSION_OP_CONTAINS = 14,
  EXPRESSION_OP_NOT_CONTAINS = 15,
  EXPRESSION_OP_IN = 16,
  EXPRESSION_OP_NOT_IN = 17,
  EXPRESSION_OP_EXISTS = 18,
  EXPRESSION_OP_NOT_EXISTS = 19,
  EXPRESSION_OP_NEAR = 20,
  EXPRESSION_OP_COUNT = 21,
  EXPRESSION_OP_SUM = 22,
  EXPRESSION_OP_AVERAGE = 23,
  EXPRESSION_OP_MIN = 24,
  EXPRESSION_OP_MAX = 25,
  EXPRESSION_OP_MEDIAN = 26,
  EXPRESSION_OP_HISTOGRAM = 27,
  EXPRESSION_OP_ASCENDING = 28,
  EXPRESSION_OP_DESCENDING = 29,
  UNRECOGNIZED = -1,
}

export function expressionOpFromJSON(object: any): ExpressionOp {
  switch (object) {
    case 0:
    case "EXPRESSION_OP_UNSET":
      return ExpressionOp.EXPRESSION_OP_UNSET;
    case 1:
    case "EXPRESSION_OP_TRUE":
      return ExpressionOp.EXPRESSION_OP_TRUE;
    case 2:
    case "EXPRESSION_OP_FALSE":
      return ExpressionOp.EXPRESSION_OP_FALSE;
    case 3:
    case "EXPRESSION_OP_NOT":
      return ExpressionOp.EXPRESSION_OP_NOT;
    case 4:
    case "EXPRESSION_OP_AND":
      return ExpressionOp.EXPRESSION_OP_AND;
    case 5:
    case "EXPRESSION_OP_OR":
      return ExpressionOp.EXPRESSION_OP_OR;
    case 6:
    case "EXPRESSION_OP_EQUALS":
      return ExpressionOp.EXPRESSION_OP_EQUALS;
    case 7:
    case "EXPRESSION_OP_NOT_EQUALS":
      return ExpressionOp.EXPRESSION_OP_NOT_EQUALS;
    case 8:
    case "EXPRESSION_OP_GREATER_THAN":
      return ExpressionOp.EXPRESSION_OP_GREATER_THAN;
    case 9:
    case "EXPRESSION_OP_GREATER_THAN_OR_EQUALS":
      return ExpressionOp.EXPRESSION_OP_GREATER_THAN_OR_EQUALS;
    case 10:
    case "EXPRESSION_OP_LESS_THAN":
      return ExpressionOp.EXPRESSION_OP_LESS_THAN;
    case 11:
    case "EXPRESSION_OP_LESS_THAN_OR_EQUALS":
      return ExpressionOp.EXPRESSION_OP_LESS_THAN_OR_EQUALS;
    case 12:
    case "EXPRESSION_OP_MATCHES":
      return ExpressionOp.EXPRESSION_OP_MATCHES;
    case 13:
    case "EXPRESSION_OP_STARTS_WITH":
      return ExpressionOp.EXPRESSION_OP_STARTS_WITH;
    case 14:
    case "EXPRESSION_OP_CONTAINS":
      return ExpressionOp.EXPRESSION_OP_CONTAINS;
    case 15:
    case "EXPRESSION_OP_NOT_CONTAINS":
      return ExpressionOp.EXPRESSION_OP_NOT_CONTAINS;
    case 16:
    case "EXPRESSION_OP_IN":
      return ExpressionOp.EXPRESSION_OP_IN;
    case 17:
    case "EXPRESSION_OP_NOT_IN":
      return ExpressionOp.EXPRESSION_OP_NOT_IN;
    case 18:
    case "EXPRESSION_OP_EXISTS":
      return ExpressionOp.EXPRESSION_OP_EXISTS;
    case 19:
    case "EXPRESSION_OP_NOT_EXISTS":
      return ExpressionOp.EXPRESSION_OP_NOT_EXISTS;
    case 20:
    case "EXPRESSION_OP_NEAR":
      return ExpressionOp.EXPRESSION_OP_NEAR;
    case 21:
    case "EXPRESSION_OP_COUNT":
      return ExpressionOp.EXPRESSION_OP_COUNT;
    case 22:
    case "EXPRESSION_OP_SUM":
      return ExpressionOp.EXPRESSION_OP_SUM;
    case 23:
    case "EXPRESSION_OP_AVERAGE":
      return ExpressionOp.EXPRESSION_OP_AVERAGE;
    case 24:
    case "EXPRESSION_OP_MIN":
      return ExpressionOp.EXPRESSION_OP_MIN;
    case 25:
    case "EXPRESSION_OP_MAX":
      return ExpressionOp.EXPRESSION_OP_MAX;
    case 26:
    case "EXPRESSION_OP_MEDIAN":
      return ExpressionOp.EXPRESSION_OP_MEDIAN;
    case 27:
    case "EXPRESSION_OP_HISTOGRAM":
      return ExpressionOp.EXPRESSION_OP_HISTOGRAM;
    case 28:
    case "EXPRESSION_OP_ASCENDING":
      return ExpressionOp.EXPRESSION_OP_ASCENDING;
    case 29:
    case "EXPRESSION_OP_DESCENDING":
      return ExpressionOp.EXPRESSION_OP_DESCENDING;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ExpressionOp.UNRECOGNIZED;
  }
}

export function expressionOpToJSON(object: ExpressionOp): string {
  switch (object) {
    case ExpressionOp.EXPRESSION_OP_UNSET:
      return "EXPRESSION_OP_UNSET";
    case ExpressionOp.EXPRESSION_OP_TRUE:
      return "EXPRESSION_OP_TRUE";
    case ExpressionOp.EXPRESSION_OP_FALSE:
      return "EXPRESSION_OP_FALSE";
    case ExpressionOp.EXPRESSION_OP_NOT:
      return "EXPRESSION_OP_NOT";
    case ExpressionOp.EXPRESSION_OP_AND:
      return "EXPRESSION_OP_AND";
    case ExpressionOp.EXPRESSION_OP_OR:
      return "EXPRESSION_OP_OR";
    case ExpressionOp.EXPRESSION_OP_EQUALS:
      return "EXPRESSION_OP_EQUALS";
    case ExpressionOp.EXPRESSION_OP_NOT_EQUALS:
      return "EXPRESSION_OP_NOT_EQUALS";
    case ExpressionOp.EXPRESSION_OP_GREATER_THAN:
      return "EXPRESSION_OP_GREATER_THAN";
    case ExpressionOp.EXPRESSION_OP_GREATER_THAN_OR_EQUALS:
      return "EXPRESSION_OP_GREATER_THAN_OR_EQUALS";
    case ExpressionOp.EXPRESSION_OP_LESS_THAN:
      return "EXPRESSION_OP_LESS_THAN";
    case ExpressionOp.EXPRESSION_OP_LESS_THAN_OR_EQUALS:
      return "EXPRESSION_OP_LESS_THAN_OR_EQUALS";
    case ExpressionOp.EXPRESSION_OP_MATCHES:
      return "EXPRESSION_OP_MATCHES";
    case ExpressionOp.EXPRESSION_OP_STARTS_WITH:
      return "EXPRESSION_OP_STARTS_WITH";
    case ExpressionOp.EXPRESSION_OP_CONTAINS:
      return "EXPRESSION_OP_CONTAINS";
    case ExpressionOp.EXPRESSION_OP_NOT_CONTAINS:
      return "EXPRESSION_OP_NOT_CONTAINS";
    case ExpressionOp.EXPRESSION_OP_IN:
      return "EXPRESSION_OP_IN";
    case ExpressionOp.EXPRESSION_OP_NOT_IN:
      return "EXPRESSION_OP_NOT_IN";
    case ExpressionOp.EXPRESSION_OP_EXISTS:
      return "EXPRESSION_OP_EXISTS";
    case ExpressionOp.EXPRESSION_OP_NOT_EXISTS:
      return "EXPRESSION_OP_NOT_EXISTS";
    case ExpressionOp.EXPRESSION_OP_NEAR:
      return "EXPRESSION_OP_NEAR";
    case ExpressionOp.EXPRESSION_OP_COUNT:
      return "EXPRESSION_OP_COUNT";
    case ExpressionOp.EXPRESSION_OP_SUM:
      return "EXPRESSION_OP_SUM";
    case ExpressionOp.EXPRESSION_OP_AVERAGE:
      return "EXPRESSION_OP_AVERAGE";
    case ExpressionOp.EXPRESSION_OP_MIN:
      return "EXPRESSION_OP_MIN";
    case ExpressionOp.EXPRESSION_OP_MAX:
      return "EXPRESSION_OP_MAX";
    case ExpressionOp.EXPRESSION_OP_MEDIAN:
      return "EXPRESSION_OP_MEDIAN";
    case ExpressionOp.EXPRESSION_OP_HISTOGRAM:
      return "EXPRESSION_OP_HISTOGRAM";
    case ExpressionOp.EXPRESSION_OP_ASCENDING:
      return "EXPRESSION_OP_ASCENDING";
    case ExpressionOp.EXPRESSION_OP_DESCENDING:
      return "EXPRESSION_OP_DESCENDING";
    case ExpressionOp.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum IdentifierType {
  IDENTIFIER_TYPE_UNSET = 0,
  IDENTIFIER_TYPE_METHOD = 1,
  IDENTIFIER_TYPE_TYPE = 2,
  IDENTIFIER_TYPE_CONSTANT = 3,
  IDENTIFIER_TYPE_PATH = 4,
  IDENTIFIER_TYPE_VARIABLE = 5,
  IDENTIFIER_TYPE_FIELD = 6,
  UNRECOGNIZED = -1,
}

export function identifierTypeFromJSON(object: any): IdentifierType {
  switch (object) {
    case 0:
    case "IDENTIFIER_TYPE_UNSET":
      return IdentifierType.IDENTIFIER_TYPE_UNSET;
    case 1:
    case "IDENTIFIER_TYPE_METHOD":
      return IdentifierType.IDENTIFIER_TYPE_METHOD;
    case 2:
    case "IDENTIFIER_TYPE_TYPE":
      return IdentifierType.IDENTIFIER_TYPE_TYPE;
    case 3:
    case "IDENTIFIER_TYPE_CONSTANT":
      return IdentifierType.IDENTIFIER_TYPE_CONSTANT;
    case 4:
    case "IDENTIFIER_TYPE_PATH":
      return IdentifierType.IDENTIFIER_TYPE_PATH;
    case 5:
    case "IDENTIFIER_TYPE_VARIABLE":
      return IdentifierType.IDENTIFIER_TYPE_VARIABLE;
    case 6:
    case "IDENTIFIER_TYPE_FIELD":
      return IdentifierType.IDENTIFIER_TYPE_FIELD;
    case -1:
    case "UNRECOGNIZED":
    default:
      return IdentifierType.UNRECOGNIZED;
  }
}

export function identifierTypeToJSON(object: IdentifierType): string {
  switch (object) {
    case IdentifierType.IDENTIFIER_TYPE_UNSET:
      return "IDENTIFIER_TYPE_UNSET";
    case IdentifierType.IDENTIFIER_TYPE_METHOD:
      return "IDENTIFIER_TYPE_METHOD";
    case IdentifierType.IDENTIFIER_TYPE_TYPE:
      return "IDENTIFIER_TYPE_TYPE";
    case IdentifierType.IDENTIFIER_TYPE_CONSTANT:
      return "IDENTIFIER_TYPE_CONSTANT";
    case IdentifierType.IDENTIFIER_TYPE_PATH:
      return "IDENTIFIER_TYPE_PATH";
    case IdentifierType.IDENTIFIER_TYPE_VARIABLE:
      return "IDENTIFIER_TYPE_VARIABLE";
    case IdentifierType.IDENTIFIER_TYPE_FIELD:
      return "IDENTIFIER_TYPE_FIELD";
    case IdentifierType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum IssueKind {
  ISSUE_KIND_UNSET = 0,
  ISSUE_KIND_ERROR = 1,
  ISSUE_KIND_WARNING = 2,
  ISSUE_KIND_NOTICE = 3,
  UNRECOGNIZED = -1,
}

export function issueKindFromJSON(object: any): IssueKind {
  switch (object) {
    case 0:
    case "ISSUE_KIND_UNSET":
      return IssueKind.ISSUE_KIND_UNSET;
    case 1:
    case "ISSUE_KIND_ERROR":
      return IssueKind.ISSUE_KIND_ERROR;
    case 2:
    case "ISSUE_KIND_WARNING":
      return IssueKind.ISSUE_KIND_WARNING;
    case 3:
    case "ISSUE_KIND_NOTICE":
      return IssueKind.ISSUE_KIND_NOTICE;
    case -1:
    case "UNRECOGNIZED":
    default:
      return IssueKind.UNRECOGNIZED;
  }
}

export function issueKindToJSON(object: IssueKind): string {
  switch (object) {
    case IssueKind.ISSUE_KIND_UNSET:
      return "ISSUE_KIND_UNSET";
    case IssueKind.ISSUE_KIND_ERROR:
      return "ISSUE_KIND_ERROR";
    case IssueKind.ISSUE_KIND_WARNING:
      return "ISSUE_KIND_WARNING";
    case IssueKind.ISSUE_KIND_NOTICE:
      return "ISSUE_KIND_NOTICE";
    case IssueKind.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum IssueType {
  ISSUE_TYPE_UNSET = 0,
  ISSUE_TYPE_INTERNAL = 1,
  ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE = 2,
  ISSUE_TYPE_MISSING_REFERENCE = 3,
  ISSUE_TYPE_CIRCULAR_ANCESTRY = 4,
  ISSUE_TYPE_CIRCULAR_UNION = 5,
  ISSUE_TYPE_MISMATCHED_UNION = 6,
  ISSUE_TYPE_INVALID_DATA = 7,
  ISSUE_TYPE_AMBIGUOUS_DEFINITION = 8,
  ISSUE_TYPE_CODE_NOT_EXPORTABLE = 9,
  ISSUE_TYPE_CODE_NOT_CACHEABLE = 10,
  ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED = 11,
  ISSUE_TYPE_TASK_MISSING_IO = 12,
  ISSUE_TYPE_TASK_IS_STATIC = 13,
  UNRECOGNIZED = -1,
}

export function issueTypeFromJSON(object: any): IssueType {
  switch (object) {
    case 0:
    case "ISSUE_TYPE_UNSET":
      return IssueType.ISSUE_TYPE_UNSET;
    case 1:
    case "ISSUE_TYPE_INTERNAL":
      return IssueType.ISSUE_TYPE_INTERNAL;
    case 2:
    case "ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE":
      return IssueType.ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE;
    case 3:
    case "ISSUE_TYPE_MISSING_REFERENCE":
      return IssueType.ISSUE_TYPE_MISSING_REFERENCE;
    case 4:
    case "ISSUE_TYPE_CIRCULAR_ANCESTRY":
      return IssueType.ISSUE_TYPE_CIRCULAR_ANCESTRY;
    case 5:
    case "ISSUE_TYPE_CIRCULAR_UNION":
      return IssueType.ISSUE_TYPE_CIRCULAR_UNION;
    case 6:
    case "ISSUE_TYPE_MISMATCHED_UNION":
      return IssueType.ISSUE_TYPE_MISMATCHED_UNION;
    case 7:
    case "ISSUE_TYPE_INVALID_DATA":
      return IssueType.ISSUE_TYPE_INVALID_DATA;
    case 8:
    case "ISSUE_TYPE_AMBIGUOUS_DEFINITION":
      return IssueType.ISSUE_TYPE_AMBIGUOUS_DEFINITION;
    case 9:
    case "ISSUE_TYPE_CODE_NOT_EXPORTABLE":
      return IssueType.ISSUE_TYPE_CODE_NOT_EXPORTABLE;
    case 10:
    case "ISSUE_TYPE_CODE_NOT_CACHEABLE":
      return IssueType.ISSUE_TYPE_CODE_NOT_CACHEABLE;
    case 11:
    case "ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED":
      return IssueType.ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED;
    case 12:
    case "ISSUE_TYPE_TASK_MISSING_IO":
      return IssueType.ISSUE_TYPE_TASK_MISSING_IO;
    case 13:
    case "ISSUE_TYPE_TASK_IS_STATIC":
      return IssueType.ISSUE_TYPE_TASK_IS_STATIC;
    case -1:
    case "UNRECOGNIZED":
    default:
      return IssueType.UNRECOGNIZED;
  }
}

export function issueTypeToJSON(object: IssueType): string {
  switch (object) {
    case IssueType.ISSUE_TYPE_UNSET:
      return "ISSUE_TYPE_UNSET";
    case IssueType.ISSUE_TYPE_INTERNAL:
      return "ISSUE_TYPE_INTERNAL";
    case IssueType.ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE:
      return "ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE";
    case IssueType.ISSUE_TYPE_MISSING_REFERENCE:
      return "ISSUE_TYPE_MISSING_REFERENCE";
    case IssueType.ISSUE_TYPE_CIRCULAR_ANCESTRY:
      return "ISSUE_TYPE_CIRCULAR_ANCESTRY";
    case IssueType.ISSUE_TYPE_CIRCULAR_UNION:
      return "ISSUE_TYPE_CIRCULAR_UNION";
    case IssueType.ISSUE_TYPE_MISMATCHED_UNION:
      return "ISSUE_TYPE_MISMATCHED_UNION";
    case IssueType.ISSUE_TYPE_INVALID_DATA:
      return "ISSUE_TYPE_INVALID_DATA";
    case IssueType.ISSUE_TYPE_AMBIGUOUS_DEFINITION:
      return "ISSUE_TYPE_AMBIGUOUS_DEFINITION";
    case IssueType.ISSUE_TYPE_CODE_NOT_EXPORTABLE:
      return "ISSUE_TYPE_CODE_NOT_EXPORTABLE";
    case IssueType.ISSUE_TYPE_CODE_NOT_CACHEABLE:
      return "ISSUE_TYPE_CODE_NOT_CACHEABLE";
    case IssueType.ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED:
      return "ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED";
    case IssueType.ISSUE_TYPE_TASK_MISSING_IO:
      return "ISSUE_TYPE_TASK_MISSING_IO";
    case IssueType.ISSUE_TYPE_TASK_IS_STATIC:
      return "ISSUE_TYPE_TASK_IS_STATIC";
    case IssueType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum NodeTrackingLevel {
  NODE_TRACKING_LEVEL_NONE = 0,
  NODE_TRACKING_LEVEL_ANONYMOUS = 1,
  NODE_TRACKING_LEVEL_FULL = 2,
  UNRECOGNIZED = -1,
}

export function nodeTrackingLevelFromJSON(object: any): NodeTrackingLevel {
  switch (object) {
    case 0:
    case "NODE_TRACKING_LEVEL_NONE":
      return NodeTrackingLevel.NODE_TRACKING_LEVEL_NONE;
    case 1:
    case "NODE_TRACKING_LEVEL_ANONYMOUS":
      return NodeTrackingLevel.NODE_TRACKING_LEVEL_ANONYMOUS;
    case 2:
    case "NODE_TRACKING_LEVEL_FULL":
      return NodeTrackingLevel.NODE_TRACKING_LEVEL_FULL;
    case -1:
    case "UNRECOGNIZED":
    default:
      return NodeTrackingLevel.UNRECOGNIZED;
  }
}

export function nodeTrackingLevelToJSON(object: NodeTrackingLevel): string {
  switch (object) {
    case NodeTrackingLevel.NODE_TRACKING_LEVEL_NONE:
      return "NODE_TRACKING_LEVEL_NONE";
    case NodeTrackingLevel.NODE_TRACKING_LEVEL_ANONYMOUS:
      return "NODE_TRACKING_LEVEL_ANONYMOUS";
    case NodeTrackingLevel.NODE_TRACKING_LEVEL_FULL:
      return "NODE_TRACKING_LEVEL_FULL";
    case NodeTrackingLevel.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum NodeType {
  NODE_TYPE_UNSET = 0,
  NODE_TYPE_MODULE = 1,
  NODE_TYPE_FILE = 2,
  NODE_TYPE_STATEMENT = 3,
  NODE_TYPE_TRIGGER = 4,
  NODE_TYPE_TAGGING = 5,
  NODE_TYPE_FIELD = 6,
  NODE_TYPE_RECORD = 7,
  NODE_TYPE_VIEW = 8,
  NODE_TYPE_ISSUE = 9,
  NODE_TYPE_RESOLVED_FIELD = 10,
  NODE_TYPE_USER = 11,
  NODE_TYPE_COMMENT = 12,
  NODE_TYPE_ACCESS = 13,
  NODE_TYPE_BLOB = 14,
  NODE_TYPE_SECRET = 15,
  NODE_TYPE_SESSION = 16,
  NODE_TYPE_RUN = 17,
  UNRECOGNIZED = -1,
}

export function nodeTypeFromJSON(object: any): NodeType {
  switch (object) {
    case 0:
    case "NODE_TYPE_UNSET":
      return NodeType.NODE_TYPE_UNSET;
    case 1:
    case "NODE_TYPE_MODULE":
      return NodeType.NODE_TYPE_MODULE;
    case 2:
    case "NODE_TYPE_FILE":
      return NodeType.NODE_TYPE_FILE;
    case 3:
    case "NODE_TYPE_STATEMENT":
      return NodeType.NODE_TYPE_STATEMENT;
    case 4:
    case "NODE_TYPE_TRIGGER":
      return NodeType.NODE_TYPE_TRIGGER;
    case 5:
    case "NODE_TYPE_TAGGING":
      return NodeType.NODE_TYPE_TAGGING;
    case 6:
    case "NODE_TYPE_FIELD":
      return NodeType.NODE_TYPE_FIELD;
    case 7:
    case "NODE_TYPE_RECORD":
      return NodeType.NODE_TYPE_RECORD;
    case 8:
    case "NODE_TYPE_VIEW":
      return NodeType.NODE_TYPE_VIEW;
    case 9:
    case "NODE_TYPE_ISSUE":
      return NodeType.NODE_TYPE_ISSUE;
    case 10:
    case "NODE_TYPE_RESOLVED_FIELD":
      return NodeType.NODE_TYPE_RESOLVED_FIELD;
    case 11:
    case "NODE_TYPE_USER":
      return NodeType.NODE_TYPE_USER;
    case 12:
    case "NODE_TYPE_COMMENT":
      return NodeType.NODE_TYPE_COMMENT;
    case 13:
    case "NODE_TYPE_ACCESS":
      return NodeType.NODE_TYPE_ACCESS;
    case 14:
    case "NODE_TYPE_BLOB":
      return NodeType.NODE_TYPE_BLOB;
    case 15:
    case "NODE_TYPE_SECRET":
      return NodeType.NODE_TYPE_SECRET;
    case 16:
    case "NODE_TYPE_SESSION":
      return NodeType.NODE_TYPE_SESSION;
    case 17:
    case "NODE_TYPE_RUN":
      return NodeType.NODE_TYPE_RUN;
    case -1:
    case "UNRECOGNIZED":
    default:
      return NodeType.UNRECOGNIZED;
  }
}

export function nodeTypeToJSON(object: NodeType): string {
  switch (object) {
    case NodeType.NODE_TYPE_UNSET:
      return "NODE_TYPE_UNSET";
    case NodeType.NODE_TYPE_MODULE:
      return "NODE_TYPE_MODULE";
    case NodeType.NODE_TYPE_FILE:
      return "NODE_TYPE_FILE";
    case NodeType.NODE_TYPE_STATEMENT:
      return "NODE_TYPE_STATEMENT";
    case NodeType.NODE_TYPE_TRIGGER:
      return "NODE_TYPE_TRIGGER";
    case NodeType.NODE_TYPE_TAGGING:
      return "NODE_TYPE_TAGGING";
    case NodeType.NODE_TYPE_FIELD:
      return "NODE_TYPE_FIELD";
    case NodeType.NODE_TYPE_RECORD:
      return "NODE_TYPE_RECORD";
    case NodeType.NODE_TYPE_VIEW:
      return "NODE_TYPE_VIEW";
    case NodeType.NODE_TYPE_ISSUE:
      return "NODE_TYPE_ISSUE";
    case NodeType.NODE_TYPE_RESOLVED_FIELD:
      return "NODE_TYPE_RESOLVED_FIELD";
    case NodeType.NODE_TYPE_USER:
      return "NODE_TYPE_USER";
    case NodeType.NODE_TYPE_COMMENT:
      return "NODE_TYPE_COMMENT";
    case NodeType.NODE_TYPE_ACCESS:
      return "NODE_TYPE_ACCESS";
    case NodeType.NODE_TYPE_BLOB:
      return "NODE_TYPE_BLOB";
    case NodeType.NODE_TYPE_SECRET:
      return "NODE_TYPE_SECRET";
    case NodeType.NODE_TYPE_SESSION:
      return "NODE_TYPE_SESSION";
    case NodeType.NODE_TYPE_RUN:
      return "NODE_TYPE_RUN";
    case NodeType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ProjectRegion {
  PROJECT_REGION_UNSET = 0,
  PROJECT_REGION_US_WEST = 1,
  PROJECT_REGION_EU_CENTRAL = 2,
  UNRECOGNIZED = -1,
}

export function projectRegionFromJSON(object: any): ProjectRegion {
  switch (object) {
    case 0:
    case "PROJECT_REGION_UNSET":
      return ProjectRegion.PROJECT_REGION_UNSET;
    case 1:
    case "PROJECT_REGION_US_WEST":
      return ProjectRegion.PROJECT_REGION_US_WEST;
    case 2:
    case "PROJECT_REGION_EU_CENTRAL":
      return ProjectRegion.PROJECT_REGION_EU_CENTRAL;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ProjectRegion.UNRECOGNIZED;
  }
}

export function projectRegionToJSON(object: ProjectRegion): string {
  switch (object) {
    case ProjectRegion.PROJECT_REGION_UNSET:
      return "PROJECT_REGION_UNSET";
    case ProjectRegion.PROJECT_REGION_US_WEST:
      return "PROJECT_REGION_US_WEST";
    case ProjectRegion.PROJECT_REGION_EU_CENTRAL:
      return "PROJECT_REGION_EU_CENTRAL";
    case ProjectRegion.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum QueryEngine {
  QUERY_ENGINE_UNSET = 0,
  QUERY_ENGINE_MODULE = 1,
  QUERY_ENGINE_HOST = 2,
  QUERY_ENGINE_OPENSEARCH = 3,
  QUERY_ENGINE_POSTGRES = 4,
  UNRECOGNIZED = -1,
}

export function queryEngineFromJSON(object: any): QueryEngine {
  switch (object) {
    case 0:
    case "QUERY_ENGINE_UNSET":
      return QueryEngine.QUERY_ENGINE_UNSET;
    case 1:
    case "QUERY_ENGINE_MODULE":
      return QueryEngine.QUERY_ENGINE_MODULE;
    case 2:
    case "QUERY_ENGINE_HOST":
      return QueryEngine.QUERY_ENGINE_HOST;
    case 3:
    case "QUERY_ENGINE_OPENSEARCH":
      return QueryEngine.QUERY_ENGINE_OPENSEARCH;
    case 4:
    case "QUERY_ENGINE_POSTGRES":
      return QueryEngine.QUERY_ENGINE_POSTGRES;
    case -1:
    case "UNRECOGNIZED":
    default:
      return QueryEngine.UNRECOGNIZED;
  }
}

export function queryEngineToJSON(object: QueryEngine): string {
  switch (object) {
    case QueryEngine.QUERY_ENGINE_UNSET:
      return "QUERY_ENGINE_UNSET";
    case QueryEngine.QUERY_ENGINE_MODULE:
      return "QUERY_ENGINE_MODULE";
    case QueryEngine.QUERY_ENGINE_HOST:
      return "QUERY_ENGINE_HOST";
    case QueryEngine.QUERY_ENGINE_OPENSEARCH:
      return "QUERY_ENGINE_OPENSEARCH";
    case QueryEngine.QUERY_ENGINE_POSTGRES:
      return "QUERY_ENGINE_POSTGRES";
    case QueryEngine.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum RunErrorKind {
  RUN_ERROR_KIND_UNSET = 0,
  RUN_ERROR_KIND_Internal = 1,
  RUN_ERROR_KIND_Parse = 2,
  RUN_ERROR_KIND_Validation = 3,
  RUN_ERROR_KIND_Runtime = 4,
  RUN_ERROR_KIND_Untrusted = 5,
  UNRECOGNIZED = -1,
}

export function runErrorKindFromJSON(object: any): RunErrorKind {
  switch (object) {
    case 0:
    case "RUN_ERROR_KIND_UNSET":
      return RunErrorKind.RUN_ERROR_KIND_UNSET;
    case 1:
    case "RUN_ERROR_KIND_Internal":
      return RunErrorKind.RUN_ERROR_KIND_Internal;
    case 2:
    case "RUN_ERROR_KIND_Parse":
      return RunErrorKind.RUN_ERROR_KIND_Parse;
    case 3:
    case "RUN_ERROR_KIND_Validation":
      return RunErrorKind.RUN_ERROR_KIND_Validation;
    case 4:
    case "RUN_ERROR_KIND_Runtime":
      return RunErrorKind.RUN_ERROR_KIND_Runtime;
    case 5:
    case "RUN_ERROR_KIND_Untrusted":
      return RunErrorKind.RUN_ERROR_KIND_Untrusted;
    case -1:
    case "UNRECOGNIZED":
    default:
      return RunErrorKind.UNRECOGNIZED;
  }
}

export function runErrorKindToJSON(object: RunErrorKind): string {
  switch (object) {
    case RunErrorKind.RUN_ERROR_KIND_UNSET:
      return "RUN_ERROR_KIND_UNSET";
    case RunErrorKind.RUN_ERROR_KIND_Internal:
      return "RUN_ERROR_KIND_Internal";
    case RunErrorKind.RUN_ERROR_KIND_Parse:
      return "RUN_ERROR_KIND_Parse";
    case RunErrorKind.RUN_ERROR_KIND_Validation:
      return "RUN_ERROR_KIND_Validation";
    case RunErrorKind.RUN_ERROR_KIND_Runtime:
      return "RUN_ERROR_KIND_Runtime";
    case RunErrorKind.RUN_ERROR_KIND_Untrusted:
      return "RUN_ERROR_KIND_Untrusted";
    case RunErrorKind.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum RunStatus {
  RUN_STATUS_UNSET = 0,
  RUN_STATUS_SCHEDULED = 1,
  RUN_STATUS_QUEUED = 2,
  RUN_STATUS_RUNNING = 3,
  RUN_STATUS_SUSPENDED = 4,
  RUN_STATUS_ABORTING = 5,
  RUN_STATUS_CANCELLED = 6,
  RUN_STATUS_ABORTED = 7,
  RUN_STATUS_FAILED = 8,
  RUN_STATUS_COMPLETED = 9,
  UNRECOGNIZED = -1,
}

export function runStatusFromJSON(object: any): RunStatus {
  switch (object) {
    case 0:
    case "RUN_STATUS_UNSET":
      return RunStatus.RUN_STATUS_UNSET;
    case 1:
    case "RUN_STATUS_SCHEDULED":
      return RunStatus.RUN_STATUS_SCHEDULED;
    case 2:
    case "RUN_STATUS_QUEUED":
      return RunStatus.RUN_STATUS_QUEUED;
    case 3:
    case "RUN_STATUS_RUNNING":
      return RunStatus.RUN_STATUS_RUNNING;
    case 4:
    case "RUN_STATUS_SUSPENDED":
      return RunStatus.RUN_STATUS_SUSPENDED;
    case 5:
    case "RUN_STATUS_ABORTING":
      return RunStatus.RUN_STATUS_ABORTING;
    case 6:
    case "RUN_STATUS_CANCELLED":
      return RunStatus.RUN_STATUS_CANCELLED;
    case 7:
    case "RUN_STATUS_ABORTED":
      return RunStatus.RUN_STATUS_ABORTED;
    case 8:
    case "RUN_STATUS_FAILED":
      return RunStatus.RUN_STATUS_FAILED;
    case 9:
    case "RUN_STATUS_COMPLETED":
      return RunStatus.RUN_STATUS_COMPLETED;
    case -1:
    case "UNRECOGNIZED":
    default:
      return RunStatus.UNRECOGNIZED;
  }
}

export function runStatusToJSON(object: RunStatus): string {
  switch (object) {
    case RunStatus.RUN_STATUS_UNSET:
      return "RUN_STATUS_UNSET";
    case RunStatus.RUN_STATUS_SCHEDULED:
      return "RUN_STATUS_SCHEDULED";
    case RunStatus.RUN_STATUS_QUEUED:
      return "RUN_STATUS_QUEUED";
    case RunStatus.RUN_STATUS_RUNNING:
      return "RUN_STATUS_RUNNING";
    case RunStatus.RUN_STATUS_SUSPENDED:
      return "RUN_STATUS_SUSPENDED";
    case RunStatus.RUN_STATUS_ABORTING:
      return "RUN_STATUS_ABORTING";
    case RunStatus.RUN_STATUS_CANCELLED:
      return "RUN_STATUS_CANCELLED";
    case RunStatus.RUN_STATUS_ABORTED:
      return "RUN_STATUS_ABORTED";
    case RunStatus.RUN_STATUS_FAILED:
      return "RUN_STATUS_FAILED";
    case RunStatus.RUN_STATUS_COMPLETED:
      return "RUN_STATUS_COMPLETED";
    case RunStatus.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ScheduleType {
  SCHEDULE_TYPE_UNSET = 0,
  SCHEDULE_TYPE_INTERVAL = 1,
  SCHEDULE_TYPE_CRON = 2,
  UNRECOGNIZED = -1,
}

export function scheduleTypeFromJSON(object: any): ScheduleType {
  switch (object) {
    case 0:
    case "SCHEDULE_TYPE_UNSET":
      return ScheduleType.SCHEDULE_TYPE_UNSET;
    case 1:
    case "SCHEDULE_TYPE_INTERVAL":
      return ScheduleType.SCHEDULE_TYPE_INTERVAL;
    case 2:
    case "SCHEDULE_TYPE_CRON":
      return ScheduleType.SCHEDULE_TYPE_CRON;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ScheduleType.UNRECOGNIZED;
  }
}

export function scheduleTypeToJSON(object: ScheduleType): string {
  switch (object) {
    case ScheduleType.SCHEDULE_TYPE_UNSET:
      return "SCHEDULE_TYPE_UNSET";
    case ScheduleType.SCHEDULE_TYPE_INTERVAL:
      return "SCHEDULE_TYPE_INTERVAL";
    case ScheduleType.SCHEDULE_TYPE_CRON:
      return "SCHEDULE_TYPE_CRON";
    case ScheduleType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum SessionAccessLevel {
  SESSION_ACCESS_LEVEL_Zero = 0,
  SESSION_ACCESS_LEVEL_Read = 1,
  SESSION_ACCESS_LEVEL_Create = 2,
  SESSION_ACCESS_LEVEL_Update = 3,
  SESSION_ACCESS_LEVEL_Delete = 4,
  SESSION_ACCESS_LEVEL_Full = 4,
  UNRECOGNIZED = -1,
}

export function sessionAccessLevelFromJSON(object: any): SessionAccessLevel {
  switch (object) {
    case 0:
    case "SESSION_ACCESS_LEVEL_Zero":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Zero;
    case 1:
    case "SESSION_ACCESS_LEVEL_Read":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Read;
    case 2:
    case "SESSION_ACCESS_LEVEL_Create":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Create;
    case 3:
    case "SESSION_ACCESS_LEVEL_Update":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Update;
    case 4:
    case "SESSION_ACCESS_LEVEL_Delete":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Delete;
    case 4:
    case "SESSION_ACCESS_LEVEL_Full":
      return SessionAccessLevel.SESSION_ACCESS_LEVEL_Full;
    case -1:
    case "UNRECOGNIZED":
    default:
      return SessionAccessLevel.UNRECOGNIZED;
  }
}

export function sessionAccessLevelToJSON(object: SessionAccessLevel): string {
  switch (object) {
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Zero:
      return "SESSION_ACCESS_LEVEL_Zero";
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Read:
      return "SESSION_ACCESS_LEVEL_Read";
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Create:
      return "SESSION_ACCESS_LEVEL_Create";
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Update:
      return "SESSION_ACCESS_LEVEL_Update";
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Delete:
      return "SESSION_ACCESS_LEVEL_Delete";
    case SessionAccessLevel.SESSION_ACCESS_LEVEL_Full:
      return "SESSION_ACCESS_LEVEL_Full";
    case SessionAccessLevel.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum SessionStatus {
  SESSION_STATUS_UNSET = 0,
  SESSION_STATUS_ACTIVE = 1,
  SESSION_STATUS_SUSPENDED = 2,
  SESSION_STATUS_TERMINATED = 3,
  UNRECOGNIZED = -1,
}

export function sessionStatusFromJSON(object: any): SessionStatus {
  switch (object) {
    case 0:
    case "SESSION_STATUS_UNSET":
      return SessionStatus.SESSION_STATUS_UNSET;
    case 1:
    case "SESSION_STATUS_ACTIVE":
      return SessionStatus.SESSION_STATUS_ACTIVE;
    case 2:
    case "SESSION_STATUS_SUSPENDED":
      return SessionStatus.SESSION_STATUS_SUSPENDED;
    case 3:
    case "SESSION_STATUS_TERMINATED":
      return SessionStatus.SESSION_STATUS_TERMINATED;
    case -1:
    case "UNRECOGNIZED":
    default:
      return SessionStatus.UNRECOGNIZED;
  }
}

export function sessionStatusToJSON(object: SessionStatus): string {
  switch (object) {
    case SessionStatus.SESSION_STATUS_UNSET:
      return "SESSION_STATUS_UNSET";
    case SessionStatus.SESSION_STATUS_ACTIVE:
      return "SESSION_STATUS_ACTIVE";
    case SessionStatus.SESSION_STATUS_SUSPENDED:
      return "SESSION_STATUS_SUSPENDED";
    case SessionStatus.SESSION_STATUS_TERMINATED:
      return "SESSION_STATUS_TERMINATED";
    case SessionStatus.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum SortMode {
  SORT_MODE_UNSET = 0,
  SORT_MODE_MAX = 1,
  SORT_MODE_MIN = 2,
  SORT_MODE_AVERAGE = 3,
  SORT_MODE_SUM = 4,
  SORT_MODE_MEDIAN = 5,
  UNRECOGNIZED = -1,
}

export function sortModeFromJSON(object: any): SortMode {
  switch (object) {
    case 0:
    case "SORT_MODE_UNSET":
      return SortMode.SORT_MODE_UNSET;
    case 1:
    case "SORT_MODE_MAX":
      return SortMode.SORT_MODE_MAX;
    case 2:
    case "SORT_MODE_MIN":
      return SortMode.SORT_MODE_MIN;
    case 3:
    case "SORT_MODE_AVERAGE":
      return SortMode.SORT_MODE_AVERAGE;
    case 4:
    case "SORT_MODE_SUM":
      return SortMode.SORT_MODE_SUM;
    case 5:
    case "SORT_MODE_MEDIAN":
      return SortMode.SORT_MODE_MEDIAN;
    case -1:
    case "UNRECOGNIZED":
    default:
      return SortMode.UNRECOGNIZED;
  }
}

export function sortModeToJSON(object: SortMode): string {
  switch (object) {
    case SortMode.SORT_MODE_UNSET:
      return "SORT_MODE_UNSET";
    case SortMode.SORT_MODE_MAX:
      return "SORT_MODE_MAX";
    case SortMode.SORT_MODE_MIN:
      return "SORT_MODE_MIN";
    case SortMode.SORT_MODE_AVERAGE:
      return "SORT_MODE_AVERAGE";
    case SortMode.SORT_MODE_SUM:
      return "SORT_MODE_SUM";
    case SortMode.SORT_MODE_MEDIAN:
      return "SORT_MODE_MEDIAN";
    case SortMode.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum SortOp {
  SORT_OP_UNSET = 0,
  SORT_OP_ASCENDING = 1,
  SORT_OP_DESCENDING = 2,
  UNRECOGNIZED = -1,
}

export function sortOpFromJSON(object: any): SortOp {
  switch (object) {
    case 0:
    case "SORT_OP_UNSET":
      return SortOp.SORT_OP_UNSET;
    case 1:
    case "SORT_OP_ASCENDING":
      return SortOp.SORT_OP_ASCENDING;
    case 2:
    case "SORT_OP_DESCENDING":
      return SortOp.SORT_OP_DESCENDING;
    case -1:
    case "UNRECOGNIZED":
    default:
      return SortOp.UNRECOGNIZED;
  }
}

export function sortOpToJSON(object: SortOp): string {
  switch (object) {
    case SortOp.SORT_OP_UNSET:
      return "SORT_OP_UNSET";
    case SortOp.SORT_OP_ASCENDING:
      return "SORT_OP_ASCENDING";
    case SortOp.SORT_OP_DESCENDING:
      return "SORT_OP_DESCENDING";
    case SortOp.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum StatementType {
  STATEMENT_TYPE_UNSET = 0,
  STATEMENT_TYPE_TAG = 1,
  STATEMENT_TYPE_TEXT = 2,
  STATEMENT_TYPE_BLANK = 3,
  STATEMENT_TYPE_CLASS = 4,
  STATEMENT_TYPE_CHOICE = 5,
  STATEMENT_TYPE_TASK = 6,
  STATEMENT_TYPE_CODE = 7,
  STATEMENT_TYPE_FLOW = 8,
  STATEMENT_TYPE_MODEL = 9,
  STATEMENT_TYPE_VARIABLE = 10,
  STATEMENT_TYPE_DATABASE = 11,
  STATEMENT_TYPE_VIEW = 12,
  STATEMENT_TYPE_GROUP = 13,
  UNRECOGNIZED = -1,
}

export function statementTypeFromJSON(object: any): StatementType {
  switch (object) {
    case 0:
    case "STATEMENT_TYPE_UNSET":
      return StatementType.STATEMENT_TYPE_UNSET;
    case 1:
    case "STATEMENT_TYPE_TAG":
      return StatementType.STATEMENT_TYPE_TAG;
    case 2:
    case "STATEMENT_TYPE_TEXT":
      return StatementType.STATEMENT_TYPE_TEXT;
    case 3:
    case "STATEMENT_TYPE_BLANK":
      return StatementType.STATEMENT_TYPE_BLANK;
    case 4:
    case "STATEMENT_TYPE_CLASS":
      return StatementType.STATEMENT_TYPE_CLASS;
    case 5:
    case "STATEMENT_TYPE_CHOICE":
      return StatementType.STATEMENT_TYPE_CHOICE;
    case 6:
    case "STATEMENT_TYPE_TASK":
      return StatementType.STATEMENT_TYPE_TASK;
    case 7:
    case "STATEMENT_TYPE_CODE":
      return StatementType.STATEMENT_TYPE_CODE;
    case 8:
    case "STATEMENT_TYPE_FLOW":
      return StatementType.STATEMENT_TYPE_FLOW;
    case 9:
    case "STATEMENT_TYPE_MODEL":
      return StatementType.STATEMENT_TYPE_MODEL;
    case 10:
    case "STATEMENT_TYPE_VARIABLE":
      return StatementType.STATEMENT_TYPE_VARIABLE;
    case 11:
    case "STATEMENT_TYPE_DATABASE":
      return StatementType.STATEMENT_TYPE_DATABASE;
    case 12:
    case "STATEMENT_TYPE_VIEW":
      return StatementType.STATEMENT_TYPE_VIEW;
    case 13:
    case "STATEMENT_TYPE_GROUP":
      return StatementType.STATEMENT_TYPE_GROUP;
    case -1:
    case "UNRECOGNIZED":
    default:
      return StatementType.UNRECOGNIZED;
  }
}

export function statementTypeToJSON(object: StatementType): string {
  switch (object) {
    case StatementType.STATEMENT_TYPE_UNSET:
      return "STATEMENT_TYPE_UNSET";
    case StatementType.STATEMENT_TYPE_TAG:
      return "STATEMENT_TYPE_TAG";
    case StatementType.STATEMENT_TYPE_TEXT:
      return "STATEMENT_TYPE_TEXT";
    case StatementType.STATEMENT_TYPE_BLANK:
      return "STATEMENT_TYPE_BLANK";
    case StatementType.STATEMENT_TYPE_CLASS:
      return "STATEMENT_TYPE_CLASS";
    case StatementType.STATEMENT_TYPE_CHOICE:
      return "STATEMENT_TYPE_CHOICE";
    case StatementType.STATEMENT_TYPE_TASK:
      return "STATEMENT_TYPE_TASK";
    case StatementType.STATEMENT_TYPE_CODE:
      return "STATEMENT_TYPE_CODE";
    case StatementType.STATEMENT_TYPE_FLOW:
      return "STATEMENT_TYPE_FLOW";
    case StatementType.STATEMENT_TYPE_MODEL:
      return "STATEMENT_TYPE_MODEL";
    case StatementType.STATEMENT_TYPE_VARIABLE:
      return "STATEMENT_TYPE_VARIABLE";
    case StatementType.STATEMENT_TYPE_DATABASE:
      return "STATEMENT_TYPE_DATABASE";
    case StatementType.STATEMENT_TYPE_VIEW:
      return "STATEMENT_TYPE_VIEW";
    case StatementType.STATEMENT_TYPE_GROUP:
      return "STATEMENT_TYPE_GROUP";
    case StatementType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum StructType {
  STRUCT_TYPE_UNSET = 0,
  STRUCT_TYPE_EXPRESSION = 1,
  STRUCT_TYPE_RUN_CODE_FRAME = 2,
  STRUCT_TYPE_RUN_ERROR = 3,
  STRUCT_TYPE_LOG_ENTRY = 4,
  STRUCT_TYPE_PROJECTION = 5,
  UNRECOGNIZED = -1,
}

export function structTypeFromJSON(object: any): StructType {
  switch (object) {
    case 0:
    case "STRUCT_TYPE_UNSET":
      return StructType.STRUCT_TYPE_UNSET;
    case 1:
    case "STRUCT_TYPE_EXPRESSION":
      return StructType.STRUCT_TYPE_EXPRESSION;
    case 2:
    case "STRUCT_TYPE_RUN_CODE_FRAME":
      return StructType.STRUCT_TYPE_RUN_CODE_FRAME;
    case 3:
    case "STRUCT_TYPE_RUN_ERROR":
      return StructType.STRUCT_TYPE_RUN_ERROR;
    case 4:
    case "STRUCT_TYPE_LOG_ENTRY":
      return StructType.STRUCT_TYPE_LOG_ENTRY;
    case 5:
    case "STRUCT_TYPE_PROJECTION":
      return StructType.STRUCT_TYPE_PROJECTION;
    case -1:
    case "UNRECOGNIZED":
    default:
      return StructType.UNRECOGNIZED;
  }
}

export function structTypeToJSON(object: StructType): string {
  switch (object) {
    case StructType.STRUCT_TYPE_UNSET:
      return "STRUCT_TYPE_UNSET";
    case StructType.STRUCT_TYPE_EXPRESSION:
      return "STRUCT_TYPE_EXPRESSION";
    case StructType.STRUCT_TYPE_RUN_CODE_FRAME:
      return "STRUCT_TYPE_RUN_CODE_FRAME";
    case StructType.STRUCT_TYPE_RUN_ERROR:
      return "STRUCT_TYPE_RUN_ERROR";
    case StructType.STRUCT_TYPE_LOG_ENTRY:
      return "STRUCT_TYPE_LOG_ENTRY";
    case StructType.STRUCT_TYPE_PROJECTION:
      return "STRUCT_TYPE_PROJECTION";
    case StructType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TextHeadingLevel {
  TEXT_HEADING_LEVEL_UNSET = 0,
  TEXT_HEADING_LEVEL_H1 = 1,
  TEXT_HEADING_LEVEL_H2 = 2,
  TEXT_HEADING_LEVEL_H3 = 3,
  UNRECOGNIZED = -1,
}

export function textHeadingLevelFromJSON(object: any): TextHeadingLevel {
  switch (object) {
    case 0:
    case "TEXT_HEADING_LEVEL_UNSET":
      return TextHeadingLevel.TEXT_HEADING_LEVEL_UNSET;
    case 1:
    case "TEXT_HEADING_LEVEL_H1":
      return TextHeadingLevel.TEXT_HEADING_LEVEL_H1;
    case 2:
    case "TEXT_HEADING_LEVEL_H2":
      return TextHeadingLevel.TEXT_HEADING_LEVEL_H2;
    case 3:
    case "TEXT_HEADING_LEVEL_H3":
      return TextHeadingLevel.TEXT_HEADING_LEVEL_H3;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TextHeadingLevel.UNRECOGNIZED;
  }
}

export function textHeadingLevelToJSON(object: TextHeadingLevel): string {
  switch (object) {
    case TextHeadingLevel.TEXT_HEADING_LEVEL_UNSET:
      return "TEXT_HEADING_LEVEL_UNSET";
    case TextHeadingLevel.TEXT_HEADING_LEVEL_H1:
      return "TEXT_HEADING_LEVEL_H1";
    case TextHeadingLevel.TEXT_HEADING_LEVEL_H2:
      return "TEXT_HEADING_LEVEL_H2";
    case TextHeadingLevel.TEXT_HEADING_LEVEL_H3:
      return "TEXT_HEADING_LEVEL_H3";
    case TextHeadingLevel.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TriggerType {
  TRIGGER_TYPE_UNSET = 0,
  TRIGGER_TYPE_INVOKE = 1,
  TRIGGER_TYPE_TIME = 2,
  TRIGGER_TYPE_RUN = 3,
  TRIGGER_TYPE_EDIT = 4,
  TRIGGER_TYPE_MESSAGE = 5,
  TRIGGER_TYPE_USER = 6,
  TRIGGER_TYPE_API = 7,
  UNRECOGNIZED = -1,
}

export function triggerTypeFromJSON(object: any): TriggerType {
  switch (object) {
    case 0:
    case "TRIGGER_TYPE_UNSET":
      return TriggerType.TRIGGER_TYPE_UNSET;
    case 1:
    case "TRIGGER_TYPE_INVOKE":
      return TriggerType.TRIGGER_TYPE_INVOKE;
    case 2:
    case "TRIGGER_TYPE_TIME":
      return TriggerType.TRIGGER_TYPE_TIME;
    case 3:
    case "TRIGGER_TYPE_RUN":
      return TriggerType.TRIGGER_TYPE_RUN;
    case 4:
    case "TRIGGER_TYPE_EDIT":
      return TriggerType.TRIGGER_TYPE_EDIT;
    case 5:
    case "TRIGGER_TYPE_MESSAGE":
      return TriggerType.TRIGGER_TYPE_MESSAGE;
    case 6:
    case "TRIGGER_TYPE_USER":
      return TriggerType.TRIGGER_TYPE_USER;
    case 7:
    case "TRIGGER_TYPE_API":
      return TriggerType.TRIGGER_TYPE_API;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TriggerType.UNRECOGNIZED;
  }
}

export function triggerTypeToJSON(object: TriggerType): string {
  switch (object) {
    case TriggerType.TRIGGER_TYPE_UNSET:
      return "TRIGGER_TYPE_UNSET";
    case TriggerType.TRIGGER_TYPE_INVOKE:
      return "TRIGGER_TYPE_INVOKE";
    case TriggerType.TRIGGER_TYPE_TIME:
      return "TRIGGER_TYPE_TIME";
    case TriggerType.TRIGGER_TYPE_RUN:
      return "TRIGGER_TYPE_RUN";
    case TriggerType.TRIGGER_TYPE_EDIT:
      return "TRIGGER_TYPE_EDIT";
    case TriggerType.TRIGGER_TYPE_MESSAGE:
      return "TRIGGER_TYPE_MESSAGE";
    case TriggerType.TRIGGER_TYPE_USER:
      return "TRIGGER_TYPE_USER";
    case TriggerType.TRIGGER_TYPE_API:
      return "TRIGGER_TYPE_API";
    case TriggerType.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TypeFlag {
  TYPE_FLAG_ZERO = 0,
  TYPE_FLAG_IS_OUTPUT = 1,
  TYPE_FLAG_IS_ARRAY = 2,
  TYPE_FLAG_IS_OPTIONAL = 4,
  TYPE_FLAG_IS_UNION_WITH = 8,
  TYPE_FLAG_IS_SECRET = 16,
  TYPE_FLAG_IS_STORE_ONLY = 32,
  TYPE_FLAG_IS_ARRAYABLE = 64,
  TYPE_FLAG_IS_META = 128,
  TYPE_FLAG_IS_CONFIG = 256,
  TYPE_FLAG_IS_HIDDEN = 512,
  UNRECOGNIZED = -1,
}

export function typeFlagFromJSON(object: any): TypeFlag {
  switch (object) {
    case 0:
    case "TYPE_FLAG_ZERO":
      return TypeFlag.TYPE_FLAG_ZERO;
    case 1:
    case "TYPE_FLAG_IS_OUTPUT":
      return TypeFlag.TYPE_FLAG_IS_OUTPUT;
    case 2:
    case "TYPE_FLAG_IS_ARRAY":
      return TypeFlag.TYPE_FLAG_IS_ARRAY;
    case 4:
    case "TYPE_FLAG_IS_OPTIONAL":
      return TypeFlag.TYPE_FLAG_IS_OPTIONAL;
    case 8:
    case "TYPE_FLAG_IS_UNION_WITH":
      return TypeFlag.TYPE_FLAG_IS_UNION_WITH;
    case 16:
    case "TYPE_FLAG_IS_SECRET":
      return TypeFlag.TYPE_FLAG_IS_SECRET;
    case 32:
    case "TYPE_FLAG_IS_STORE_ONLY":
      return TypeFlag.TYPE_FLAG_IS_STORE_ONLY;
    case 64:
    case "TYPE_FLAG_IS_ARRAYABLE":
      return TypeFlag.TYPE_FLAG_IS_ARRAYABLE;
    case 128:
    case "TYPE_FLAG_IS_META":
      return TypeFlag.TYPE_FLAG_IS_META;
    case 256:
    case "TYPE_FLAG_IS_CONFIG":
      return TypeFlag.TYPE_FLAG_IS_CONFIG;
    case 512:
    case "TYPE_FLAG_IS_HIDDEN":
      return TypeFlag.TYPE_FLAG_IS_HIDDEN;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TypeFlag.UNRECOGNIZED;
  }
}

export function typeFlagToJSON(object: TypeFlag): string {
  switch (object) {
    case TypeFlag.TYPE_FLAG_ZERO:
      return "TYPE_FLAG_ZERO";
    case TypeFlag.TYPE_FLAG_IS_OUTPUT:
      return "TYPE_FLAG_IS_OUTPUT";
    case TypeFlag.TYPE_FLAG_IS_ARRAY:
      return "TYPE_FLAG_IS_ARRAY";
    case TypeFlag.TYPE_FLAG_IS_OPTIONAL:
      return "TYPE_FLAG_IS_OPTIONAL";
    case TypeFlag.TYPE_FLAG_IS_UNION_WITH:
      return "TYPE_FLAG_IS_UNION_WITH";
    case TypeFlag.TYPE_FLAG_IS_SECRET:
      return "TYPE_FLAG_IS_SECRET";
    case TypeFlag.TYPE_FLAG_IS_STORE_ONLY:
      return "TYPE_FLAG_IS_STORE_ONLY";
    case TypeFlag.TYPE_FLAG_IS_ARRAYABLE:
      return "TYPE_FLAG_IS_ARRAYABLE";
    case TypeFlag.TYPE_FLAG_IS_META:
      return "TYPE_FLAG_IS_META";
    case TypeFlag.TYPE_FLAG_IS_CONFIG:
      return "TYPE_FLAG_IS_CONFIG";
    case TypeFlag.TYPE_FLAG_IS_HIDDEN:
      return "TYPE_FLAG_IS_HIDDEN";
    case TypeFlag.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TypeHint {
  TYPE_HINT_UNSET = 0,
  TYPE_HINT_NAME = 1,
  TYPE_HINT_UUID = 2,
  TYPE_HINT_DATE = 3,
  TYPE_HINT_DATETIME = 4,
  TYPE_HINT_TIME = 5,
  TYPE_HINT_DURATION = 6,
  TYPE_HINT_EMAIL = 7,
  TYPE_HINT_URL = 8,
  TYPE_HINT_MARKDOWN = 9,
  TYPE_HINT_RICH_TEXT = 10,
  TYPE_HINT_HTML = 11,
  TYPE_HINT_CODE = 12,
  TYPE_HINT_KEY = 13,
  TYPE_HINT_INTEGER = 14,
  TYPE_HINT_FLOAT = 15,
  TYPE_HINT_SLIDER = 16,
  TYPE_HINT_PHONE = 17,
  TYPE_HINT_RATING = 18,
  TYPE_HINT_TOGGLE = 19,
  TYPE_HINT_CHECKBOX = 20,
  TYPE_HINT_THUMBS = 21,
  TYPE_HINT_EMBEDDING = 22,
  TYPE_HINT_IMAGE = 23,
  TYPE_HINT_VIDEO = 24,
  TYPE_HINT_AUDIO = 25,
  TYPE_HINT_FILE = 26,
  TYPE_HINT_STATEMENT = 27,
  TYPE_HINT_RECORD = 28,
  TYPE_HINT_FIELD = 29,
  TYPE_HINT_RUN = 30,
  TYPE_HINT_SECRET = 31,
  TYPE_HINT_BLOB = 32,
  UNRECOGNIZED = -1,
}

export function typeHintFromJSON(object: any): TypeHint {
  switch (object) {
    case 0:
    case "TYPE_HINT_UNSET":
      return TypeHint.TYPE_HINT_UNSET;
    case 1:
    case "TYPE_HINT_NAME":
      return TypeHint.TYPE_HINT_NAME;
    case 2:
    case "TYPE_HINT_UUID":
      return TypeHint.TYPE_HINT_UUID;
    case 3:
    case "TYPE_HINT_DATE":
      return TypeHint.TYPE_HINT_DATE;
    case 4:
    case "TYPE_HINT_DATETIME":
      return TypeHint.TYPE_HINT_DATETIME;
    case 5:
    case "TYPE_HINT_TIME":
      return TypeHint.TYPE_HINT_TIME;
    case 6:
    case "TYPE_HINT_DURATION":
      return TypeHint.TYPE_HINT_DURATION;
    case 7:
    case "TYPE_HINT_EMAIL":
      return TypeHint.TYPE_HINT_EMAIL;
    case 8:
    case "TYPE_HINT_URL":
      return TypeHint.TYPE_HINT_URL;
    case 9:
    case "TYPE_HINT_MARKDOWN":
      return TypeHint.TYPE_HINT_MARKDOWN;
    case 10:
    case "TYPE_HINT_RICH_TEXT":
      return TypeHint.TYPE_HINT_RICH_TEXT;
    case 11:
    case "TYPE_HINT_HTML":
      return TypeHint.TYPE_HINT_HTML;
    case 12:
    case "TYPE_HINT_CODE":
      return TypeHint.TYPE_HINT_CODE;
    case 13:
    case "TYPE_HINT_KEY":
      return TypeHint.TYPE_HINT_KEY;
    case 14:
    case "TYPE_HINT_INTEGER":
      return TypeHint.TYPE_HINT_INTEGER;
    case 15:
    case "TYPE_HINT_FLOAT":
      return TypeHint.TYPE_HINT_FLOAT;
    case 16:
    case "TYPE_HINT_SLIDER":
      return TypeHint.TYPE_HINT_SLIDER;
    case 17:
    case "TYPE_HINT_PHONE":
      return TypeHint.TYPE_HINT_PHONE;
    case 18:
    case "TYPE_HINT_RATING":
      return TypeHint.TYPE_HINT_RATING;
    case 19:
    case "TYPE_HINT_TOGGLE":
      return TypeHint.TYPE_HINT_TOGGLE;
    case 20:
    case "TYPE_HINT_CHECKBOX":
      return TypeHint.TYPE_HINT_CHECKBOX;
    case 21:
    case "TYPE_HINT_THUMBS":
      return TypeHint.TYPE_HINT_THUMBS;
    case 22:
    case "TYPE_HINT_EMBEDDING":
      return TypeHint.TYPE_HINT_EMBEDDING;
    case 23:
    case "TYPE_HINT_IMAGE":
      return TypeHint.TYPE_HINT_IMAGE;
    case 24:
    case "TYPE_HINT_VIDEO":
      return TypeHint.TYPE_HINT_VIDEO;
    case 25:
    case "TYPE_HINT_AUDIO":
      return TypeHint.TYPE_HINT_AUDIO;
    case 26:
    case "TYPE_HINT_FILE":
      return TypeHint.TYPE_HINT_FILE;
    case 27:
    case "TYPE_HINT_STATEMENT":
      return TypeHint.TYPE_HINT_STATEMENT;
    case 28:
    case "TYPE_HINT_RECORD":
      return TypeHint.TYPE_HINT_RECORD;
    case 29:
    case "TYPE_HINT_FIELD":
      return TypeHint.TYPE_HINT_FIELD;
    case 30:
    case "TYPE_HINT_RUN":
      return TypeHint.TYPE_HINT_RUN;
    case 31:
    case "TYPE_HINT_SECRET":
      return TypeHint.TYPE_HINT_SECRET;
    case 32:
    case "TYPE_HINT_BLOB":
      return TypeHint.TYPE_HINT_BLOB;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TypeHint.UNRECOGNIZED;
  }
}

export function typeHintToJSON(object: TypeHint): string {
  switch (object) {
    case TypeHint.TYPE_HINT_UNSET:
      return "TYPE_HINT_UNSET";
    case TypeHint.TYPE_HINT_NAME:
      return "TYPE_HINT_NAME";
    case TypeHint.TYPE_HINT_UUID:
      return "TYPE_HINT_UUID";
    case TypeHint.TYPE_HINT_DATE:
      return "TYPE_HINT_DATE";
    case TypeHint.TYPE_HINT_DATETIME:
      return "TYPE_HINT_DATETIME";
    case TypeHint.TYPE_HINT_TIME:
      return "TYPE_HINT_TIME";
    case TypeHint.TYPE_HINT_DURATION:
      return "TYPE_HINT_DURATION";
    case TypeHint.TYPE_HINT_EMAIL:
      return "TYPE_HINT_EMAIL";
    case TypeHint.TYPE_HINT_URL:
      return "TYPE_HINT_URL";
    case TypeHint.TYPE_HINT_MARKDOWN:
      return "TYPE_HINT_MARKDOWN";
    case TypeHint.TYPE_HINT_RICH_TEXT:
      return "TYPE_HINT_RICH_TEXT";
    case TypeHint.TYPE_HINT_HTML:
      return "TYPE_HINT_HTML";
    case TypeHint.TYPE_HINT_CODE:
      return "TYPE_HINT_CODE";
    case TypeHint.TYPE_HINT_KEY:
      return "TYPE_HINT_KEY";
    case TypeHint.TYPE_HINT_INTEGER:
      return "TYPE_HINT_INTEGER";
    case TypeHint.TYPE_HINT_FLOAT:
      return "TYPE_HINT_FLOAT";
    case TypeHint.TYPE_HINT_SLIDER:
      return "TYPE_HINT_SLIDER";
    case TypeHint.TYPE_HINT_PHONE:
      return "TYPE_HINT_PHONE";
    case TypeHint.TYPE_HINT_RATING:
      return "TYPE_HINT_RATING";
    case TypeHint.TYPE_HINT_TOGGLE:
      return "TYPE_HINT_TOGGLE";
    case TypeHint.TYPE_HINT_CHECKBOX:
      return "TYPE_HINT_CHECKBOX";
    case TypeHint.TYPE_HINT_THUMBS:
      return "TYPE_HINT_THUMBS";
    case TypeHint.TYPE_HINT_EMBEDDING:
      return "TYPE_HINT_EMBEDDING";
    case TypeHint.TYPE_HINT_IMAGE:
      return "TYPE_HINT_IMAGE";
    case TypeHint.TYPE_HINT_VIDEO:
      return "TYPE_HINT_VIDEO";
    case TypeHint.TYPE_HINT_AUDIO:
      return "TYPE_HINT_AUDIO";
    case TypeHint.TYPE_HINT_FILE:
      return "TYPE_HINT_FILE";
    case TypeHint.TYPE_HINT_STATEMENT:
      return "TYPE_HINT_STATEMENT";
    case TypeHint.TYPE_HINT_RECORD:
      return "TYPE_HINT_RECORD";
    case TypeHint.TYPE_HINT_FIELD:
      return "TYPE_HINT_FIELD";
    case TypeHint.TYPE_HINT_RUN:
      return "TYPE_HINT_RUN";
    case TypeHint.TYPE_HINT_SECRET:
      return "TYPE_HINT_SECRET";
    case TypeHint.TYPE_HINT_BLOB:
      return "TYPE_HINT_BLOB";
    case TypeHint.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TypeStorageFormat {
  TYPE_STORAGE_FORMAT_UNSET = 0,
  TYPE_STORAGE_FORMAT_STRING = 1,
  TYPE_STORAGE_FORMAT_DOUBLE = 2,
  TYPE_STORAGE_FORMAT_LONG = 3,
  TYPE_STORAGE_FORMAT_VECTOR = 4,
  TYPE_STORAGE_FORMAT_BINARY = 5,
  TYPE_STORAGE_FORMAT_BOOLEAN = 6,
  TYPE_STORAGE_FORMAT_DATE = 7,
  TYPE_STORAGE_FORMAT_KEYWORD = 8,
  TYPE_STORAGE_FORMAT_OBJECT = 9,
  TYPE_STORAGE_FORMAT_RELATION = 10,
  UNRECOGNIZED = -1,
}

export function typeStorageFormatFromJSON(object: any): TypeStorageFormat {
  switch (object) {
    case 0:
    case "TYPE_STORAGE_FORMAT_UNSET":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_UNSET;
    case 1:
    case "TYPE_STORAGE_FORMAT_STRING":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_STRING;
    case 2:
    case "TYPE_STORAGE_FORMAT_DOUBLE":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_DOUBLE;
    case 3:
    case "TYPE_STORAGE_FORMAT_LONG":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_LONG;
    case 4:
    case "TYPE_STORAGE_FORMAT_VECTOR":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_VECTOR;
    case 5:
    case "TYPE_STORAGE_FORMAT_BINARY":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_BINARY;
    case 6:
    case "TYPE_STORAGE_FORMAT_BOOLEAN":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_BOOLEAN;
    case 7:
    case "TYPE_STORAGE_FORMAT_DATE":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_DATE;
    case 8:
    case "TYPE_STORAGE_FORMAT_KEYWORD":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_KEYWORD;
    case 9:
    case "TYPE_STORAGE_FORMAT_OBJECT":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_OBJECT;
    case 10:
    case "TYPE_STORAGE_FORMAT_RELATION":
      return TypeStorageFormat.TYPE_STORAGE_FORMAT_RELATION;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TypeStorageFormat.UNRECOGNIZED;
  }
}

export function typeStorageFormatToJSON(object: TypeStorageFormat): string {
  switch (object) {
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_UNSET:
      return "TYPE_STORAGE_FORMAT_UNSET";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_STRING:
      return "TYPE_STORAGE_FORMAT_STRING";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_DOUBLE:
      return "TYPE_STORAGE_FORMAT_DOUBLE";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_LONG:
      return "TYPE_STORAGE_FORMAT_LONG";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_VECTOR:
      return "TYPE_STORAGE_FORMAT_VECTOR";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_BINARY:
      return "TYPE_STORAGE_FORMAT_BINARY";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_BOOLEAN:
      return "TYPE_STORAGE_FORMAT_BOOLEAN";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_DATE:
      return "TYPE_STORAGE_FORMAT_DATE";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_KEYWORD:
      return "TYPE_STORAGE_FORMAT_KEYWORD";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_OBJECT:
      return "TYPE_STORAGE_FORMAT_OBJECT";
    case TypeStorageFormat.TYPE_STORAGE_FORMAT_RELATION:
      return "TYPE_STORAGE_FORMAT_RELATION";
    case TypeStorageFormat.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum TypeTag {
  TYPE_TAG_UNSET = 0,
  TYPE_TAG_STRING = 1,
  TYPE_TAG_NUMBER = 2,
  TYPE_TAG_BOOLEAN = 3,
  TYPE_TAG_VECTOR = 4,
  TYPE_TAG_BLOB = 5,
  TYPE_TAG_STRUCT = 6,
  TYPE_TAG_JSON = 7,
  TYPE_TAG_FUNCTION = 8,
  TYPE_TAG_ENUM = 9,
  TYPE_TAG_LITERAL = 10,
  TYPE_TAG_TYPE_REFERENCE = 11,
  TYPE_TAG_NODE = 12,
  TYPE_TAG_ANY = 13,
  UNRECOGNIZED = -1,
}

export function typeTagFromJSON(object: any): TypeTag {
  switch (object) {
    case 0:
    case "TYPE_TAG_UNSET":
      return TypeTag.TYPE_TAG_UNSET;
    case 1:
    case "TYPE_TAG_STRING":
      return TypeTag.TYPE_TAG_STRING;
    case 2:
    case "TYPE_TAG_NUMBER":
      return TypeTag.TYPE_TAG_NUMBER;
    case 3:
    case "TYPE_TAG_BOOLEAN":
      return TypeTag.TYPE_TAG_BOOLEAN;
    case 4:
    case "TYPE_TAG_VECTOR":
      return TypeTag.TYPE_TAG_VECTOR;
    case 5:
    case "TYPE_TAG_BLOB":
      return TypeTag.TYPE_TAG_BLOB;
    case 6:
    case "TYPE_TAG_STRUCT":
      return TypeTag.TYPE_TAG_STRUCT;
    case 7:
    case "TYPE_TAG_JSON":
      return TypeTag.TYPE_TAG_JSON;
    case 8:
    case "TYPE_TAG_FUNCTION":
      return TypeTag.TYPE_TAG_FUNCTION;
    case 9:
    case "TYPE_TAG_ENUM":
      return TypeTag.TYPE_TAG_ENUM;
    case 10:
    case "TYPE_TAG_LITERAL":
      return TypeTag.TYPE_TAG_LITERAL;
    case 11:
    case "TYPE_TAG_TYPE_REFERENCE":
      return TypeTag.TYPE_TAG_TYPE_REFERENCE;
    case 12:
    case "TYPE_TAG_NODE":
      return TypeTag.TYPE_TAG_NODE;
    case 13:
    case "TYPE_TAG_ANY":
      return TypeTag.TYPE_TAG_ANY;
    case -1:
    case "UNRECOGNIZED":
    default:
      return TypeTag.UNRECOGNIZED;
  }
}

export function typeTagToJSON(object: TypeTag): string {
  switch (object) {
    case TypeTag.TYPE_TAG_UNSET:
      return "TYPE_TAG_UNSET";
    case TypeTag.TYPE_TAG_STRING:
      return "TYPE_TAG_STRING";
    case TypeTag.TYPE_TAG_NUMBER:
      return "TYPE_TAG_NUMBER";
    case TypeTag.TYPE_TAG_BOOLEAN:
      return "TYPE_TAG_BOOLEAN";
    case TypeTag.TYPE_TAG_VECTOR:
      return "TYPE_TAG_VECTOR";
    case TypeTag.TYPE_TAG_BLOB:
      return "TYPE_TAG_BLOB";
    case TypeTag.TYPE_TAG_STRUCT:
      return "TYPE_TAG_STRUCT";
    case TypeTag.TYPE_TAG_JSON:
      return "TYPE_TAG_JSON";
    case TypeTag.TYPE_TAG_FUNCTION:
      return "TYPE_TAG_FUNCTION";
    case TypeTag.TYPE_TAG_ENUM:
      return "TYPE_TAG_ENUM";
    case TypeTag.TYPE_TAG_LITERAL:
      return "TYPE_TAG_LITERAL";
    case TypeTag.TYPE_TAG_TYPE_REFERENCE:
      return "TYPE_TAG_TYPE_REFERENCE";
    case TypeTag.TYPE_TAG_NODE:
      return "TYPE_TAG_NODE";
    case TypeTag.TYPE_TAG_ANY:
      return "TYPE_TAG_ANY";
    case TypeTag.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum ViewLayout {
  VIEW_LAYOUT_UNSET = 0,
  VIEW_LAYOUT_TABLE = 1,
  UNRECOGNIZED = -1,
}

export function viewLayoutFromJSON(object: any): ViewLayout {
  switch (object) {
    case 0:
    case "VIEW_LAYOUT_UNSET":
      return ViewLayout.VIEW_LAYOUT_UNSET;
    case 1:
    case "VIEW_LAYOUT_TABLE":
      return ViewLayout.VIEW_LAYOUT_TABLE;
    case -1:
    case "UNRECOGNIZED":
    default:
      return ViewLayout.UNRECOGNIZED;
  }
}

export function viewLayoutToJSON(object: ViewLayout): string {
  switch (object) {
    case ViewLayout.VIEW_LAYOUT_UNSET:
      return "VIEW_LAYOUT_UNSET";
    case ViewLayout.VIEW_LAYOUT_TABLE:
      return "VIEW_LAYOUT_TABLE";
    case ViewLayout.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum WorkerProfile {
  WORKER_PROFILE_UNSET = 0,
  WORKER_PROFILE_TINY = 1,
  WORKER_PROFILE_SMALL = 2,
  WORKER_PROFILE_MEDIUM = 3,
  WORKER_PROFILE_LARGE = 4,
  WORKER_PROFILE_XLARGE_CPU = 5,
  WORKER_PROFILE_XLARGE_MEM = 6,
  UNRECOGNIZED = -1,
}

export function workerProfileFromJSON(object: any): WorkerProfile {
  switch (object) {
    case 0:
    case "WORKER_PROFILE_UNSET":
      return WorkerProfile.WORKER_PROFILE_UNSET;
    case 1:
    case "WORKER_PROFILE_TINY":
      return WorkerProfile.WORKER_PROFILE_TINY;
    case 2:
    case "WORKER_PROFILE_SMALL":
      return WorkerProfile.WORKER_PROFILE_SMALL;
    case 3:
    case "WORKER_PROFILE_MEDIUM":
      return WorkerProfile.WORKER_PROFILE_MEDIUM;
    case 4:
    case "WORKER_PROFILE_LARGE":
      return WorkerProfile.WORKER_PROFILE_LARGE;
    case 5:
    case "WORKER_PROFILE_XLARGE_CPU":
      return WorkerProfile.WORKER_PROFILE_XLARGE_CPU;
    case 6:
    case "WORKER_PROFILE_XLARGE_MEM":
      return WorkerProfile.WORKER_PROFILE_XLARGE_MEM;
    case -1:
    case "UNRECOGNIZED":
    default:
      return WorkerProfile.UNRECOGNIZED;
  }
}

export function workerProfileToJSON(object: WorkerProfile): string {
  switch (object) {
    case WorkerProfile.WORKER_PROFILE_UNSET:
      return "WORKER_PROFILE_UNSET";
    case WorkerProfile.WORKER_PROFILE_TINY:
      return "WORKER_PROFILE_TINY";
    case WorkerProfile.WORKER_PROFILE_SMALL:
      return "WORKER_PROFILE_SMALL";
    case WorkerProfile.WORKER_PROFILE_MEDIUM:
      return "WORKER_PROFILE_MEDIUM";
    case WorkerProfile.WORKER_PROFILE_LARGE:
      return "WORKER_PROFILE_LARGE";
    case WorkerProfile.WORKER_PROFILE_XLARGE_CPU:
      return "WORKER_PROFILE_XLARGE_CPU";
    case WorkerProfile.WORKER_PROFILE_XLARGE_MEM:
      return "WORKER_PROFILE_XLARGE_MEM";
    case WorkerProfile.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export enum WorkerSetStatus {
  WORKER_SET_STATUS_UNSET = 0,
  WORKER_SET_STATUS_SLEEPING = 1,
  WORKER_SET_STATUS_PENDING = 2,
  WORKER_SET_STATUS_UPDATING = 3,
  WORKER_SET_STATUS_HEALTHY = 4,
  WORKER_SET_STATUS_UNHEALTHY = 5,
  WORKER_SET_STATUS_UNAVAILABLE = 6,
  WORKER_SET_STATUS_UNKNOWN = 7,
  UNRECOGNIZED = -1,
}

export function workerSetStatusFromJSON(object: any): WorkerSetStatus {
  switch (object) {
    case 0:
    case "WORKER_SET_STATUS_UNSET":
      return WorkerSetStatus.WORKER_SET_STATUS_UNSET;
    case 1:
    case "WORKER_SET_STATUS_SLEEPING":
      return WorkerSetStatus.WORKER_SET_STATUS_SLEEPING;
    case 2:
    case "WORKER_SET_STATUS_PENDING":
      return WorkerSetStatus.WORKER_SET_STATUS_PENDING;
    case 3:
    case "WORKER_SET_STATUS_UPDATING":
      return WorkerSetStatus.WORKER_SET_STATUS_UPDATING;
    case 4:
    case "WORKER_SET_STATUS_HEALTHY":
      return WorkerSetStatus.WORKER_SET_STATUS_HEALTHY;
    case 5:
    case "WORKER_SET_STATUS_UNHEALTHY":
      return WorkerSetStatus.WORKER_SET_STATUS_UNHEALTHY;
    case 6:
    case "WORKER_SET_STATUS_UNAVAILABLE":
      return WorkerSetStatus.WORKER_SET_STATUS_UNAVAILABLE;
    case 7:
    case "WORKER_SET_STATUS_UNKNOWN":
      return WorkerSetStatus.WORKER_SET_STATUS_UNKNOWN;
    case -1:
    case "UNRECOGNIZED":
    default:
      return WorkerSetStatus.UNRECOGNIZED;
  }
}

export function workerSetStatusToJSON(object: WorkerSetStatus): string {
  switch (object) {
    case WorkerSetStatus.WORKER_SET_STATUS_UNSET:
      return "WORKER_SET_STATUS_UNSET";
    case WorkerSetStatus.WORKER_SET_STATUS_SLEEPING:
      return "WORKER_SET_STATUS_SLEEPING";
    case WorkerSetStatus.WORKER_SET_STATUS_PENDING:
      return "WORKER_SET_STATUS_PENDING";
    case WorkerSetStatus.WORKER_SET_STATUS_UPDATING:
      return "WORKER_SET_STATUS_UPDATING";
    case WorkerSetStatus.WORKER_SET_STATUS_HEALTHY:
      return "WORKER_SET_STATUS_HEALTHY";
    case WorkerSetStatus.WORKER_SET_STATUS_UNHEALTHY:
      return "WORKER_SET_STATUS_UNHEALTHY";
    case WorkerSetStatus.WORKER_SET_STATUS_UNAVAILABLE:
      return "WORKER_SET_STATUS_UNAVAILABLE";
    case WorkerSetStatus.WORKER_SET_STATUS_UNKNOWN:
      return "WORKER_SET_STATUS_UNKNOWN";
    case WorkerSetStatus.UNRECOGNIZED:
    default:
      return "UNRECOGNIZED";
  }
}

export interface ExpressionData {
  op: ExpressionOp;
  fieldCk: string;
  fieldKey: string;
  clauses: ExpressionData[];
  value: { [key: string]: any } | undefined;
  mode: string;
}

export interface LogEntryData {
  id: string;
  moduleCk: string;
  createdAt: Date | undefined;
  stream: string;
  sessionCk: string;
  level: string;
  logger: string;
  statementCk: string;
  runCk: string;
  message: string;
  value: { [key: string]: any } | undefined;
}

export interface RunCodeFrameData {
  filename: string;
  lineno: number;
  name: string;
  locals: { [key: string]: any } | undefined;
  line: string;
}

export interface RunErrorData {
  kind: RunErrorKind;
  type: string;
  message: string;
  statementCk: string;
  traceback: { [key: string]: any }[];
}

export interface BlobData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  sha512: string;
  contentLength: number;
  contentType: string;
  name: string;
  status: BlobStatus;
}

export interface FieldData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
  orderKey: string;
  text: string;
  tag: TypeTag;
  key: string;
  value: { [key: string]: any } | undefined;
  hint: string;
  flags: number;
  referenceCk: string;
}

export interface FileData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
}

export interface IssueData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  type: IssueType;
  kind: IssueKind;
  message: string;
  subjectCk: string;
  path: string;
  properties: string[];
}

export interface ModuleData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
  committed: boolean;
}

export interface RecordData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  value: { [key: string]: any } | undefined;
}

export interface ResolvedFieldData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  fieldCk: string;
  name: string;
  orderKey: string;
  text: string;
  tag: TypeTag;
  key: string;
  value: { [key: string]: any } | undefined;
  hint: string;
  flags: number;
  referenceCk: string;
}

export interface RunData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  statementCk: string;
  statementPath: string;
  scheduledAt: Date | undefined;
  startedAt: Date | undefined;
  terminatedAt: Date | undefined;
  triggerType: string;
  triggerCk: string;
  accessLevel: number;
  status: RunStatus;
  inputs: { [key: string]: any } | undefined;
  outputs: { [key: string]: any } | undefined;
  error: { [key: string]: any } | undefined;
  value: { [key: string]: any } | undefined;
}

export interface SecretData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  sha512: string;
}

export interface SessionData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  accessLevel: number;
  workerNodeId: string;
  workerProcessId: string;
  triggerType: TriggerType;
  triggerId: string;
  openedAt: Date | undefined;
  closedAt: Date | undefined;
  inferenceTimeout: number;
  inferenceRetries: number;
}

export interface StatementData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  type: StatementType;
  name: string;
  orderKey: string;
  referenceCk: string;
  headingLevel: number;
  text: string;
  key: string;
  code: string;
  value: { [key: string]: any } | undefined;
  versioned: boolean;
  externalName: string;
}

export interface TaggingData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  key: string;
  value: { [key: string]: any } | undefined;
  referenceCk: string;
}

export interface TriggerData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  type: TriggerType;
  active: boolean;
  scheduleType: string;
  timezone: string;
  interval: number;
  cron: string;
}

export interface ViewData {
  id: string;
  ck: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
  layout: ViewLayout;
  query: { [key: string]: any } | undefined;
  sort: { [key: string]: any }[];
}

function createBaseExpressionData(): ExpressionData {
  return { op: 0, fieldCk: "", fieldKey: "", clauses: [], value: undefined, mode: "" };
}

export const ExpressionData = {
  encode(message: ExpressionData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.op !== 0) {
      writer.uint32(168).int32(message.op);
    }
    if (message.fieldCk !== "") {
      writer.uint32(178).string(message.fieldCk);
    }
    if (message.fieldKey !== "") {
      writer.uint32(186).string(message.fieldKey);
    }
    for (const v of message.clauses) {
      ExpressionData.encode(v!, writer.uint32(194).fork()).ldelim();
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(202).fork()).ldelim();
    }
    if (message.mode !== "") {
      writer.uint32(210).string(message.mode);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ExpressionData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseExpressionData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 21:
          if (tag !== 168) {
            break;
          }

          message.op = reader.int32() as any;
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.fieldCk = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.fieldKey = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.clauses.push(ExpressionData.decode(reader, reader.uint32()));
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.mode = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ExpressionData {
    return {
      op: isSet(object.op) ? expressionOpFromJSON(object.op) : 0,
      fieldCk: isSet(object.fieldCk) ? globalThis.String(object.fieldCk) : "",
      fieldKey: isSet(object.fieldKey) ? globalThis.String(object.fieldKey) : "",
      clauses: globalThis.Array.isArray(object?.clauses)
        ? object.clauses.map((e: any) => ExpressionData.fromJSON(e))
        : [],
      value: isObject(object.value) ? object.value : undefined,
      mode: isSet(object.mode) ? globalThis.String(object.mode) : "",
    };
  },

  toJSON(message: ExpressionData): unknown {
    const obj: any = {};
    if (message.op !== 0) {
      obj.op = expressionOpToJSON(message.op);
    }
    if (message.fieldCk !== "") {
      obj.fieldCk = message.fieldCk;
    }
    if (message.fieldKey !== "") {
      obj.fieldKey = message.fieldKey;
    }
    if (message.clauses?.length) {
      obj.clauses = message.clauses.map((e) => ExpressionData.toJSON(e));
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    if (message.mode !== "") {
      obj.mode = message.mode;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ExpressionData>, I>>(base?: I): ExpressionData {
    return ExpressionData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ExpressionData>, I>>(object: I): ExpressionData {
    const message = createBaseExpressionData();
    message.op = object.op ?? 0;
    message.fieldCk = object.fieldCk ?? "";
    message.fieldKey = object.fieldKey ?? "";
    message.clauses = object.clauses?.map((e) => ExpressionData.fromPartial(e)) || [];
    message.value = object.value ?? undefined;
    message.mode = object.mode ?? "";
    return message;
  },
};

function createBaseLogEntryData(): LogEntryData {
  return {
    id: "",
    moduleCk: "",
    createdAt: undefined,
    stream: "",
    sessionCk: "",
    level: "",
    logger: "",
    statementCk: "",
    runCk: "",
    message: "",
    value: undefined,
  };
}

export const LogEntryData = {
  encode(message: LogEntryData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(162).string(message.id);
    }
    if (message.moduleCk !== "") {
      writer.uint32(170).string(message.moduleCk);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(178).fork()).ldelim();
    }
    if (message.stream !== "") {
      writer.uint32(186).string(message.stream);
    }
    if (message.sessionCk !== "") {
      writer.uint32(194).string(message.sessionCk);
    }
    if (message.level !== "") {
      writer.uint32(202).string(message.level);
    }
    if (message.logger !== "") {
      writer.uint32(210).string(message.logger);
    }
    if (message.statementCk !== "") {
      writer.uint32(218).string(message.statementCk);
    }
    if (message.runCk !== "") {
      writer.uint32(226).string(message.runCk);
    }
    if (message.message !== "") {
      writer.uint32(234).string(message.message);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(242).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): LogEntryData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseLogEntryData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 20:
          if (tag !== 162) {
            break;
          }

          message.id = reader.string();
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.moduleCk = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.stream = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.sessionCk = reader.string();
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.level = reader.string();
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.logger = reader.string();
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.statementCk = reader.string();
          continue;
        case 28:
          if (tag !== 226) {
            break;
          }

          message.runCk = reader.string();
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.message = reader.string();
          continue;
        case 30:
          if (tag !== 242) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): LogEntryData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      moduleCk: isSet(object.moduleCk) ? globalThis.String(object.moduleCk) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      stream: isSet(object.stream) ? globalThis.String(object.stream) : "",
      sessionCk: isSet(object.sessionCk) ? globalThis.String(object.sessionCk) : "",
      level: isSet(object.level) ? globalThis.String(object.level) : "",
      logger: isSet(object.logger) ? globalThis.String(object.logger) : "",
      statementCk: isSet(object.statementCk) ? globalThis.String(object.statementCk) : "",
      runCk: isSet(object.runCk) ? globalThis.String(object.runCk) : "",
      message: isSet(object.message) ? globalThis.String(object.message) : "",
      value: isObject(object.value) ? object.value : undefined,
    };
  },

  toJSON(message: LogEntryData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.moduleCk !== "") {
      obj.moduleCk = message.moduleCk;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.stream !== "") {
      obj.stream = message.stream;
    }
    if (message.sessionCk !== "") {
      obj.sessionCk = message.sessionCk;
    }
    if (message.level !== "") {
      obj.level = message.level;
    }
    if (message.logger !== "") {
      obj.logger = message.logger;
    }
    if (message.statementCk !== "") {
      obj.statementCk = message.statementCk;
    }
    if (message.runCk !== "") {
      obj.runCk = message.runCk;
    }
    if (message.message !== "") {
      obj.message = message.message;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<LogEntryData>, I>>(base?: I): LogEntryData {
    return LogEntryData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<LogEntryData>, I>>(object: I): LogEntryData {
    const message = createBaseLogEntryData();
    message.id = object.id ?? "";
    message.moduleCk = object.moduleCk ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.stream = object.stream ?? "";
    message.sessionCk = object.sessionCk ?? "";
    message.level = object.level ?? "";
    message.logger = object.logger ?? "";
    message.statementCk = object.statementCk ?? "";
    message.runCk = object.runCk ?? "";
    message.message = object.message ?? "";
    message.value = object.value ?? undefined;
    return message;
  },
};

function createBaseRunCodeFrameData(): RunCodeFrameData {
  return { filename: "", lineno: 0, name: "", locals: undefined, line: "" };
}

export const RunCodeFrameData = {
  encode(message: RunCodeFrameData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.filename !== "") {
      writer.uint32(162).string(message.filename);
    }
    if (message.lineno !== 0) {
      writer.uint32(168).int64(message.lineno);
    }
    if (message.name !== "") {
      writer.uint32(178).string(message.name);
    }
    if (message.locals !== undefined) {
      Struct.encode(Struct.wrap(message.locals), writer.uint32(186).fork()).ldelim();
    }
    if (message.line !== "") {
      writer.uint32(194).string(message.line);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RunCodeFrameData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRunCodeFrameData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 20:
          if (tag !== 162) {
            break;
          }

          message.filename = reader.string();
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.lineno = longToNumber(reader.int64() as Long);
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.name = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.locals = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.line = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RunCodeFrameData {
    return {
      filename: isSet(object.filename) ? globalThis.String(object.filename) : "",
      lineno: isSet(object.lineno) ? globalThis.Number(object.lineno) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      locals: isObject(object.locals) ? object.locals : undefined,
      line: isSet(object.line) ? globalThis.String(object.line) : "",
    };
  },

  toJSON(message: RunCodeFrameData): unknown {
    const obj: any = {};
    if (message.filename !== "") {
      obj.filename = message.filename;
    }
    if (message.lineno !== 0) {
      obj.lineno = Math.round(message.lineno);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.locals !== undefined) {
      obj.locals = message.locals;
    }
    if (message.line !== "") {
      obj.line = message.line;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunCodeFrameData>, I>>(base?: I): RunCodeFrameData {
    return RunCodeFrameData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunCodeFrameData>, I>>(object: I): RunCodeFrameData {
    const message = createBaseRunCodeFrameData();
    message.filename = object.filename ?? "";
    message.lineno = object.lineno ?? 0;
    message.name = object.name ?? "";
    message.locals = object.locals ?? undefined;
    message.line = object.line ?? "";
    return message;
  },
};

function createBaseRunErrorData(): RunErrorData {
  return { kind: 0, type: "", message: "", statementCk: "", traceback: [] };
}

export const RunErrorData = {
  encode(message: RunErrorData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.kind !== 0) {
      writer.uint32(160).int32(message.kind);
    }
    if (message.type !== "") {
      writer.uint32(170).string(message.type);
    }
    if (message.message !== "") {
      writer.uint32(178).string(message.message);
    }
    if (message.statementCk !== "") {
      writer.uint32(186).string(message.statementCk);
    }
    for (const v of message.traceback) {
      Struct.encode(Struct.wrap(v!), writer.uint32(194).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RunErrorData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRunErrorData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 20:
          if (tag !== 160) {
            break;
          }

          message.kind = reader.int32() as any;
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.type = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.message = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.statementCk = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.traceback.push(Struct.unwrap(Struct.decode(reader, reader.uint32())));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RunErrorData {
    return {
      kind: isSet(object.kind) ? runErrorKindFromJSON(object.kind) : 0,
      type: isSet(object.type) ? globalThis.String(object.type) : "",
      message: isSet(object.message) ? globalThis.String(object.message) : "",
      statementCk: isSet(object.statementCk) ? globalThis.String(object.statementCk) : "",
      traceback: globalThis.Array.isArray(object?.traceback) ? [...object.traceback] : [],
    };
  },

  toJSON(message: RunErrorData): unknown {
    const obj: any = {};
    if (message.kind !== 0) {
      obj.kind = runErrorKindToJSON(message.kind);
    }
    if (message.type !== "") {
      obj.type = message.type;
    }
    if (message.message !== "") {
      obj.message = message.message;
    }
    if (message.statementCk !== "") {
      obj.statementCk = message.statementCk;
    }
    if (message.traceback?.length) {
      obj.traceback = message.traceback;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunErrorData>, I>>(base?: I): RunErrorData {
    return RunErrorData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunErrorData>, I>>(object: I): RunErrorData {
    const message = createBaseRunErrorData();
    message.kind = object.kind ?? 0;
    message.type = object.type ?? "";
    message.message = object.message ?? "";
    message.statementCk = object.statementCk ?? "";
    message.traceback = object.traceback?.map((e) => e) || [];
    return message;
  },
};

function createBaseBlobData(): BlobData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    sha512: "",
    contentLength: 0,
    contentType: "",
    name: "",
    status: 0,
  };
}

export const BlobData = {
  encode(message: BlobData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.sha512 !== "") {
      writer.uint32(162).string(message.sha512);
    }
    if (message.contentLength !== 0) {
      writer.uint32(168).int64(message.contentLength);
    }
    if (message.contentType !== "") {
      writer.uint32(178).string(message.contentType);
    }
    if (message.name !== "") {
      writer.uint32(186).string(message.name);
    }
    if (message.status !== 0) {
      writer.uint32(192).int32(message.status);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BlobData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBlobData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.sha512 = reader.string();
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.contentLength = longToNumber(reader.int64() as Long);
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.contentType = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.name = reader.string();
          continue;
        case 24:
          if (tag !== 192) {
            break;
          }

          message.status = reader.int32() as any;
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): BlobData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      sha512: isSet(object.sha512) ? globalThis.String(object.sha512) : "",
      contentLength: isSet(object.contentLength) ? globalThis.Number(object.contentLength) : 0,
      contentType: isSet(object.contentType) ? globalThis.String(object.contentType) : "",
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      status: isSet(object.status) ? blobStatusFromJSON(object.status) : 0,
    };
  },

  toJSON(message: BlobData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.sha512 !== "") {
      obj.sha512 = message.sha512;
    }
    if (message.contentLength !== 0) {
      obj.contentLength = Math.round(message.contentLength);
    }
    if (message.contentType !== "") {
      obj.contentType = message.contentType;
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.status !== 0) {
      obj.status = blobStatusToJSON(message.status);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<BlobData>, I>>(base?: I): BlobData {
    return BlobData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<BlobData>, I>>(object: I): BlobData {
    const message = createBaseBlobData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.sha512 = object.sha512 ?? "";
    message.contentLength = object.contentLength ?? 0;
    message.contentType = object.contentType ?? "";
    message.name = object.name ?? "";
    message.status = object.status ?? 0;
    return message;
  },
};

function createBaseFieldData(): FieldData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    name: "",
    orderKey: "",
    text: "",
    tag: 0,
    key: "",
    value: undefined,
    hint: "",
    flags: 0,
    referenceCk: "",
  };
}

export const FieldData = {
  encode(message: FieldData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.name !== "") {
      writer.uint32(170).string(message.name);
    }
    if (message.orderKey !== "") {
      writer.uint32(178).string(message.orderKey);
    }
    if (message.text !== "") {
      writer.uint32(186).string(message.text);
    }
    if (message.tag !== 0) {
      writer.uint32(192).int32(message.tag);
    }
    if (message.key !== "") {
      writer.uint32(202).string(message.key);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(210).fork()).ldelim();
    }
    if (message.hint !== "") {
      writer.uint32(218).string(message.hint);
    }
    if (message.flags !== 0) {
      writer.uint32(224).int64(message.flags);
    }
    if (message.referenceCk !== "") {
      writer.uint32(234).string(message.referenceCk);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): FieldData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseFieldData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.name = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.orderKey = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.text = reader.string();
          continue;
        case 24:
          if (tag !== 192) {
            break;
          }

          message.tag = reader.int32() as any;
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.key = reader.string();
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.hint = reader.string();
          continue;
        case 28:
          if (tag !== 224) {
            break;
          }

          message.flags = longToNumber(reader.int64() as Long);
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.referenceCk = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): FieldData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      orderKey: isSet(object.orderKey) ? globalThis.String(object.orderKey) : "",
      text: isSet(object.text) ? globalThis.String(object.text) : "",
      tag: isSet(object.tag) ? typeTagFromJSON(object.tag) : 0,
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isObject(object.value) ? object.value : undefined,
      hint: isSet(object.hint) ? globalThis.String(object.hint) : "",
      flags: isSet(object.flags) ? globalThis.Number(object.flags) : 0,
      referenceCk: isSet(object.referenceCk) ? globalThis.String(object.referenceCk) : "",
    };
  },

  toJSON(message: FieldData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.orderKey !== "") {
      obj.orderKey = message.orderKey;
    }
    if (message.text !== "") {
      obj.text = message.text;
    }
    if (message.tag !== 0) {
      obj.tag = typeTagToJSON(message.tag);
    }
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    if (message.hint !== "") {
      obj.hint = message.hint;
    }
    if (message.flags !== 0) {
      obj.flags = Math.round(message.flags);
    }
    if (message.referenceCk !== "") {
      obj.referenceCk = message.referenceCk;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<FieldData>, I>>(base?: I): FieldData {
    return FieldData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<FieldData>, I>>(object: I): FieldData {
    const message = createBaseFieldData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.name = object.name ?? "";
    message.orderKey = object.orderKey ?? "";
    message.text = object.text ?? "";
    message.tag = object.tag ?? 0;
    message.key = object.key ?? "";
    message.value = object.value ?? undefined;
    message.hint = object.hint ?? "";
    message.flags = object.flags ?? 0;
    message.referenceCk = object.referenceCk ?? "";
    return message;
  },
};

function createBaseFileData(): FileData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    name: "",
  };
}

export const FileData = {
  encode(message: FileData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.name !== "") {
      writer.uint32(162).string(message.name);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): FileData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseFileData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.name = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): FileData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
    };
  },

  toJSON(message: FileData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<FileData>, I>>(base?: I): FileData {
    return FileData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<FileData>, I>>(object: I): FileData {
    const message = createBaseFileData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.name = object.name ?? "";
    return message;
  },
};

function createBaseIssueData(): IssueData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    type: 0,
    kind: 0,
    message: "",
    subjectCk: "",
    path: "",
    properties: [],
  };
}

export const IssueData = {
  encode(message: IssueData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.type !== 0) {
      writer.uint32(160).int32(message.type);
    }
    if (message.kind !== 0) {
      writer.uint32(168).int32(message.kind);
    }
    if (message.message !== "") {
      writer.uint32(178).string(message.message);
    }
    if (message.subjectCk !== "") {
      writer.uint32(186).string(message.subjectCk);
    }
    if (message.path !== "") {
      writer.uint32(194).string(message.path);
    }
    for (const v of message.properties) {
      writer.uint32(202).string(v!);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): IssueData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseIssueData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 160) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.kind = reader.int32() as any;
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.message = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.subjectCk = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.path = reader.string();
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.properties.push(reader.string());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): IssueData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      type: isSet(object.type) ? issueTypeFromJSON(object.type) : 0,
      kind: isSet(object.kind) ? issueKindFromJSON(object.kind) : 0,
      message: isSet(object.message) ? globalThis.String(object.message) : "",
      subjectCk: isSet(object.subjectCk) ? globalThis.String(object.subjectCk) : "",
      path: isSet(object.path) ? globalThis.String(object.path) : "",
      properties: globalThis.Array.isArray(object?.properties)
        ? object.properties.map((e: any) => globalThis.String(e))
        : [],
    };
  },

  toJSON(message: IssueData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.type !== 0) {
      obj.type = issueTypeToJSON(message.type);
    }
    if (message.kind !== 0) {
      obj.kind = issueKindToJSON(message.kind);
    }
    if (message.message !== "") {
      obj.message = message.message;
    }
    if (message.subjectCk !== "") {
      obj.subjectCk = message.subjectCk;
    }
    if (message.path !== "") {
      obj.path = message.path;
    }
    if (message.properties?.length) {
      obj.properties = message.properties;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<IssueData>, I>>(base?: I): IssueData {
    return IssueData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<IssueData>, I>>(object: I): IssueData {
    const message = createBaseIssueData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.type = object.type ?? 0;
    message.kind = object.kind ?? 0;
    message.message = object.message ?? "";
    message.subjectCk = object.subjectCk ?? "";
    message.path = object.path ?? "";
    message.properties = object.properties?.map((e) => e) || [];
    return message;
  },
};

function createBaseModuleData(): ModuleData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    name: "",
    committed: false,
  };
}

export const ModuleData = {
  encode(message: ModuleData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.name !== "") {
      writer.uint32(162).string(message.name);
    }
    if (message.committed === true) {
      writer.uint32(168).bool(message.committed);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ModuleData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseModuleData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.name = reader.string();
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.committed = reader.bool();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ModuleData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      committed: isSet(object.committed) ? globalThis.Boolean(object.committed) : false,
    };
  },

  toJSON(message: ModuleData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.committed === true) {
      obj.committed = message.committed;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ModuleData>, I>>(base?: I): ModuleData {
    return ModuleData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ModuleData>, I>>(object: I): ModuleData {
    const message = createBaseModuleData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.name = object.name ?? "";
    message.committed = object.committed ?? false;
    return message;
  },
};

function createBaseRecordData(): RecordData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    value: undefined,
  };
}

export const RecordData = {
  encode(message: RecordData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(162).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RecordData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRecordData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RecordData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      value: isObject(object.value) ? object.value : undefined,
    };
  },

  toJSON(message: RecordData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RecordData>, I>>(base?: I): RecordData {
    return RecordData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RecordData>, I>>(object: I): RecordData {
    const message = createBaseRecordData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.value = object.value ?? undefined;
    return message;
  },
};

function createBaseResolvedFieldData(): ResolvedFieldData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    fieldCk: "",
    name: "",
    orderKey: "",
    text: "",
    tag: 0,
    key: "",
    value: undefined,
    hint: "",
    flags: 0,
    referenceCk: "",
  };
}

export const ResolvedFieldData = {
  encode(message: ResolvedFieldData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.fieldCk !== "") {
      writer.uint32(162).string(message.fieldCk);
    }
    if (message.name !== "") {
      writer.uint32(170).string(message.name);
    }
    if (message.orderKey !== "") {
      writer.uint32(178).string(message.orderKey);
    }
    if (message.text !== "") {
      writer.uint32(186).string(message.text);
    }
    if (message.tag !== 0) {
      writer.uint32(192).int32(message.tag);
    }
    if (message.key !== "") {
      writer.uint32(202).string(message.key);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(210).fork()).ldelim();
    }
    if (message.hint !== "") {
      writer.uint32(218).string(message.hint);
    }
    if (message.flags !== 0) {
      writer.uint32(224).int64(message.flags);
    }
    if (message.referenceCk !== "") {
      writer.uint32(234).string(message.referenceCk);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ResolvedFieldData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseResolvedFieldData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.fieldCk = reader.string();
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.name = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.orderKey = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.text = reader.string();
          continue;
        case 24:
          if (tag !== 192) {
            break;
          }

          message.tag = reader.int32() as any;
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.key = reader.string();
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.hint = reader.string();
          continue;
        case 28:
          if (tag !== 224) {
            break;
          }

          message.flags = longToNumber(reader.int64() as Long);
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.referenceCk = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ResolvedFieldData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      fieldCk: isSet(object.fieldCk) ? globalThis.String(object.fieldCk) : "",
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      orderKey: isSet(object.orderKey) ? globalThis.String(object.orderKey) : "",
      text: isSet(object.text) ? globalThis.String(object.text) : "",
      tag: isSet(object.tag) ? typeTagFromJSON(object.tag) : 0,
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isObject(object.value) ? object.value : undefined,
      hint: isSet(object.hint) ? globalThis.String(object.hint) : "",
      flags: isSet(object.flags) ? globalThis.Number(object.flags) : 0,
      referenceCk: isSet(object.referenceCk) ? globalThis.String(object.referenceCk) : "",
    };
  },

  toJSON(message: ResolvedFieldData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.fieldCk !== "") {
      obj.fieldCk = message.fieldCk;
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.orderKey !== "") {
      obj.orderKey = message.orderKey;
    }
    if (message.text !== "") {
      obj.text = message.text;
    }
    if (message.tag !== 0) {
      obj.tag = typeTagToJSON(message.tag);
    }
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    if (message.hint !== "") {
      obj.hint = message.hint;
    }
    if (message.flags !== 0) {
      obj.flags = Math.round(message.flags);
    }
    if (message.referenceCk !== "") {
      obj.referenceCk = message.referenceCk;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ResolvedFieldData>, I>>(base?: I): ResolvedFieldData {
    return ResolvedFieldData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ResolvedFieldData>, I>>(object: I): ResolvedFieldData {
    const message = createBaseResolvedFieldData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.fieldCk = object.fieldCk ?? "";
    message.name = object.name ?? "";
    message.orderKey = object.orderKey ?? "";
    message.text = object.text ?? "";
    message.tag = object.tag ?? 0;
    message.key = object.key ?? "";
    message.value = object.value ?? undefined;
    message.hint = object.hint ?? "";
    message.flags = object.flags ?? 0;
    message.referenceCk = object.referenceCk ?? "";
    return message;
  },
};

function createBaseRunData(): RunData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    statementCk: "",
    statementPath: "",
    scheduledAt: undefined,
    startedAt: undefined,
    terminatedAt: undefined,
    triggerType: "",
    triggerCk: "",
    accessLevel: 0,
    status: 0,
    inputs: undefined,
    outputs: undefined,
    error: undefined,
    value: undefined,
  };
}

export const RunData = {
  encode(message: RunData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.statementCk !== "") {
      writer.uint32(178).string(message.statementCk);
    }
    if (message.statementPath !== "") {
      writer.uint32(186).string(message.statementPath);
    }
    if (message.scheduledAt !== undefined) {
      Timestamp.encode(toTimestamp(message.scheduledAt), writer.uint32(194).fork()).ldelim();
    }
    if (message.startedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.startedAt), writer.uint32(202).fork()).ldelim();
    }
    if (message.terminatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.terminatedAt), writer.uint32(210).fork()).ldelim();
    }
    if (message.triggerType !== "") {
      writer.uint32(218).string(message.triggerType);
    }
    if (message.triggerCk !== "") {
      writer.uint32(226).string(message.triggerCk);
    }
    if (message.accessLevel !== 0) {
      writer.uint32(232).int64(message.accessLevel);
    }
    if (message.status !== 0) {
      writer.uint32(240).int32(message.status);
    }
    if (message.inputs !== undefined) {
      Struct.encode(Struct.wrap(message.inputs), writer.uint32(250).fork()).ldelim();
    }
    if (message.outputs !== undefined) {
      Struct.encode(Struct.wrap(message.outputs), writer.uint32(258).fork()).ldelim();
    }
    if (message.error !== undefined) {
      Struct.encode(Struct.wrap(message.error), writer.uint32(266).fork()).ldelim();
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(274).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RunData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRunData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.statementCk = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.statementPath = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.scheduledAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.startedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.terminatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.triggerType = reader.string();
          continue;
        case 28:
          if (tag !== 226) {
            break;
          }

          message.triggerCk = reader.string();
          continue;
        case 29:
          if (tag !== 232) {
            break;
          }

          message.accessLevel = longToNumber(reader.int64() as Long);
          continue;
        case 30:
          if (tag !== 240) {
            break;
          }

          message.status = reader.int32() as any;
          continue;
        case 31:
          if (tag !== 250) {
            break;
          }

          message.inputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 32:
          if (tag !== 258) {
            break;
          }

          message.outputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 33:
          if (tag !== 266) {
            break;
          }

          message.error = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 34:
          if (tag !== 274) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RunData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      statementCk: isSet(object.statementCk) ? globalThis.String(object.statementCk) : "",
      statementPath: isSet(object.statementPath) ? globalThis.String(object.statementPath) : "",
      scheduledAt: isSet(object.scheduledAt) ? fromJsonTimestamp(object.scheduledAt) : undefined,
      startedAt: isSet(object.startedAt) ? fromJsonTimestamp(object.startedAt) : undefined,
      terminatedAt: isSet(object.terminatedAt) ? fromJsonTimestamp(object.terminatedAt) : undefined,
      triggerType: isSet(object.triggerType) ? globalThis.String(object.triggerType) : "",
      triggerCk: isSet(object.triggerCk) ? globalThis.String(object.triggerCk) : "",
      accessLevel: isSet(object.accessLevel) ? globalThis.Number(object.accessLevel) : 0,
      status: isSet(object.status) ? runStatusFromJSON(object.status) : 0,
      inputs: isObject(object.inputs) ? object.inputs : undefined,
      outputs: isObject(object.outputs) ? object.outputs : undefined,
      error: isObject(object.error) ? object.error : undefined,
      value: isObject(object.value) ? object.value : undefined,
    };
  },

  toJSON(message: RunData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.statementCk !== "") {
      obj.statementCk = message.statementCk;
    }
    if (message.statementPath !== "") {
      obj.statementPath = message.statementPath;
    }
    if (message.scheduledAt !== undefined) {
      obj.scheduledAt = message.scheduledAt.toISOString();
    }
    if (message.startedAt !== undefined) {
      obj.startedAt = message.startedAt.toISOString();
    }
    if (message.terminatedAt !== undefined) {
      obj.terminatedAt = message.terminatedAt.toISOString();
    }
    if (message.triggerType !== "") {
      obj.triggerType = message.triggerType;
    }
    if (message.triggerCk !== "") {
      obj.triggerCk = message.triggerCk;
    }
    if (message.accessLevel !== 0) {
      obj.accessLevel = Math.round(message.accessLevel);
    }
    if (message.status !== 0) {
      obj.status = runStatusToJSON(message.status);
    }
    if (message.inputs !== undefined) {
      obj.inputs = message.inputs;
    }
    if (message.outputs !== undefined) {
      obj.outputs = message.outputs;
    }
    if (message.error !== undefined) {
      obj.error = message.error;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunData>, I>>(base?: I): RunData {
    return RunData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunData>, I>>(object: I): RunData {
    const message = createBaseRunData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.statementCk = object.statementCk ?? "";
    message.statementPath = object.statementPath ?? "";
    message.scheduledAt = object.scheduledAt ?? undefined;
    message.startedAt = object.startedAt ?? undefined;
    message.terminatedAt = object.terminatedAt ?? undefined;
    message.triggerType = object.triggerType ?? "";
    message.triggerCk = object.triggerCk ?? "";
    message.accessLevel = object.accessLevel ?? 0;
    message.status = object.status ?? 0;
    message.inputs = object.inputs ?? undefined;
    message.outputs = object.outputs ?? undefined;
    message.error = object.error ?? undefined;
    message.value = object.value ?? undefined;
    return message;
  },
};

function createBaseSecretData(): SecretData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    sha512: "",
  };
}

export const SecretData = {
  encode(message: SecretData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.sha512 !== "") {
      writer.uint32(162).string(message.sha512);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SecretData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSecretData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.sha512 = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SecretData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      sha512: isSet(object.sha512) ? globalThis.String(object.sha512) : "",
    };
  },

  toJSON(message: SecretData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.sha512 !== "") {
      obj.sha512 = message.sha512;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SecretData>, I>>(base?: I): SecretData {
    return SecretData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SecretData>, I>>(object: I): SecretData {
    const message = createBaseSecretData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.sha512 = object.sha512 ?? "";
    return message;
  },
};

function createBaseSessionData(): SessionData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    accessLevel: 0,
    workerNodeId: "",
    workerProcessId: "",
    triggerType: 0,
    triggerId: "",
    openedAt: undefined,
    closedAt: undefined,
    inferenceTimeout: 0,
    inferenceRetries: 0,
  };
}

export const SessionData = {
  encode(message: SessionData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.accessLevel !== 0) {
      writer.uint32(160).int64(message.accessLevel);
    }
    if (message.workerNodeId !== "") {
      writer.uint32(170).string(message.workerNodeId);
    }
    if (message.workerProcessId !== "") {
      writer.uint32(178).string(message.workerProcessId);
    }
    if (message.triggerType !== 0) {
      writer.uint32(184).int32(message.triggerType);
    }
    if (message.triggerId !== "") {
      writer.uint32(194).string(message.triggerId);
    }
    if (message.openedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.openedAt), writer.uint32(202).fork()).ldelim();
    }
    if (message.closedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.closedAt), writer.uint32(210).fork()).ldelim();
    }
    if (message.inferenceTimeout !== 0) {
      writer.uint32(216).int64(message.inferenceTimeout);
    }
    if (message.inferenceRetries !== 0) {
      writer.uint32(224).int64(message.inferenceRetries);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SessionData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSessionData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 160) {
            break;
          }

          message.accessLevel = longToNumber(reader.int64() as Long);
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.workerNodeId = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.workerProcessId = reader.string();
          continue;
        case 23:
          if (tag !== 184) {
            break;
          }

          message.triggerType = reader.int32() as any;
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.triggerId = reader.string();
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.openedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.closedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 27:
          if (tag !== 216) {
            break;
          }

          message.inferenceTimeout = longToNumber(reader.int64() as Long);
          continue;
        case 28:
          if (tag !== 224) {
            break;
          }

          message.inferenceRetries = longToNumber(reader.int64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SessionData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      accessLevel: isSet(object.accessLevel) ? globalThis.Number(object.accessLevel) : 0,
      workerNodeId: isSet(object.workerNodeId) ? globalThis.String(object.workerNodeId) : "",
      workerProcessId: isSet(object.workerProcessId) ? globalThis.String(object.workerProcessId) : "",
      triggerType: isSet(object.triggerType) ? triggerTypeFromJSON(object.triggerType) : 0,
      triggerId: isSet(object.triggerId) ? globalThis.String(object.triggerId) : "",
      openedAt: isSet(object.openedAt) ? fromJsonTimestamp(object.openedAt) : undefined,
      closedAt: isSet(object.closedAt) ? fromJsonTimestamp(object.closedAt) : undefined,
      inferenceTimeout: isSet(object.inferenceTimeout) ? globalThis.Number(object.inferenceTimeout) : 0,
      inferenceRetries: isSet(object.inferenceRetries) ? globalThis.Number(object.inferenceRetries) : 0,
    };
  },

  toJSON(message: SessionData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.accessLevel !== 0) {
      obj.accessLevel = Math.round(message.accessLevel);
    }
    if (message.workerNodeId !== "") {
      obj.workerNodeId = message.workerNodeId;
    }
    if (message.workerProcessId !== "") {
      obj.workerProcessId = message.workerProcessId;
    }
    if (message.triggerType !== 0) {
      obj.triggerType = triggerTypeToJSON(message.triggerType);
    }
    if (message.triggerId !== "") {
      obj.triggerId = message.triggerId;
    }
    if (message.openedAt !== undefined) {
      obj.openedAt = message.openedAt.toISOString();
    }
    if (message.closedAt !== undefined) {
      obj.closedAt = message.closedAt.toISOString();
    }
    if (message.inferenceTimeout !== 0) {
      obj.inferenceTimeout = Math.round(message.inferenceTimeout);
    }
    if (message.inferenceRetries !== 0) {
      obj.inferenceRetries = Math.round(message.inferenceRetries);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SessionData>, I>>(base?: I): SessionData {
    return SessionData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SessionData>, I>>(object: I): SessionData {
    const message = createBaseSessionData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.accessLevel = object.accessLevel ?? 0;
    message.workerNodeId = object.workerNodeId ?? "";
    message.workerProcessId = object.workerProcessId ?? "";
    message.triggerType = object.triggerType ?? 0;
    message.triggerId = object.triggerId ?? "";
    message.openedAt = object.openedAt ?? undefined;
    message.closedAt = object.closedAt ?? undefined;
    message.inferenceTimeout = object.inferenceTimeout ?? 0;
    message.inferenceRetries = object.inferenceRetries ?? 0;
    return message;
  },
};

function createBaseStatementData(): StatementData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    type: 0,
    name: "",
    orderKey: "",
    referenceCk: "",
    headingLevel: 0,
    text: "",
    key: "",
    code: "",
    value: undefined,
    versioned: false,
    externalName: "",
  };
}

export const StatementData = {
  encode(message: StatementData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.type !== 0) {
      writer.uint32(160).int32(message.type);
    }
    if (message.name !== "") {
      writer.uint32(178).string(message.name);
    }
    if (message.orderKey !== "") {
      writer.uint32(186).string(message.orderKey);
    }
    if (message.referenceCk !== "") {
      writer.uint32(194).string(message.referenceCk);
    }
    if (message.headingLevel !== 0) {
      writer.uint32(200).int64(message.headingLevel);
    }
    if (message.text !== "") {
      writer.uint32(210).string(message.text);
    }
    if (message.key !== "") {
      writer.uint32(218).string(message.key);
    }
    if (message.code !== "") {
      writer.uint32(226).string(message.code);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(234).fork()).ldelim();
    }
    if (message.versioned === true) {
      writer.uint32(240).bool(message.versioned);
    }
    if (message.externalName !== "") {
      writer.uint32(250).string(message.externalName);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StatementData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStatementData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 160) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.name = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.orderKey = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.referenceCk = reader.string();
          continue;
        case 25:
          if (tag !== 200) {
            break;
          }

          message.headingLevel = longToNumber(reader.int64() as Long);
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.text = reader.string();
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.key = reader.string();
          continue;
        case 28:
          if (tag !== 226) {
            break;
          }

          message.code = reader.string();
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 30:
          if (tag !== 240) {
            break;
          }

          message.versioned = reader.bool();
          continue;
        case 31:
          if (tag !== 250) {
            break;
          }

          message.externalName = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): StatementData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      type: isSet(object.type) ? statementTypeFromJSON(object.type) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      orderKey: isSet(object.orderKey) ? globalThis.String(object.orderKey) : "",
      referenceCk: isSet(object.referenceCk) ? globalThis.String(object.referenceCk) : "",
      headingLevel: isSet(object.headingLevel) ? globalThis.Number(object.headingLevel) : 0,
      text: isSet(object.text) ? globalThis.String(object.text) : "",
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      code: isSet(object.code) ? globalThis.String(object.code) : "",
      value: isObject(object.value) ? object.value : undefined,
      versioned: isSet(object.versioned) ? globalThis.Boolean(object.versioned) : false,
      externalName: isSet(object.externalName) ? globalThis.String(object.externalName) : "",
    };
  },

  toJSON(message: StatementData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.type !== 0) {
      obj.type = statementTypeToJSON(message.type);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.orderKey !== "") {
      obj.orderKey = message.orderKey;
    }
    if (message.referenceCk !== "") {
      obj.referenceCk = message.referenceCk;
    }
    if (message.headingLevel !== 0) {
      obj.headingLevel = Math.round(message.headingLevel);
    }
    if (message.text !== "") {
      obj.text = message.text;
    }
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.code !== "") {
      obj.code = message.code;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    if (message.versioned === true) {
      obj.versioned = message.versioned;
    }
    if (message.externalName !== "") {
      obj.externalName = message.externalName;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<StatementData>, I>>(base?: I): StatementData {
    return StatementData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<StatementData>, I>>(object: I): StatementData {
    const message = createBaseStatementData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.type = object.type ?? 0;
    message.name = object.name ?? "";
    message.orderKey = object.orderKey ?? "";
    message.referenceCk = object.referenceCk ?? "";
    message.headingLevel = object.headingLevel ?? 0;
    message.text = object.text ?? "";
    message.key = object.key ?? "";
    message.code = object.code ?? "";
    message.value = object.value ?? undefined;
    message.versioned = object.versioned ?? false;
    message.externalName = object.externalName ?? "";
    return message;
  },
};

function createBaseTaggingData(): TaggingData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    key: "",
    value: undefined,
    referenceCk: "",
  };
}

export const TaggingData = {
  encode(message: TaggingData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.key !== "") {
      writer.uint32(162).string(message.key);
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(170).fork()).ldelim();
    }
    if (message.referenceCk !== "") {
      writer.uint32(178).string(message.referenceCk);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TaggingData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTaggingData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.key = reader.string();
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.value = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.referenceCk = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): TaggingData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isObject(object.value) ? object.value : undefined,
      referenceCk: isSet(object.referenceCk) ? globalThis.String(object.referenceCk) : "",
    };
  },

  toJSON(message: TaggingData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== undefined) {
      obj.value = message.value;
    }
    if (message.referenceCk !== "") {
      obj.referenceCk = message.referenceCk;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<TaggingData>, I>>(base?: I): TaggingData {
    return TaggingData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<TaggingData>, I>>(object: I): TaggingData {
    const message = createBaseTaggingData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.key = object.key ?? "";
    message.value = object.value ?? undefined;
    message.referenceCk = object.referenceCk ?? "";
    return message;
  },
};

function createBaseTriggerData(): TriggerData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    type: 0,
    active: false,
    scheduleType: "",
    timezone: "",
    interval: 0,
    cron: "",
  };
}

export const TriggerData = {
  encode(message: TriggerData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.type !== 0) {
      writer.uint32(160).int32(message.type);
    }
    if (message.active === true) {
      writer.uint32(168).bool(message.active);
    }
    if (message.scheduleType !== "") {
      writer.uint32(178).string(message.scheduleType);
    }
    if (message.timezone !== "") {
      writer.uint32(186).string(message.timezone);
    }
    if (message.interval !== 0) {
      writer.uint32(192).int64(message.interval);
    }
    if (message.cron !== "") {
      writer.uint32(202).string(message.cron);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): TriggerData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseTriggerData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 160) {
            break;
          }

          message.type = reader.int32() as any;
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.active = reader.bool();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.scheduleType = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.timezone = reader.string();
          continue;
        case 24:
          if (tag !== 192) {
            break;
          }

          message.interval = longToNumber(reader.int64() as Long);
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.cron = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): TriggerData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      type: isSet(object.type) ? triggerTypeFromJSON(object.type) : 0,
      active: isSet(object.active) ? globalThis.Boolean(object.active) : false,
      scheduleType: isSet(object.scheduleType) ? globalThis.String(object.scheduleType) : "",
      timezone: isSet(object.timezone) ? globalThis.String(object.timezone) : "",
      interval: isSet(object.interval) ? globalThis.Number(object.interval) : 0,
      cron: isSet(object.cron) ? globalThis.String(object.cron) : "",
    };
  },

  toJSON(message: TriggerData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.type !== 0) {
      obj.type = triggerTypeToJSON(message.type);
    }
    if (message.active === true) {
      obj.active = message.active;
    }
    if (message.scheduleType !== "") {
      obj.scheduleType = message.scheduleType;
    }
    if (message.timezone !== "") {
      obj.timezone = message.timezone;
    }
    if (message.interval !== 0) {
      obj.interval = Math.round(message.interval);
    }
    if (message.cron !== "") {
      obj.cron = message.cron;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<TriggerData>, I>>(base?: I): TriggerData {
    return TriggerData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<TriggerData>, I>>(object: I): TriggerData {
    const message = createBaseTriggerData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.type = object.type ?? 0;
    message.active = object.active ?? false;
    message.scheduleType = object.scheduleType ?? "";
    message.timezone = object.timezone ?? "";
    message.interval = object.interval ?? 0;
    message.cron = object.cron ?? "";
    return message;
  },
};

function createBaseViewData(): ViewData {
  return {
    id: "",
    ck: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    name: "",
    layout: 0,
    query: undefined,
    sort: [],
  };
}

export const ViewData = {
  encode(message: ViewData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.id !== "") {
      writer.uint32(10).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(18).string(message.ck);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.deletedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.deletedAt), writer.uint32(98).fork()).ldelim();
    }
    if (message.lastEditedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastEditedAt), writer.uint32(106).fork()).ldelim();
    }
    if (message.lastChangedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastChangedAt), writer.uint32(114).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(120).int64(message.revision);
    }
    if (message.name !== "") {
      writer.uint32(162).string(message.name);
    }
    if (message.layout !== 0) {
      writer.uint32(168).int32(message.layout);
    }
    if (message.query !== undefined) {
      Struct.encode(Struct.wrap(message.query), writer.uint32(178).fork()).ldelim();
    }
    for (const v of message.sort) {
      Struct.encode(Struct.wrap(v!), writer.uint32(186).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ViewData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseViewData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.id = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.createdAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.updatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.deletedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.lastEditedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.lastChangedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 120) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.name = reader.string();
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.layout = reader.int32() as any;
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.query = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.sort.push(Struct.unwrap(Struct.decode(reader, reader.uint32())));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ViewData {
    return {
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      layout: isSet(object.layout) ? viewLayoutFromJSON(object.layout) : 0,
      query: isObject(object.query) ? object.query : undefined,
      sort: globalThis.Array.isArray(object?.sort) ? [...object.sort] : [],
    };
  },

  toJSON(message: ViewData): unknown {
    const obj: any = {};
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.deletedAt !== undefined) {
      obj.deletedAt = message.deletedAt.toISOString();
    }
    if (message.lastEditedAt !== undefined) {
      obj.lastEditedAt = message.lastEditedAt.toISOString();
    }
    if (message.lastChangedAt !== undefined) {
      obj.lastChangedAt = message.lastChangedAt.toISOString();
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.layout !== 0) {
      obj.layout = viewLayoutToJSON(message.layout);
    }
    if (message.query !== undefined) {
      obj.query = message.query;
    }
    if (message.sort?.length) {
      obj.sort = message.sort;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ViewData>, I>>(base?: I): ViewData {
    return ViewData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ViewData>, I>>(object: I): ViewData {
    const message = createBaseViewData();
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.name = object.name ?? "";
    message.layout = object.layout ?? 0;
    message.query = object.query ?? undefined;
    message.sort = object.sort?.map((e) => e) || [];
    return message;
  },
};

type Builtin = Date | Function | Uint8Array | string | number | boolean | undefined;

export type DeepPartial<T> = T extends Builtin
  ? T
  : T extends globalThis.Array<infer U>
  ? globalThis.Array<DeepPartial<U>>
  : T extends ReadonlyArray<infer U>
  ? ReadonlyArray<DeepPartial<U>>
  : T extends {}
  ? { [K in keyof T]?: DeepPartial<T[K]> }
  : Partial<T>;

type KeysOfUnion<T> = T extends T ? keyof T : never;
export type Exact<P, I extends P> = P extends Builtin
  ? P
  : P & { [K in keyof P]: Exact<P[K], I[K]> } & { [K in Exclude<keyof I, KeysOfUnion<P>>]: never };

function toTimestamp(date: Date): Timestamp {
  const seconds = date.getTime() / 1_000;
  const nanos = (date.getTime() % 1_000) * 1_000_000;
  return { seconds, nanos };
}

function fromTimestamp(t: Timestamp): Date {
  let millis = (t.seconds || 0) * 1_000;
  millis += (t.nanos || 0) / 1_000_000;
  return new globalThis.Date(millis);
}

function fromJsonTimestamp(o: any): Date {
  if (o instanceof globalThis.Date) {
    return o;
  } else if (typeof o === "string") {
    return new globalThis.Date(o);
  } else {
    return fromTimestamp(Timestamp.fromJSON(o));
  }
}

function longToNumber(long: Long): number {
  if (long.gt(globalThis.Number.MAX_SAFE_INTEGER)) {
    throw new globalThis.Error("Value is larger than Number.MAX_SAFE_INTEGER");
  }
  return long.toNumber();
}

if (_m0.util.Long !== Long) {
  _m0.util.Long = Long as any;
  _m0.configure();
}

function isObject(value: any): boolean {
  return typeof value === "object" && value !== null;
}

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
