/* eslint-disable */
import * as _m0 from "protobufjs/minimal";
import { Struct } from "../../google/protobuf/struct";
import { Timestamp } from "../../google/protobuf/timestamp";
import Long = require("long");

export const protobufPackage = "";

export enum AggregationOp {
  UNSPECIFIED = 0,
  COUNT = 1,
  SUM = 2,
  AVERAGE = 3,
  MIN = 4,
  MAX = 5,
  MEDIAN = 6,
  HISTOGRAM = 7,
}

export function aggregationOpFromJSON(object: any): AggregationOp {
  switch (object) {
    case 0:
    case "AGGREGATION_OP_UNSPECIFIED":
      return AggregationOp.UNSPECIFIED;
    case 1:
    case "AGGREGATION_OP_COUNT":
      return AggregationOp.COUNT;
    case 2:
    case "AGGREGATION_OP_SUM":
      return AggregationOp.SUM;
    case 3:
    case "AGGREGATION_OP_AVERAGE":
      return AggregationOp.AVERAGE;
    case 4:
    case "AGGREGATION_OP_MIN":
      return AggregationOp.MIN;
    case 5:
    case "AGGREGATION_OP_MAX":
      return AggregationOp.MAX;
    case 6:
    case "AGGREGATION_OP_MEDIAN":
      return AggregationOp.MEDIAN;
    case 7:
    case "AGGREGATION_OP_HISTOGRAM":
      return AggregationOp.HISTOGRAM;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum AggregationOp");
  }
}

export function aggregationOpToJSON(object: AggregationOp): string {
  switch (object) {
    case AggregationOp.UNSPECIFIED:
      return "AGGREGATION_OP_UNSPECIFIED";
    case AggregationOp.COUNT:
      return "AGGREGATION_OP_COUNT";
    case AggregationOp.SUM:
      return "AGGREGATION_OP_SUM";
    case AggregationOp.AVERAGE:
      return "AGGREGATION_OP_AVERAGE";
    case AggregationOp.MIN:
      return "AGGREGATION_OP_MIN";
    case AggregationOp.MAX:
      return "AGGREGATION_OP_MAX";
    case AggregationOp.MEDIAN:
      return "AGGREGATION_OP_MEDIAN";
    case AggregationOp.HISTOGRAM:
      return "AGGREGATION_OP_HISTOGRAM";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum AggregationOp");
  }
}

export enum BenchType {
  UNSPECIFIED = 0,
  MODULE = 1,
  FILE = 2,
  STATEMENT = 3,
  TRIGGER = 4,
  TAGGING = 5,
  FIELD = 6,
  RECORD = 7,
  VIEW = 8,
  ISSUE = 9,
  RESOLVED_FIELD = 10,
  BLOB = 11,
  SECRET = 12,
  SESSION = 13,
  RUN = 14,
  EXPRESSION = 15,
  RUN_CODE_FRAME = 16,
  RUN_ERROR = 17,
  LOG_ENTRY = 18,
  WORKER_SET = 19,
  ENVIRONMENT = 20,
}

export function benchTypeFromJSON(object: any): BenchType {
  switch (object) {
    case 0:
    case "BENCH_TYPE_UNSPECIFIED":
      return BenchType.UNSPECIFIED;
    case 1:
    case "BENCH_TYPE_MODULE":
      return BenchType.MODULE;
    case 2:
    case "BENCH_TYPE_FILE":
      return BenchType.FILE;
    case 3:
    case "BENCH_TYPE_STATEMENT":
      return BenchType.STATEMENT;
    case 4:
    case "BENCH_TYPE_TRIGGER":
      return BenchType.TRIGGER;
    case 5:
    case "BENCH_TYPE_TAGGING":
      return BenchType.TAGGING;
    case 6:
    case "BENCH_TYPE_FIELD":
      return BenchType.FIELD;
    case 7:
    case "BENCH_TYPE_RECORD":
      return BenchType.RECORD;
    case 8:
    case "BENCH_TYPE_VIEW":
      return BenchType.VIEW;
    case 9:
    case "BENCH_TYPE_ISSUE":
      return BenchType.ISSUE;
    case 10:
    case "BENCH_TYPE_RESOLVED_FIELD":
      return BenchType.RESOLVED_FIELD;
    case 11:
    case "BENCH_TYPE_BLOB":
      return BenchType.BLOB;
    case 12:
    case "BENCH_TYPE_SECRET":
      return BenchType.SECRET;
    case 13:
    case "BENCH_TYPE_SESSION":
      return BenchType.SESSION;
    case 14:
    case "BENCH_TYPE_RUN":
      return BenchType.RUN;
    case 15:
    case "BENCH_TYPE_EXPRESSION":
      return BenchType.EXPRESSION;
    case 16:
    case "BENCH_TYPE_RUN_CODE_FRAME":
      return BenchType.RUN_CODE_FRAME;
    case 17:
    case "BENCH_TYPE_RUN_ERROR":
      return BenchType.RUN_ERROR;
    case 18:
    case "BENCH_TYPE_LOG_ENTRY":
      return BenchType.LOG_ENTRY;
    case 19:
    case "BENCH_TYPE_WORKER_SET":
      return BenchType.WORKER_SET;
    case 20:
    case "BENCH_TYPE_ENVIRONMENT":
      return BenchType.ENVIRONMENT;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum BenchType");
  }
}

export function benchTypeToJSON(object: BenchType): string {
  switch (object) {
    case BenchType.UNSPECIFIED:
      return "BENCH_TYPE_UNSPECIFIED";
    case BenchType.MODULE:
      return "BENCH_TYPE_MODULE";
    case BenchType.FILE:
      return "BENCH_TYPE_FILE";
    case BenchType.STATEMENT:
      return "BENCH_TYPE_STATEMENT";
    case BenchType.TRIGGER:
      return "BENCH_TYPE_TRIGGER";
    case BenchType.TAGGING:
      return "BENCH_TYPE_TAGGING";
    case BenchType.FIELD:
      return "BENCH_TYPE_FIELD";
    case BenchType.RECORD:
      return "BENCH_TYPE_RECORD";
    case BenchType.VIEW:
      return "BENCH_TYPE_VIEW";
    case BenchType.ISSUE:
      return "BENCH_TYPE_ISSUE";
    case BenchType.RESOLVED_FIELD:
      return "BENCH_TYPE_RESOLVED_FIELD";
    case BenchType.BLOB:
      return "BENCH_TYPE_BLOB";
    case BenchType.SECRET:
      return "BENCH_TYPE_SECRET";
    case BenchType.SESSION:
      return "BENCH_TYPE_SESSION";
    case BenchType.RUN:
      return "BENCH_TYPE_RUN";
    case BenchType.EXPRESSION:
      return "BENCH_TYPE_EXPRESSION";
    case BenchType.RUN_CODE_FRAME:
      return "BENCH_TYPE_RUN_CODE_FRAME";
    case BenchType.RUN_ERROR:
      return "BENCH_TYPE_RUN_ERROR";
    case BenchType.LOG_ENTRY:
      return "BENCH_TYPE_LOG_ENTRY";
    case BenchType.WORKER_SET:
      return "BENCH_TYPE_WORKER_SET";
    case BenchType.ENVIRONMENT:
      return "BENCH_TYPE_ENVIRONMENT";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum BenchType");
  }
}

export enum BlobStatus {
  UNSPECIFIED = 0,
  PREPARED = 1,
  UPLOADING = 2,
  AVAILABLE = 3,
}

export function blobStatusFromJSON(object: any): BlobStatus {
  switch (object) {
    case 0:
    case "BLOB_STATUS_UNSPECIFIED":
      return BlobStatus.UNSPECIFIED;
    case 1:
    case "BLOB_STATUS_PREPARED":
      return BlobStatus.PREPARED;
    case 2:
    case "BLOB_STATUS_UPLOADING":
      return BlobStatus.UPLOADING;
    case 3:
    case "BLOB_STATUS_AVAILABLE":
      return BlobStatus.AVAILABLE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum BlobStatus");
  }
}

export function blobStatusToJSON(object: BlobStatus): string {
  switch (object) {
    case BlobStatus.UNSPECIFIED:
      return "BLOB_STATUS_UNSPECIFIED";
    case BlobStatus.PREPARED:
      return "BLOB_STATUS_PREPARED";
    case BlobStatus.UPLOADING:
      return "BLOB_STATUS_UPLOADING";
    case BlobStatus.AVAILABLE:
      return "BLOB_STATUS_AVAILABLE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum BlobStatus");
  }
}

export enum ConditionalOp {
  UNSPECIFIED = 0,
  TRUE = 1,
  FALSE = 2,
  NOT = 3,
  AND = 4,
  OR = 5,
  EQUALS = 6,
  NOT_EQUALS = 7,
  GREATER_THAN = 8,
  GREATER_THAN_OR_EQUALS = 9,
  LESS_THAN = 10,
  LESS_THAN_OR_EQUALS = 11,
  MATCHES = 12,
  STARTS_WITH = 13,
  CONTAINS = 14,
  NOT_CONTAINS = 15,
  IN = 16,
  NOT_IN = 17,
  EXISTS = 18,
  NOT_EXISTS = 19,
  NEAR = 20,
}

export function conditionalOpFromJSON(object: any): ConditionalOp {
  switch (object) {
    case 0:
    case "CONDITIONAL_OP_UNSPECIFIED":
      return ConditionalOp.UNSPECIFIED;
    case 1:
    case "CONDITIONAL_OP_TRUE":
      return ConditionalOp.TRUE;
    case 2:
    case "CONDITIONAL_OP_FALSE":
      return ConditionalOp.FALSE;
    case 3:
    case "CONDITIONAL_OP_NOT":
      return ConditionalOp.NOT;
    case 4:
    case "CONDITIONAL_OP_AND":
      return ConditionalOp.AND;
    case 5:
    case "CONDITIONAL_OP_OR":
      return ConditionalOp.OR;
    case 6:
    case "CONDITIONAL_OP_EQUALS":
      return ConditionalOp.EQUALS;
    case 7:
    case "CONDITIONAL_OP_NOT_EQUALS":
      return ConditionalOp.NOT_EQUALS;
    case 8:
    case "CONDITIONAL_OP_GREATER_THAN":
      return ConditionalOp.GREATER_THAN;
    case 9:
    case "CONDITIONAL_OP_GREATER_THAN_OR_EQUALS":
      return ConditionalOp.GREATER_THAN_OR_EQUALS;
    case 10:
    case "CONDITIONAL_OP_LESS_THAN":
      return ConditionalOp.LESS_THAN;
    case 11:
    case "CONDITIONAL_OP_LESS_THAN_OR_EQUALS":
      return ConditionalOp.LESS_THAN_OR_EQUALS;
    case 12:
    case "CONDITIONAL_OP_MATCHES":
      return ConditionalOp.MATCHES;
    case 13:
    case "CONDITIONAL_OP_STARTS_WITH":
      return ConditionalOp.STARTS_WITH;
    case 14:
    case "CONDITIONAL_OP_CONTAINS":
      return ConditionalOp.CONTAINS;
    case 15:
    case "CONDITIONAL_OP_NOT_CONTAINS":
      return ConditionalOp.NOT_CONTAINS;
    case 16:
    case "CONDITIONAL_OP_IN":
      return ConditionalOp.IN;
    case 17:
    case "CONDITIONAL_OP_NOT_IN":
      return ConditionalOp.NOT_IN;
    case 18:
    case "CONDITIONAL_OP_EXISTS":
      return ConditionalOp.EXISTS;
    case 19:
    case "CONDITIONAL_OP_NOT_EXISTS":
      return ConditionalOp.NOT_EXISTS;
    case 20:
    case "CONDITIONAL_OP_NEAR":
      return ConditionalOp.NEAR;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ConditionalOp");
  }
}

export function conditionalOpToJSON(object: ConditionalOp): string {
  switch (object) {
    case ConditionalOp.UNSPECIFIED:
      return "CONDITIONAL_OP_UNSPECIFIED";
    case ConditionalOp.TRUE:
      return "CONDITIONAL_OP_TRUE";
    case ConditionalOp.FALSE:
      return "CONDITIONAL_OP_FALSE";
    case ConditionalOp.NOT:
      return "CONDITIONAL_OP_NOT";
    case ConditionalOp.AND:
      return "CONDITIONAL_OP_AND";
    case ConditionalOp.OR:
      return "CONDITIONAL_OP_OR";
    case ConditionalOp.EQUALS:
      return "CONDITIONAL_OP_EQUALS";
    case ConditionalOp.NOT_EQUALS:
      return "CONDITIONAL_OP_NOT_EQUALS";
    case ConditionalOp.GREATER_THAN:
      return "CONDITIONAL_OP_GREATER_THAN";
    case ConditionalOp.GREATER_THAN_OR_EQUALS:
      return "CONDITIONAL_OP_GREATER_THAN_OR_EQUALS";
    case ConditionalOp.LESS_THAN:
      return "CONDITIONAL_OP_LESS_THAN";
    case ConditionalOp.LESS_THAN_OR_EQUALS:
      return "CONDITIONAL_OP_LESS_THAN_OR_EQUALS";
    case ConditionalOp.MATCHES:
      return "CONDITIONAL_OP_MATCHES";
    case ConditionalOp.STARTS_WITH:
      return "CONDITIONAL_OP_STARTS_WITH";
    case ConditionalOp.CONTAINS:
      return "CONDITIONAL_OP_CONTAINS";
    case ConditionalOp.NOT_CONTAINS:
      return "CONDITIONAL_OP_NOT_CONTAINS";
    case ConditionalOp.IN:
      return "CONDITIONAL_OP_IN";
    case ConditionalOp.NOT_IN:
      return "CONDITIONAL_OP_NOT_IN";
    case ConditionalOp.EXISTS:
      return "CONDITIONAL_OP_EXISTS";
    case ConditionalOp.NOT_EXISTS:
      return "CONDITIONAL_OP_NOT_EXISTS";
    case ConditionalOp.NEAR:
      return "CONDITIONAL_OP_NEAR";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ConditionalOp");
  }
}

export enum ExpressionKind {
  UNSPECIFIED = 0,
  CONDITIONAL = 1,
  SORT = 2,
  AGGREGATION = 3,
}

export function expressionKindFromJSON(object: any): ExpressionKind {
  switch (object) {
    case 0:
    case "EXPRESSION_KIND_UNSPECIFIED":
      return ExpressionKind.UNSPECIFIED;
    case 1:
    case "EXPRESSION_KIND_CONDITIONAL":
      return ExpressionKind.CONDITIONAL;
    case 2:
    case "EXPRESSION_KIND_SORT":
      return ExpressionKind.SORT;
    case 3:
    case "EXPRESSION_KIND_AGGREGATION":
      return ExpressionKind.AGGREGATION;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ExpressionKind");
  }
}

export function expressionKindToJSON(object: ExpressionKind): string {
  switch (object) {
    case ExpressionKind.UNSPECIFIED:
      return "EXPRESSION_KIND_UNSPECIFIED";
    case ExpressionKind.CONDITIONAL:
      return "EXPRESSION_KIND_CONDITIONAL";
    case ExpressionKind.SORT:
      return "EXPRESSION_KIND_SORT";
    case ExpressionKind.AGGREGATION:
      return "EXPRESSION_KIND_AGGREGATION";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ExpressionKind");
  }
}

export enum ExpressionOp {
  UNSPECIFIED = 0,
  TRUE = 1,
  FALSE = 2,
  NOT = 3,
  AND = 4,
  OR = 5,
  EQUALS = 6,
  NOT_EQUALS = 7,
  GREATER_THAN = 8,
  GREATER_THAN_OR_EQUALS = 9,
  LESS_THAN = 10,
  LESS_THAN_OR_EQUALS = 11,
  MATCHES = 12,
  STARTS_WITH = 13,
  CONTAINS = 14,
  NOT_CONTAINS = 15,
  IN = 16,
  NOT_IN = 17,
  EXISTS = 18,
  NOT_EXISTS = 19,
  NEAR = 20,
  COUNT = 21,
  SUM = 22,
  AVERAGE = 23,
  MIN = 24,
  MAX = 25,
  MEDIAN = 26,
  HISTOGRAM = 27,
  ASCENDING = 28,
  DESCENDING = 29,
}

export function expressionOpFromJSON(object: any): ExpressionOp {
  switch (object) {
    case 0:
    case "EXPRESSION_OP_UNSPECIFIED":
      return ExpressionOp.UNSPECIFIED;
    case 1:
    case "EXPRESSION_OP_TRUE":
      return ExpressionOp.TRUE;
    case 2:
    case "EXPRESSION_OP_FALSE":
      return ExpressionOp.FALSE;
    case 3:
    case "EXPRESSION_OP_NOT":
      return ExpressionOp.NOT;
    case 4:
    case "EXPRESSION_OP_AND":
      return ExpressionOp.AND;
    case 5:
    case "EXPRESSION_OP_OR":
      return ExpressionOp.OR;
    case 6:
    case "EXPRESSION_OP_EQUALS":
      return ExpressionOp.EQUALS;
    case 7:
    case "EXPRESSION_OP_NOT_EQUALS":
      return ExpressionOp.NOT_EQUALS;
    case 8:
    case "EXPRESSION_OP_GREATER_THAN":
      return ExpressionOp.GREATER_THAN;
    case 9:
    case "EXPRESSION_OP_GREATER_THAN_OR_EQUALS":
      return ExpressionOp.GREATER_THAN_OR_EQUALS;
    case 10:
    case "EXPRESSION_OP_LESS_THAN":
      return ExpressionOp.LESS_THAN;
    case 11:
    case "EXPRESSION_OP_LESS_THAN_OR_EQUALS":
      return ExpressionOp.LESS_THAN_OR_EQUALS;
    case 12:
    case "EXPRESSION_OP_MATCHES":
      return ExpressionOp.MATCHES;
    case 13:
    case "EXPRESSION_OP_STARTS_WITH":
      return ExpressionOp.STARTS_WITH;
    case 14:
    case "EXPRESSION_OP_CONTAINS":
      return ExpressionOp.CONTAINS;
    case 15:
    case "EXPRESSION_OP_NOT_CONTAINS":
      return ExpressionOp.NOT_CONTAINS;
    case 16:
    case "EXPRESSION_OP_IN":
      return ExpressionOp.IN;
    case 17:
    case "EXPRESSION_OP_NOT_IN":
      return ExpressionOp.NOT_IN;
    case 18:
    case "EXPRESSION_OP_EXISTS":
      return ExpressionOp.EXISTS;
    case 19:
    case "EXPRESSION_OP_NOT_EXISTS":
      return ExpressionOp.NOT_EXISTS;
    case 20:
    case "EXPRESSION_OP_NEAR":
      return ExpressionOp.NEAR;
    case 21:
    case "EXPRESSION_OP_COUNT":
      return ExpressionOp.COUNT;
    case 22:
    case "EXPRESSION_OP_SUM":
      return ExpressionOp.SUM;
    case 23:
    case "EXPRESSION_OP_AVERAGE":
      return ExpressionOp.AVERAGE;
    case 24:
    case "EXPRESSION_OP_MIN":
      return ExpressionOp.MIN;
    case 25:
    case "EXPRESSION_OP_MAX":
      return ExpressionOp.MAX;
    case 26:
    case "EXPRESSION_OP_MEDIAN":
      return ExpressionOp.MEDIAN;
    case 27:
    case "EXPRESSION_OP_HISTOGRAM":
      return ExpressionOp.HISTOGRAM;
    case 28:
    case "EXPRESSION_OP_ASCENDING":
      return ExpressionOp.ASCENDING;
    case 29:
    case "EXPRESSION_OP_DESCENDING":
      return ExpressionOp.DESCENDING;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ExpressionOp");
  }
}

export function expressionOpToJSON(object: ExpressionOp): string {
  switch (object) {
    case ExpressionOp.UNSPECIFIED:
      return "EXPRESSION_OP_UNSPECIFIED";
    case ExpressionOp.TRUE:
      return "EXPRESSION_OP_TRUE";
    case ExpressionOp.FALSE:
      return "EXPRESSION_OP_FALSE";
    case ExpressionOp.NOT:
      return "EXPRESSION_OP_NOT";
    case ExpressionOp.AND:
      return "EXPRESSION_OP_AND";
    case ExpressionOp.OR:
      return "EXPRESSION_OP_OR";
    case ExpressionOp.EQUALS:
      return "EXPRESSION_OP_EQUALS";
    case ExpressionOp.NOT_EQUALS:
      return "EXPRESSION_OP_NOT_EQUALS";
    case ExpressionOp.GREATER_THAN:
      return "EXPRESSION_OP_GREATER_THAN";
    case ExpressionOp.GREATER_THAN_OR_EQUALS:
      return "EXPRESSION_OP_GREATER_THAN_OR_EQUALS";
    case ExpressionOp.LESS_THAN:
      return "EXPRESSION_OP_LESS_THAN";
    case ExpressionOp.LESS_THAN_OR_EQUALS:
      return "EXPRESSION_OP_LESS_THAN_OR_EQUALS";
    case ExpressionOp.MATCHES:
      return "EXPRESSION_OP_MATCHES";
    case ExpressionOp.STARTS_WITH:
      return "EXPRESSION_OP_STARTS_WITH";
    case ExpressionOp.CONTAINS:
      return "EXPRESSION_OP_CONTAINS";
    case ExpressionOp.NOT_CONTAINS:
      return "EXPRESSION_OP_NOT_CONTAINS";
    case ExpressionOp.IN:
      return "EXPRESSION_OP_IN";
    case ExpressionOp.NOT_IN:
      return "EXPRESSION_OP_NOT_IN";
    case ExpressionOp.EXISTS:
      return "EXPRESSION_OP_EXISTS";
    case ExpressionOp.NOT_EXISTS:
      return "EXPRESSION_OP_NOT_EXISTS";
    case ExpressionOp.NEAR:
      return "EXPRESSION_OP_NEAR";
    case ExpressionOp.COUNT:
      return "EXPRESSION_OP_COUNT";
    case ExpressionOp.SUM:
      return "EXPRESSION_OP_SUM";
    case ExpressionOp.AVERAGE:
      return "EXPRESSION_OP_AVERAGE";
    case ExpressionOp.MIN:
      return "EXPRESSION_OP_MIN";
    case ExpressionOp.MAX:
      return "EXPRESSION_OP_MAX";
    case ExpressionOp.MEDIAN:
      return "EXPRESSION_OP_MEDIAN";
    case ExpressionOp.HISTOGRAM:
      return "EXPRESSION_OP_HISTOGRAM";
    case ExpressionOp.ASCENDING:
      return "EXPRESSION_OP_ASCENDING";
    case ExpressionOp.DESCENDING:
      return "EXPRESSION_OP_DESCENDING";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ExpressionOp");
  }
}

export enum IdentifierType {
  UNSPECIFIED = 0,
  METHOD = 1,
  TYPE = 2,
  CONSTANT = 3,
  PATH = 4,
  VARIABLE = 5,
  FIELD = 6,
}

export function identifierTypeFromJSON(object: any): IdentifierType {
  switch (object) {
    case 0:
    case "IDENTIFIER_TYPE_UNSPECIFIED":
      return IdentifierType.UNSPECIFIED;
    case 1:
    case "IDENTIFIER_TYPE_METHOD":
      return IdentifierType.METHOD;
    case 2:
    case "IDENTIFIER_TYPE_TYPE":
      return IdentifierType.TYPE;
    case 3:
    case "IDENTIFIER_TYPE_CONSTANT":
      return IdentifierType.CONSTANT;
    case 4:
    case "IDENTIFIER_TYPE_PATH":
      return IdentifierType.PATH;
    case 5:
    case "IDENTIFIER_TYPE_VARIABLE":
      return IdentifierType.VARIABLE;
    case 6:
    case "IDENTIFIER_TYPE_FIELD":
      return IdentifierType.FIELD;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IdentifierType");
  }
}

export function identifierTypeToJSON(object: IdentifierType): string {
  switch (object) {
    case IdentifierType.UNSPECIFIED:
      return "IDENTIFIER_TYPE_UNSPECIFIED";
    case IdentifierType.METHOD:
      return "IDENTIFIER_TYPE_METHOD";
    case IdentifierType.TYPE:
      return "IDENTIFIER_TYPE_TYPE";
    case IdentifierType.CONSTANT:
      return "IDENTIFIER_TYPE_CONSTANT";
    case IdentifierType.PATH:
      return "IDENTIFIER_TYPE_PATH";
    case IdentifierType.VARIABLE:
      return "IDENTIFIER_TYPE_VARIABLE";
    case IdentifierType.FIELD:
      return "IDENTIFIER_TYPE_FIELD";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IdentifierType");
  }
}

export enum IssueKind {
  UNSPECIFIED = 0,
  ERROR = 1,
  WARNING = 2,
  NOTICE = 3,
}

export function issueKindFromJSON(object: any): IssueKind {
  switch (object) {
    case 0:
    case "ISSUE_KIND_UNSPECIFIED":
      return IssueKind.UNSPECIFIED;
    case 1:
    case "ISSUE_KIND_ERROR":
      return IssueKind.ERROR;
    case 2:
    case "ISSUE_KIND_WARNING":
      return IssueKind.WARNING;
    case 3:
    case "ISSUE_KIND_NOTICE":
      return IssueKind.NOTICE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IssueKind");
  }
}

export function issueKindToJSON(object: IssueKind): string {
  switch (object) {
    case IssueKind.UNSPECIFIED:
      return "ISSUE_KIND_UNSPECIFIED";
    case IssueKind.ERROR:
      return "ISSUE_KIND_ERROR";
    case IssueKind.WARNING:
      return "ISSUE_KIND_WARNING";
    case IssueKind.NOTICE:
      return "ISSUE_KIND_NOTICE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IssueKind");
  }
}

export enum IssueType {
  UNSPECIFIED = 0,
  INTERNAL = 1,
  UNKNOWN_IMPORT_SOURCE = 2,
  MISSING_REFERENCE = 3,
  CIRCULAR_ANCESTRY = 4,
  CIRCULAR_UNION = 5,
  MISMATCHED_UNION = 6,
  INVALID_DATA = 7,
  AMBIGUOUS_DEFINITION = 8,
  CODE_NOT_EXPORTABLE = 9,
  CODE_NOT_CACHEABLE = 10,
  CODE_REFERENCE_NOT_EXPORTED = 11,
  TASK_MISSING_IO = 12,
  TASK_IS_STATIC = 13,
}

export function issueTypeFromJSON(object: any): IssueType {
  switch (object) {
    case 0:
    case "ISSUE_TYPE_UNSPECIFIED":
      return IssueType.UNSPECIFIED;
    case 1:
    case "ISSUE_TYPE_INTERNAL":
      return IssueType.INTERNAL;
    case 2:
    case "ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE":
      return IssueType.UNKNOWN_IMPORT_SOURCE;
    case 3:
    case "ISSUE_TYPE_MISSING_REFERENCE":
      return IssueType.MISSING_REFERENCE;
    case 4:
    case "ISSUE_TYPE_CIRCULAR_ANCESTRY":
      return IssueType.CIRCULAR_ANCESTRY;
    case 5:
    case "ISSUE_TYPE_CIRCULAR_UNION":
      return IssueType.CIRCULAR_UNION;
    case 6:
    case "ISSUE_TYPE_MISMATCHED_UNION":
      return IssueType.MISMATCHED_UNION;
    case 7:
    case "ISSUE_TYPE_INVALID_DATA":
      return IssueType.INVALID_DATA;
    case 8:
    case "ISSUE_TYPE_AMBIGUOUS_DEFINITION":
      return IssueType.AMBIGUOUS_DEFINITION;
    case 9:
    case "ISSUE_TYPE_CODE_NOT_EXPORTABLE":
      return IssueType.CODE_NOT_EXPORTABLE;
    case 10:
    case "ISSUE_TYPE_CODE_NOT_CACHEABLE":
      return IssueType.CODE_NOT_CACHEABLE;
    case 11:
    case "ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED":
      return IssueType.CODE_REFERENCE_NOT_EXPORTED;
    case 12:
    case "ISSUE_TYPE_TASK_MISSING_IO":
      return IssueType.TASK_MISSING_IO;
    case 13:
    case "ISSUE_TYPE_TASK_IS_STATIC":
      return IssueType.TASK_IS_STATIC;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IssueType");
  }
}

export function issueTypeToJSON(object: IssueType): string {
  switch (object) {
    case IssueType.UNSPECIFIED:
      return "ISSUE_TYPE_UNSPECIFIED";
    case IssueType.INTERNAL:
      return "ISSUE_TYPE_INTERNAL";
    case IssueType.UNKNOWN_IMPORT_SOURCE:
      return "ISSUE_TYPE_UNKNOWN_IMPORT_SOURCE";
    case IssueType.MISSING_REFERENCE:
      return "ISSUE_TYPE_MISSING_REFERENCE";
    case IssueType.CIRCULAR_ANCESTRY:
      return "ISSUE_TYPE_CIRCULAR_ANCESTRY";
    case IssueType.CIRCULAR_UNION:
      return "ISSUE_TYPE_CIRCULAR_UNION";
    case IssueType.MISMATCHED_UNION:
      return "ISSUE_TYPE_MISMATCHED_UNION";
    case IssueType.INVALID_DATA:
      return "ISSUE_TYPE_INVALID_DATA";
    case IssueType.AMBIGUOUS_DEFINITION:
      return "ISSUE_TYPE_AMBIGUOUS_DEFINITION";
    case IssueType.CODE_NOT_EXPORTABLE:
      return "ISSUE_TYPE_CODE_NOT_EXPORTABLE";
    case IssueType.CODE_NOT_CACHEABLE:
      return "ISSUE_TYPE_CODE_NOT_CACHEABLE";
    case IssueType.CODE_REFERENCE_NOT_EXPORTED:
      return "ISSUE_TYPE_CODE_REFERENCE_NOT_EXPORTED";
    case IssueType.TASK_MISSING_IO:
      return "ISSUE_TYPE_TASK_MISSING_IO";
    case IssueType.TASK_IS_STATIC:
      return "ISSUE_TYPE_TASK_IS_STATIC";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum IssueType");
  }
}

export enum NodeTrackingLevel {
  NONE = 0,
  ANONYMOUS = 1,
  FULL = 2,
}

export function nodeTrackingLevelFromJSON(object: any): NodeTrackingLevel {
  switch (object) {
    case 0:
    case "NODE_TRACKING_LEVEL_NONE":
      return NodeTrackingLevel.NONE;
    case 1:
    case "NODE_TRACKING_LEVEL_ANONYMOUS":
      return NodeTrackingLevel.ANONYMOUS;
    case 2:
    case "NODE_TRACKING_LEVEL_FULL":
      return NodeTrackingLevel.FULL;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum NodeTrackingLevel");
  }
}

export function nodeTrackingLevelToJSON(object: NodeTrackingLevel): string {
  switch (object) {
    case NodeTrackingLevel.NONE:
      return "NODE_TRACKING_LEVEL_NONE";
    case NodeTrackingLevel.ANONYMOUS:
      return "NODE_TRACKING_LEVEL_ANONYMOUS";
    case NodeTrackingLevel.FULL:
      return "NODE_TRACKING_LEVEL_FULL";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum NodeTrackingLevel");
  }
}

export enum NodeType {
  UNSPECIFIED = 0,
  MODULE = 1,
  FILE = 2,
  STATEMENT = 3,
  TRIGGER = 4,
  TAGGING = 5,
  FIELD = 6,
  RECORD = 7,
  VIEW = 8,
  ISSUE = 9,
  RESOLVED_FIELD = 10,
  BLOB = 11,
  SECRET = 12,
  SESSION = 13,
  RUN = 14,
}

export function nodeTypeFromJSON(object: any): NodeType {
  switch (object) {
    case 0:
    case "NODE_TYPE_UNSPECIFIED":
      return NodeType.UNSPECIFIED;
    case 1:
    case "NODE_TYPE_MODULE":
      return NodeType.MODULE;
    case 2:
    case "NODE_TYPE_FILE":
      return NodeType.FILE;
    case 3:
    case "NODE_TYPE_STATEMENT":
      return NodeType.STATEMENT;
    case 4:
    case "NODE_TYPE_TRIGGER":
      return NodeType.TRIGGER;
    case 5:
    case "NODE_TYPE_TAGGING":
      return NodeType.TAGGING;
    case 6:
    case "NODE_TYPE_FIELD":
      return NodeType.FIELD;
    case 7:
    case "NODE_TYPE_RECORD":
      return NodeType.RECORD;
    case 8:
    case "NODE_TYPE_VIEW":
      return NodeType.VIEW;
    case 9:
    case "NODE_TYPE_ISSUE":
      return NodeType.ISSUE;
    case 10:
    case "NODE_TYPE_RESOLVED_FIELD":
      return NodeType.RESOLVED_FIELD;
    case 11:
    case "NODE_TYPE_BLOB":
      return NodeType.BLOB;
    case 12:
    case "NODE_TYPE_SECRET":
      return NodeType.SECRET;
    case 13:
    case "NODE_TYPE_SESSION":
      return NodeType.SESSION;
    case 14:
    case "NODE_TYPE_RUN":
      return NodeType.RUN;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum NodeType");
  }
}

export function nodeTypeToJSON(object: NodeType): string {
  switch (object) {
    case NodeType.UNSPECIFIED:
      return "NODE_TYPE_UNSPECIFIED";
    case NodeType.MODULE:
      return "NODE_TYPE_MODULE";
    case NodeType.FILE:
      return "NODE_TYPE_FILE";
    case NodeType.STATEMENT:
      return "NODE_TYPE_STATEMENT";
    case NodeType.TRIGGER:
      return "NODE_TYPE_TRIGGER";
    case NodeType.TAGGING:
      return "NODE_TYPE_TAGGING";
    case NodeType.FIELD:
      return "NODE_TYPE_FIELD";
    case NodeType.RECORD:
      return "NODE_TYPE_RECORD";
    case NodeType.VIEW:
      return "NODE_TYPE_VIEW";
    case NodeType.ISSUE:
      return "NODE_TYPE_ISSUE";
    case NodeType.RESOLVED_FIELD:
      return "NODE_TYPE_RESOLVED_FIELD";
    case NodeType.BLOB:
      return "NODE_TYPE_BLOB";
    case NodeType.SECRET:
      return "NODE_TYPE_SECRET";
    case NodeType.SESSION:
      return "NODE_TYPE_SESSION";
    case NodeType.RUN:
      return "NODE_TYPE_RUN";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum NodeType");
  }
}

export enum ProjectRegion {
  UNSPECIFIED = 0,
  US_WEST = 1,
  EU_CENTRAL = 2,
}

export function projectRegionFromJSON(object: any): ProjectRegion {
  switch (object) {
    case 0:
    case "PROJECT_REGION_UNSPECIFIED":
      return ProjectRegion.UNSPECIFIED;
    case 1:
    case "PROJECT_REGION_US_WEST":
      return ProjectRegion.US_WEST;
    case 2:
    case "PROJECT_REGION_EU_CENTRAL":
      return ProjectRegion.EU_CENTRAL;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ProjectRegion");
  }
}

export function projectRegionToJSON(object: ProjectRegion): string {
  switch (object) {
    case ProjectRegion.UNSPECIFIED:
      return "PROJECT_REGION_UNSPECIFIED";
    case ProjectRegion.US_WEST:
      return "PROJECT_REGION_US_WEST";
    case ProjectRegion.EU_CENTRAL:
      return "PROJECT_REGION_EU_CENTRAL";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ProjectRegion");
  }
}

export enum QueryEngine {
  UNSPECIFIED = 0,
  MODULE = 1,
  HOST = 2,
  OPENSEARCH = 3,
  POSTGRES = 4,
}

export function queryEngineFromJSON(object: any): QueryEngine {
  switch (object) {
    case 0:
    case "QUERY_ENGINE_UNSPECIFIED":
      return QueryEngine.UNSPECIFIED;
    case 1:
    case "QUERY_ENGINE_MODULE":
      return QueryEngine.MODULE;
    case 2:
    case "QUERY_ENGINE_HOST":
      return QueryEngine.HOST;
    case 3:
    case "QUERY_ENGINE_OPENSEARCH":
      return QueryEngine.OPENSEARCH;
    case 4:
    case "QUERY_ENGINE_POSTGRES":
      return QueryEngine.POSTGRES;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum QueryEngine");
  }
}

export function queryEngineToJSON(object: QueryEngine): string {
  switch (object) {
    case QueryEngine.UNSPECIFIED:
      return "QUERY_ENGINE_UNSPECIFIED";
    case QueryEngine.MODULE:
      return "QUERY_ENGINE_MODULE";
    case QueryEngine.HOST:
      return "QUERY_ENGINE_HOST";
    case QueryEngine.OPENSEARCH:
      return "QUERY_ENGINE_OPENSEARCH";
    case QueryEngine.POSTGRES:
      return "QUERY_ENGINE_POSTGRES";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum QueryEngine");
  }
}

export enum RunErrorKind {
  UNSPECIFIED = 0,
  Internal = 1,
  Parse = 2,
  Validation = 3,
  Runtime = 4,
  Untrusted = 5,
}

export function runErrorKindFromJSON(object: any): RunErrorKind {
  switch (object) {
    case 0:
    case "RUN_ERROR_KIND_UNSPECIFIED":
      return RunErrorKind.UNSPECIFIED;
    case 1:
    case "RUN_ERROR_KIND_Internal":
      return RunErrorKind.Internal;
    case 2:
    case "RUN_ERROR_KIND_Parse":
      return RunErrorKind.Parse;
    case 3:
    case "RUN_ERROR_KIND_Validation":
      return RunErrorKind.Validation;
    case 4:
    case "RUN_ERROR_KIND_Runtime":
      return RunErrorKind.Runtime;
    case 5:
    case "RUN_ERROR_KIND_Untrusted":
      return RunErrorKind.Untrusted;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum RunErrorKind");
  }
}

export function runErrorKindToJSON(object: RunErrorKind): string {
  switch (object) {
    case RunErrorKind.UNSPECIFIED:
      return "RUN_ERROR_KIND_UNSPECIFIED";
    case RunErrorKind.Internal:
      return "RUN_ERROR_KIND_Internal";
    case RunErrorKind.Parse:
      return "RUN_ERROR_KIND_Parse";
    case RunErrorKind.Validation:
      return "RUN_ERROR_KIND_Validation";
    case RunErrorKind.Runtime:
      return "RUN_ERROR_KIND_Runtime";
    case RunErrorKind.Untrusted:
      return "RUN_ERROR_KIND_Untrusted";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum RunErrorKind");
  }
}

export enum RunStatus {
  UNSPECIFIED = 0,
  SCHEDULED = 1,
  QUEUED = 2,
  RUNNING = 3,
  SUSPENDED = 4,
  ABORTING = 5,
  CANCELLED = 6,
  ABORTED = 7,
  FAILED = 8,
  COMPLETED = 9,
}

export function runStatusFromJSON(object: any): RunStatus {
  switch (object) {
    case 0:
    case "RUN_STATUS_UNSPECIFIED":
      return RunStatus.UNSPECIFIED;
    case 1:
    case "RUN_STATUS_SCHEDULED":
      return RunStatus.SCHEDULED;
    case 2:
    case "RUN_STATUS_QUEUED":
      return RunStatus.QUEUED;
    case 3:
    case "RUN_STATUS_RUNNING":
      return RunStatus.RUNNING;
    case 4:
    case "RUN_STATUS_SUSPENDED":
      return RunStatus.SUSPENDED;
    case 5:
    case "RUN_STATUS_ABORTING":
      return RunStatus.ABORTING;
    case 6:
    case "RUN_STATUS_CANCELLED":
      return RunStatus.CANCELLED;
    case 7:
    case "RUN_STATUS_ABORTED":
      return RunStatus.ABORTED;
    case 8:
    case "RUN_STATUS_FAILED":
      return RunStatus.FAILED;
    case 9:
    case "RUN_STATUS_COMPLETED":
      return RunStatus.COMPLETED;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum RunStatus");
  }
}

export function runStatusToJSON(object: RunStatus): string {
  switch (object) {
    case RunStatus.UNSPECIFIED:
      return "RUN_STATUS_UNSPECIFIED";
    case RunStatus.SCHEDULED:
      return "RUN_STATUS_SCHEDULED";
    case RunStatus.QUEUED:
      return "RUN_STATUS_QUEUED";
    case RunStatus.RUNNING:
      return "RUN_STATUS_RUNNING";
    case RunStatus.SUSPENDED:
      return "RUN_STATUS_SUSPENDED";
    case RunStatus.ABORTING:
      return "RUN_STATUS_ABORTING";
    case RunStatus.CANCELLED:
      return "RUN_STATUS_CANCELLED";
    case RunStatus.ABORTED:
      return "RUN_STATUS_ABORTED";
    case RunStatus.FAILED:
      return "RUN_STATUS_FAILED";
    case RunStatus.COMPLETED:
      return "RUN_STATUS_COMPLETED";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum RunStatus");
  }
}

export enum ScheduleType {
  UNSPECIFIED = 0,
  INTERVAL = 1,
  CRON = 2,
}

export function scheduleTypeFromJSON(object: any): ScheduleType {
  switch (object) {
    case 0:
    case "SCHEDULE_TYPE_UNSPECIFIED":
      return ScheduleType.UNSPECIFIED;
    case 1:
    case "SCHEDULE_TYPE_INTERVAL":
      return ScheduleType.INTERVAL;
    case 2:
    case "SCHEDULE_TYPE_CRON":
      return ScheduleType.CRON;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ScheduleType");
  }
}

export function scheduleTypeToJSON(object: ScheduleType): string {
  switch (object) {
    case ScheduleType.UNSPECIFIED:
      return "SCHEDULE_TYPE_UNSPECIFIED";
    case ScheduleType.INTERVAL:
      return "SCHEDULE_TYPE_INTERVAL";
    case ScheduleType.CRON:
      return "SCHEDULE_TYPE_CRON";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ScheduleType");
  }
}

export enum SessionAccessLevel {
  Zero = 0,
  Read = 1,
  Create = 2,
  Update = 3,
  Delete = 4,
  Full = 4,
}

export function sessionAccessLevelFromJSON(object: any): SessionAccessLevel {
  switch (object) {
    case 0:
    case "SESSION_ACCESS_LEVEL_Zero":
      return SessionAccessLevel.Zero;
    case 1:
    case "SESSION_ACCESS_LEVEL_Read":
      return SessionAccessLevel.Read;
    case 2:
    case "SESSION_ACCESS_LEVEL_Create":
      return SessionAccessLevel.Create;
    case 3:
    case "SESSION_ACCESS_LEVEL_Update":
      return SessionAccessLevel.Update;
    case 4:
    case "SESSION_ACCESS_LEVEL_Delete":
      return SessionAccessLevel.Delete;
    case 4:
    case "SESSION_ACCESS_LEVEL_Full":
      return SessionAccessLevel.Full;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SessionAccessLevel");
  }
}

export function sessionAccessLevelToJSON(object: SessionAccessLevel): string {
  switch (object) {
    case SessionAccessLevel.Zero:
      return "SESSION_ACCESS_LEVEL_Zero";
    case SessionAccessLevel.Read:
      return "SESSION_ACCESS_LEVEL_Read";
    case SessionAccessLevel.Create:
      return "SESSION_ACCESS_LEVEL_Create";
    case SessionAccessLevel.Update:
      return "SESSION_ACCESS_LEVEL_Update";
    case SessionAccessLevel.Delete:
      return "SESSION_ACCESS_LEVEL_Delete";
    case SessionAccessLevel.Full:
      return "SESSION_ACCESS_LEVEL_Full";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SessionAccessLevel");
  }
}

export enum SessionStatus {
  UNSPECIFIED = 0,
  ACTIVE = 1,
  SUSPENDED = 2,
  TERMINATED = 3,
}

export function sessionStatusFromJSON(object: any): SessionStatus {
  switch (object) {
    case 0:
    case "SESSION_STATUS_UNSPECIFIED":
      return SessionStatus.UNSPECIFIED;
    case 1:
    case "SESSION_STATUS_ACTIVE":
      return SessionStatus.ACTIVE;
    case 2:
    case "SESSION_STATUS_SUSPENDED":
      return SessionStatus.SUSPENDED;
    case 3:
    case "SESSION_STATUS_TERMINATED":
      return SessionStatus.TERMINATED;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SessionStatus");
  }
}

export function sessionStatusToJSON(object: SessionStatus): string {
  switch (object) {
    case SessionStatus.UNSPECIFIED:
      return "SESSION_STATUS_UNSPECIFIED";
    case SessionStatus.ACTIVE:
      return "SESSION_STATUS_ACTIVE";
    case SessionStatus.SUSPENDED:
      return "SESSION_STATUS_SUSPENDED";
    case SessionStatus.TERMINATED:
      return "SESSION_STATUS_TERMINATED";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SessionStatus");
  }
}

export enum SortMode {
  UNSPECIFIED = 0,
  MAX = 1,
  MIN = 2,
  AVERAGE = 3,
  SUM = 4,
  MEDIAN = 5,
}

export function sortModeFromJSON(object: any): SortMode {
  switch (object) {
    case 0:
    case "SORT_MODE_UNSPECIFIED":
      return SortMode.UNSPECIFIED;
    case 1:
    case "SORT_MODE_MAX":
      return SortMode.MAX;
    case 2:
    case "SORT_MODE_MIN":
      return SortMode.MIN;
    case 3:
    case "SORT_MODE_AVERAGE":
      return SortMode.AVERAGE;
    case 4:
    case "SORT_MODE_SUM":
      return SortMode.SUM;
    case 5:
    case "SORT_MODE_MEDIAN":
      return SortMode.MEDIAN;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SortMode");
  }
}

export function sortModeToJSON(object: SortMode): string {
  switch (object) {
    case SortMode.UNSPECIFIED:
      return "SORT_MODE_UNSPECIFIED";
    case SortMode.MAX:
      return "SORT_MODE_MAX";
    case SortMode.MIN:
      return "SORT_MODE_MIN";
    case SortMode.AVERAGE:
      return "SORT_MODE_AVERAGE";
    case SortMode.SUM:
      return "SORT_MODE_SUM";
    case SortMode.MEDIAN:
      return "SORT_MODE_MEDIAN";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SortMode");
  }
}

export enum SortOp {
  UNSPECIFIED = 0,
  ASCENDING = 1,
  DESCENDING = 2,
}

export function sortOpFromJSON(object: any): SortOp {
  switch (object) {
    case 0:
    case "SORT_OP_UNSPECIFIED":
      return SortOp.UNSPECIFIED;
    case 1:
    case "SORT_OP_ASCENDING":
      return SortOp.ASCENDING;
    case 2:
    case "SORT_OP_DESCENDING":
      return SortOp.DESCENDING;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SortOp");
  }
}

export function sortOpToJSON(object: SortOp): string {
  switch (object) {
    case SortOp.UNSPECIFIED:
      return "SORT_OP_UNSPECIFIED";
    case SortOp.ASCENDING:
      return "SORT_OP_ASCENDING";
    case SortOp.DESCENDING:
      return "SORT_OP_DESCENDING";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum SortOp");
  }
}

export enum StatementType {
  UNSPECIFIED = 0,
  TAG = 1,
  TEXT = 2,
  BLANK = 3,
  CLASS = 4,
  CHOICE = 5,
  TASK = 6,
  CODE = 7,
  FLOW = 8,
  MODEL = 9,
  VARIABLE = 10,
  DATABASE = 11,
  VIEW = 12,
  GROUP = 13,
}

export function statementTypeFromJSON(object: any): StatementType {
  switch (object) {
    case 0:
    case "STATEMENT_TYPE_UNSPECIFIED":
      return StatementType.UNSPECIFIED;
    case 1:
    case "STATEMENT_TYPE_TAG":
      return StatementType.TAG;
    case 2:
    case "STATEMENT_TYPE_TEXT":
      return StatementType.TEXT;
    case 3:
    case "STATEMENT_TYPE_BLANK":
      return StatementType.BLANK;
    case 4:
    case "STATEMENT_TYPE_CLASS":
      return StatementType.CLASS;
    case 5:
    case "STATEMENT_TYPE_CHOICE":
      return StatementType.CHOICE;
    case 6:
    case "STATEMENT_TYPE_TASK":
      return StatementType.TASK;
    case 7:
    case "STATEMENT_TYPE_CODE":
      return StatementType.CODE;
    case 8:
    case "STATEMENT_TYPE_FLOW":
      return StatementType.FLOW;
    case 9:
    case "STATEMENT_TYPE_MODEL":
      return StatementType.MODEL;
    case 10:
    case "STATEMENT_TYPE_VARIABLE":
      return StatementType.VARIABLE;
    case 11:
    case "STATEMENT_TYPE_DATABASE":
      return StatementType.DATABASE;
    case 12:
    case "STATEMENT_TYPE_VIEW":
      return StatementType.VIEW;
    case 13:
    case "STATEMENT_TYPE_GROUP":
      return StatementType.GROUP;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StatementType");
  }
}

export function statementTypeToJSON(object: StatementType): string {
  switch (object) {
    case StatementType.UNSPECIFIED:
      return "STATEMENT_TYPE_UNSPECIFIED";
    case StatementType.TAG:
      return "STATEMENT_TYPE_TAG";
    case StatementType.TEXT:
      return "STATEMENT_TYPE_TEXT";
    case StatementType.BLANK:
      return "STATEMENT_TYPE_BLANK";
    case StatementType.CLASS:
      return "STATEMENT_TYPE_CLASS";
    case StatementType.CHOICE:
      return "STATEMENT_TYPE_CHOICE";
    case StatementType.TASK:
      return "STATEMENT_TYPE_TASK";
    case StatementType.CODE:
      return "STATEMENT_TYPE_CODE";
    case StatementType.FLOW:
      return "STATEMENT_TYPE_FLOW";
    case StatementType.MODEL:
      return "STATEMENT_TYPE_MODEL";
    case StatementType.VARIABLE:
      return "STATEMENT_TYPE_VARIABLE";
    case StatementType.DATABASE:
      return "STATEMENT_TYPE_DATABASE";
    case StatementType.VIEW:
      return "STATEMENT_TYPE_VIEW";
    case StatementType.GROUP:
      return "STATEMENT_TYPE_GROUP";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StatementType");
  }
}

export enum StructType {
  UNSPECIFIED = 0,
  EXPRESSION = 1,
  RUN_CODE_FRAME = 2,
  RUN_ERROR = 3,
  LOG_ENTRY = 4,
  WORKER_SET = 5,
  ENVIRONMENT = 6,
}

export function structTypeFromJSON(object: any): StructType {
  switch (object) {
    case 0:
    case "STRUCT_TYPE_UNSPECIFIED":
      return StructType.UNSPECIFIED;
    case 1:
    case "STRUCT_TYPE_EXPRESSION":
      return StructType.EXPRESSION;
    case 2:
    case "STRUCT_TYPE_RUN_CODE_FRAME":
      return StructType.RUN_CODE_FRAME;
    case 3:
    case "STRUCT_TYPE_RUN_ERROR":
      return StructType.RUN_ERROR;
    case 4:
    case "STRUCT_TYPE_LOG_ENTRY":
      return StructType.LOG_ENTRY;
    case 5:
    case "STRUCT_TYPE_WORKER_SET":
      return StructType.WORKER_SET;
    case 6:
    case "STRUCT_TYPE_ENVIRONMENT":
      return StructType.ENVIRONMENT;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StructType");
  }
}

export function structTypeToJSON(object: StructType): string {
  switch (object) {
    case StructType.UNSPECIFIED:
      return "STRUCT_TYPE_UNSPECIFIED";
    case StructType.EXPRESSION:
      return "STRUCT_TYPE_EXPRESSION";
    case StructType.RUN_CODE_FRAME:
      return "STRUCT_TYPE_RUN_CODE_FRAME";
    case StructType.RUN_ERROR:
      return "STRUCT_TYPE_RUN_ERROR";
    case StructType.LOG_ENTRY:
      return "STRUCT_TYPE_LOG_ENTRY";
    case StructType.WORKER_SET:
      return "STRUCT_TYPE_WORKER_SET";
    case StructType.ENVIRONMENT:
      return "STRUCT_TYPE_ENVIRONMENT";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StructType");
  }
}

export enum TextHeadingLevel {
  UNSPECIFIED = 0,
  H1 = 1,
  H2 = 2,
  H3 = 3,
}

export function textHeadingLevelFromJSON(object: any): TextHeadingLevel {
  switch (object) {
    case 0:
    case "TEXT_HEADING_LEVEL_UNSPECIFIED":
      return TextHeadingLevel.UNSPECIFIED;
    case 1:
    case "TEXT_HEADING_LEVEL_H1":
      return TextHeadingLevel.H1;
    case 2:
    case "TEXT_HEADING_LEVEL_H2":
      return TextHeadingLevel.H2;
    case 3:
    case "TEXT_HEADING_LEVEL_H3":
      return TextHeadingLevel.H3;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TextHeadingLevel");
  }
}

export function textHeadingLevelToJSON(object: TextHeadingLevel): string {
  switch (object) {
    case TextHeadingLevel.UNSPECIFIED:
      return "TEXT_HEADING_LEVEL_UNSPECIFIED";
    case TextHeadingLevel.H1:
      return "TEXT_HEADING_LEVEL_H1";
    case TextHeadingLevel.H2:
      return "TEXT_HEADING_LEVEL_H2";
    case TextHeadingLevel.H3:
      return "TEXT_HEADING_LEVEL_H3";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TextHeadingLevel");
  }
}

export enum TriggerType {
  UNSPECIFIED = 0,
  INVOKE = 1,
  TIME = 2,
  RUN = 3,
  EDIT = 4,
  MESSAGE = 5,
  USER = 6,
  API = 7,
}

export function triggerTypeFromJSON(object: any): TriggerType {
  switch (object) {
    case 0:
    case "TRIGGER_TYPE_UNSPECIFIED":
      return TriggerType.UNSPECIFIED;
    case 1:
    case "TRIGGER_TYPE_INVOKE":
      return TriggerType.INVOKE;
    case 2:
    case "TRIGGER_TYPE_TIME":
      return TriggerType.TIME;
    case 3:
    case "TRIGGER_TYPE_RUN":
      return TriggerType.RUN;
    case 4:
    case "TRIGGER_TYPE_EDIT":
      return TriggerType.EDIT;
    case 5:
    case "TRIGGER_TYPE_MESSAGE":
      return TriggerType.MESSAGE;
    case 6:
    case "TRIGGER_TYPE_USER":
      return TriggerType.USER;
    case 7:
    case "TRIGGER_TYPE_API":
      return TriggerType.API;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TriggerType");
  }
}

export function triggerTypeToJSON(object: TriggerType): string {
  switch (object) {
    case TriggerType.UNSPECIFIED:
      return "TRIGGER_TYPE_UNSPECIFIED";
    case TriggerType.INVOKE:
      return "TRIGGER_TYPE_INVOKE";
    case TriggerType.TIME:
      return "TRIGGER_TYPE_TIME";
    case TriggerType.RUN:
      return "TRIGGER_TYPE_RUN";
    case TriggerType.EDIT:
      return "TRIGGER_TYPE_EDIT";
    case TriggerType.MESSAGE:
      return "TRIGGER_TYPE_MESSAGE";
    case TriggerType.USER:
      return "TRIGGER_TYPE_USER";
    case TriggerType.API:
      return "TRIGGER_TYPE_API";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TriggerType");
  }
}

export enum TypeFlag {
  ZERO = 0,
  IS_OUTPUT = 1,
  IS_ARRAY = 2,
  IS_OPTIONAL = 4,
  IS_UNION_WITH = 8,
  IS_SECRET = 16,
  IS_STORE_ONLY = 32,
  IS_ARRAYABLE = 64,
  IS_META = 128,
  IS_CONFIG = 256,
  IS_HIDDEN = 512,
}

export function typeFlagFromJSON(object: any): TypeFlag {
  switch (object) {
    case 0:
    case "TYPE_FLAG_ZERO":
      return TypeFlag.ZERO;
    case 1:
    case "TYPE_FLAG_IS_OUTPUT":
      return TypeFlag.IS_OUTPUT;
    case 2:
    case "TYPE_FLAG_IS_ARRAY":
      return TypeFlag.IS_ARRAY;
    case 4:
    case "TYPE_FLAG_IS_OPTIONAL":
      return TypeFlag.IS_OPTIONAL;
    case 8:
    case "TYPE_FLAG_IS_UNION_WITH":
      return TypeFlag.IS_UNION_WITH;
    case 16:
    case "TYPE_FLAG_IS_SECRET":
      return TypeFlag.IS_SECRET;
    case 32:
    case "TYPE_FLAG_IS_STORE_ONLY":
      return TypeFlag.IS_STORE_ONLY;
    case 64:
    case "TYPE_FLAG_IS_ARRAYABLE":
      return TypeFlag.IS_ARRAYABLE;
    case 128:
    case "TYPE_FLAG_IS_META":
      return TypeFlag.IS_META;
    case 256:
    case "TYPE_FLAG_IS_CONFIG":
      return TypeFlag.IS_CONFIG;
    case 512:
    case "TYPE_FLAG_IS_HIDDEN":
      return TypeFlag.IS_HIDDEN;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeFlag");
  }
}

export function typeFlagToJSON(object: TypeFlag): string {
  switch (object) {
    case TypeFlag.ZERO:
      return "TYPE_FLAG_ZERO";
    case TypeFlag.IS_OUTPUT:
      return "TYPE_FLAG_IS_OUTPUT";
    case TypeFlag.IS_ARRAY:
      return "TYPE_FLAG_IS_ARRAY";
    case TypeFlag.IS_OPTIONAL:
      return "TYPE_FLAG_IS_OPTIONAL";
    case TypeFlag.IS_UNION_WITH:
      return "TYPE_FLAG_IS_UNION_WITH";
    case TypeFlag.IS_SECRET:
      return "TYPE_FLAG_IS_SECRET";
    case TypeFlag.IS_STORE_ONLY:
      return "TYPE_FLAG_IS_STORE_ONLY";
    case TypeFlag.IS_ARRAYABLE:
      return "TYPE_FLAG_IS_ARRAYABLE";
    case TypeFlag.IS_META:
      return "TYPE_FLAG_IS_META";
    case TypeFlag.IS_CONFIG:
      return "TYPE_FLAG_IS_CONFIG";
    case TypeFlag.IS_HIDDEN:
      return "TYPE_FLAG_IS_HIDDEN";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeFlag");
  }
}

export enum TypeHint {
  UNSPECIFIED = 0,
  NAME = 1,
  UUID = 2,
  DATE = 3,
  DATETIME = 4,
  TIME = 5,
  DURATION = 6,
  EMAIL = 7,
  URL = 8,
  MARKDOWN = 9,
  RICH_TEXT = 10,
  HTML = 11,
  CODE = 12,
  KEY = 13,
  INTEGER = 14,
  FLOAT = 15,
  SLIDER = 16,
  PHONE = 17,
  RATING = 18,
  TOGGLE = 19,
  CHECKBOX = 20,
  THUMBS = 21,
  EMBEDDING = 22,
  IMAGE = 23,
  VIDEO = 24,
  AUDIO = 25,
  FILE = 26,
  STATEMENT = 27,
  RECORD = 28,
  FIELD = 29,
  RUN = 30,
  SECRET = 31,
  BLOB = 32,
}

export function typeHintFromJSON(object: any): TypeHint {
  switch (object) {
    case 0:
    case "TYPE_HINT_UNSPECIFIED":
      return TypeHint.UNSPECIFIED;
    case 1:
    case "TYPE_HINT_NAME":
      return TypeHint.NAME;
    case 2:
    case "TYPE_HINT_UUID":
      return TypeHint.UUID;
    case 3:
    case "TYPE_HINT_DATE":
      return TypeHint.DATE;
    case 4:
    case "TYPE_HINT_DATETIME":
      return TypeHint.DATETIME;
    case 5:
    case "TYPE_HINT_TIME":
      return TypeHint.TIME;
    case 6:
    case "TYPE_HINT_DURATION":
      return TypeHint.DURATION;
    case 7:
    case "TYPE_HINT_EMAIL":
      return TypeHint.EMAIL;
    case 8:
    case "TYPE_HINT_URL":
      return TypeHint.URL;
    case 9:
    case "TYPE_HINT_MARKDOWN":
      return TypeHint.MARKDOWN;
    case 10:
    case "TYPE_HINT_RICH_TEXT":
      return TypeHint.RICH_TEXT;
    case 11:
    case "TYPE_HINT_HTML":
      return TypeHint.HTML;
    case 12:
    case "TYPE_HINT_CODE":
      return TypeHint.CODE;
    case 13:
    case "TYPE_HINT_KEY":
      return TypeHint.KEY;
    case 14:
    case "TYPE_HINT_INTEGER":
      return TypeHint.INTEGER;
    case 15:
    case "TYPE_HINT_FLOAT":
      return TypeHint.FLOAT;
    case 16:
    case "TYPE_HINT_SLIDER":
      return TypeHint.SLIDER;
    case 17:
    case "TYPE_HINT_PHONE":
      return TypeHint.PHONE;
    case 18:
    case "TYPE_HINT_RATING":
      return TypeHint.RATING;
    case 19:
    case "TYPE_HINT_TOGGLE":
      return TypeHint.TOGGLE;
    case 20:
    case "TYPE_HINT_CHECKBOX":
      return TypeHint.CHECKBOX;
    case 21:
    case "TYPE_HINT_THUMBS":
      return TypeHint.THUMBS;
    case 22:
    case "TYPE_HINT_EMBEDDING":
      return TypeHint.EMBEDDING;
    case 23:
    case "TYPE_HINT_IMAGE":
      return TypeHint.IMAGE;
    case 24:
    case "TYPE_HINT_VIDEO":
      return TypeHint.VIDEO;
    case 25:
    case "TYPE_HINT_AUDIO":
      return TypeHint.AUDIO;
    case 26:
    case "TYPE_HINT_FILE":
      return TypeHint.FILE;
    case 27:
    case "TYPE_HINT_STATEMENT":
      return TypeHint.STATEMENT;
    case 28:
    case "TYPE_HINT_RECORD":
      return TypeHint.RECORD;
    case 29:
    case "TYPE_HINT_FIELD":
      return TypeHint.FIELD;
    case 30:
    case "TYPE_HINT_RUN":
      return TypeHint.RUN;
    case 31:
    case "TYPE_HINT_SECRET":
      return TypeHint.SECRET;
    case 32:
    case "TYPE_HINT_BLOB":
      return TypeHint.BLOB;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeHint");
  }
}

export function typeHintToJSON(object: TypeHint): string {
  switch (object) {
    case TypeHint.UNSPECIFIED:
      return "TYPE_HINT_UNSPECIFIED";
    case TypeHint.NAME:
      return "TYPE_HINT_NAME";
    case TypeHint.UUID:
      return "TYPE_HINT_UUID";
    case TypeHint.DATE:
      return "TYPE_HINT_DATE";
    case TypeHint.DATETIME:
      return "TYPE_HINT_DATETIME";
    case TypeHint.TIME:
      return "TYPE_HINT_TIME";
    case TypeHint.DURATION:
      return "TYPE_HINT_DURATION";
    case TypeHint.EMAIL:
      return "TYPE_HINT_EMAIL";
    case TypeHint.URL:
      return "TYPE_HINT_URL";
    case TypeHint.MARKDOWN:
      return "TYPE_HINT_MARKDOWN";
    case TypeHint.RICH_TEXT:
      return "TYPE_HINT_RICH_TEXT";
    case TypeHint.HTML:
      return "TYPE_HINT_HTML";
    case TypeHint.CODE:
      return "TYPE_HINT_CODE";
    case TypeHint.KEY:
      return "TYPE_HINT_KEY";
    case TypeHint.INTEGER:
      return "TYPE_HINT_INTEGER";
    case TypeHint.FLOAT:
      return "TYPE_HINT_FLOAT";
    case TypeHint.SLIDER:
      return "TYPE_HINT_SLIDER";
    case TypeHint.PHONE:
      return "TYPE_HINT_PHONE";
    case TypeHint.RATING:
      return "TYPE_HINT_RATING";
    case TypeHint.TOGGLE:
      return "TYPE_HINT_TOGGLE";
    case TypeHint.CHECKBOX:
      return "TYPE_HINT_CHECKBOX";
    case TypeHint.THUMBS:
      return "TYPE_HINT_THUMBS";
    case TypeHint.EMBEDDING:
      return "TYPE_HINT_EMBEDDING";
    case TypeHint.IMAGE:
      return "TYPE_HINT_IMAGE";
    case TypeHint.VIDEO:
      return "TYPE_HINT_VIDEO";
    case TypeHint.AUDIO:
      return "TYPE_HINT_AUDIO";
    case TypeHint.FILE:
      return "TYPE_HINT_FILE";
    case TypeHint.STATEMENT:
      return "TYPE_HINT_STATEMENT";
    case TypeHint.RECORD:
      return "TYPE_HINT_RECORD";
    case TypeHint.FIELD:
      return "TYPE_HINT_FIELD";
    case TypeHint.RUN:
      return "TYPE_HINT_RUN";
    case TypeHint.SECRET:
      return "TYPE_HINT_SECRET";
    case TypeHint.BLOB:
      return "TYPE_HINT_BLOB";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeHint");
  }
}

export enum TypeStorageFormat {
  UNSPECIFIED = 0,
  STRING = 1,
  DOUBLE = 2,
  LONG = 3,
  VECTOR = 4,
  BINARY = 5,
  BOOLEAN = 6,
  DATE = 7,
  KEYWORD = 8,
  OBJECT = 9,
  RELATION = 10,
}

export function typeStorageFormatFromJSON(object: any): TypeStorageFormat {
  switch (object) {
    case 0:
    case "TYPE_STORAGE_FORMAT_UNSPECIFIED":
      return TypeStorageFormat.UNSPECIFIED;
    case 1:
    case "TYPE_STORAGE_FORMAT_STRING":
      return TypeStorageFormat.STRING;
    case 2:
    case "TYPE_STORAGE_FORMAT_DOUBLE":
      return TypeStorageFormat.DOUBLE;
    case 3:
    case "TYPE_STORAGE_FORMAT_LONG":
      return TypeStorageFormat.LONG;
    case 4:
    case "TYPE_STORAGE_FORMAT_VECTOR":
      return TypeStorageFormat.VECTOR;
    case 5:
    case "TYPE_STORAGE_FORMAT_BINARY":
      return TypeStorageFormat.BINARY;
    case 6:
    case "TYPE_STORAGE_FORMAT_BOOLEAN":
      return TypeStorageFormat.BOOLEAN;
    case 7:
    case "TYPE_STORAGE_FORMAT_DATE":
      return TypeStorageFormat.DATE;
    case 8:
    case "TYPE_STORAGE_FORMAT_KEYWORD":
      return TypeStorageFormat.KEYWORD;
    case 9:
    case "TYPE_STORAGE_FORMAT_OBJECT":
      return TypeStorageFormat.OBJECT;
    case 10:
    case "TYPE_STORAGE_FORMAT_RELATION":
      return TypeStorageFormat.RELATION;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeStorageFormat");
  }
}

export function typeStorageFormatToJSON(object: TypeStorageFormat): string {
  switch (object) {
    case TypeStorageFormat.UNSPECIFIED:
      return "TYPE_STORAGE_FORMAT_UNSPECIFIED";
    case TypeStorageFormat.STRING:
      return "TYPE_STORAGE_FORMAT_STRING";
    case TypeStorageFormat.DOUBLE:
      return "TYPE_STORAGE_FORMAT_DOUBLE";
    case TypeStorageFormat.LONG:
      return "TYPE_STORAGE_FORMAT_LONG";
    case TypeStorageFormat.VECTOR:
      return "TYPE_STORAGE_FORMAT_VECTOR";
    case TypeStorageFormat.BINARY:
      return "TYPE_STORAGE_FORMAT_BINARY";
    case TypeStorageFormat.BOOLEAN:
      return "TYPE_STORAGE_FORMAT_BOOLEAN";
    case TypeStorageFormat.DATE:
      return "TYPE_STORAGE_FORMAT_DATE";
    case TypeStorageFormat.KEYWORD:
      return "TYPE_STORAGE_FORMAT_KEYWORD";
    case TypeStorageFormat.OBJECT:
      return "TYPE_STORAGE_FORMAT_OBJECT";
    case TypeStorageFormat.RELATION:
      return "TYPE_STORAGE_FORMAT_RELATION";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeStorageFormat");
  }
}

export enum TypeTag {
  UNSPECIFIED = 0,
  STRING = 1,
  NUMBER = 2,
  BOOLEAN = 3,
  VECTOR = 4,
  BLOB = 5,
  STRUCT = 6,
  JSON = 7,
  FUNCTION = 8,
  ENUM = 9,
  LITERAL = 10,
  TYPE_REFERENCE = 11,
  NODE = 12,
  ANY = 13,
}

export function typeTagFromJSON(object: any): TypeTag {
  switch (object) {
    case 0:
    case "TYPE_TAG_UNSPECIFIED":
      return TypeTag.UNSPECIFIED;
    case 1:
    case "TYPE_TAG_STRING":
      return TypeTag.STRING;
    case 2:
    case "TYPE_TAG_NUMBER":
      return TypeTag.NUMBER;
    case 3:
    case "TYPE_TAG_BOOLEAN":
      return TypeTag.BOOLEAN;
    case 4:
    case "TYPE_TAG_VECTOR":
      return TypeTag.VECTOR;
    case 5:
    case "TYPE_TAG_BLOB":
      return TypeTag.BLOB;
    case 6:
    case "TYPE_TAG_STRUCT":
      return TypeTag.STRUCT;
    case 7:
    case "TYPE_TAG_JSON":
      return TypeTag.JSON;
    case 8:
    case "TYPE_TAG_FUNCTION":
      return TypeTag.FUNCTION;
    case 9:
    case "TYPE_TAG_ENUM":
      return TypeTag.ENUM;
    case 10:
    case "TYPE_TAG_LITERAL":
      return TypeTag.LITERAL;
    case 11:
    case "TYPE_TAG_TYPE_REFERENCE":
      return TypeTag.TYPE_REFERENCE;
    case 12:
    case "TYPE_TAG_NODE":
      return TypeTag.NODE;
    case 13:
    case "TYPE_TAG_ANY":
      return TypeTag.ANY;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeTag");
  }
}

export function typeTagToJSON(object: TypeTag): string {
  switch (object) {
    case TypeTag.UNSPECIFIED:
      return "TYPE_TAG_UNSPECIFIED";
    case TypeTag.STRING:
      return "TYPE_TAG_STRING";
    case TypeTag.NUMBER:
      return "TYPE_TAG_NUMBER";
    case TypeTag.BOOLEAN:
      return "TYPE_TAG_BOOLEAN";
    case TypeTag.VECTOR:
      return "TYPE_TAG_VECTOR";
    case TypeTag.BLOB:
      return "TYPE_TAG_BLOB";
    case TypeTag.STRUCT:
      return "TYPE_TAG_STRUCT";
    case TypeTag.JSON:
      return "TYPE_TAG_JSON";
    case TypeTag.FUNCTION:
      return "TYPE_TAG_FUNCTION";
    case TypeTag.ENUM:
      return "TYPE_TAG_ENUM";
    case TypeTag.LITERAL:
      return "TYPE_TAG_LITERAL";
    case TypeTag.TYPE_REFERENCE:
      return "TYPE_TAG_TYPE_REFERENCE";
    case TypeTag.NODE:
      return "TYPE_TAG_NODE";
    case TypeTag.ANY:
      return "TYPE_TAG_ANY";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum TypeTag");
  }
}

export enum ViewLayout {
  UNSPECIFIED = 0,
  TABLE = 1,
}

export function viewLayoutFromJSON(object: any): ViewLayout {
  switch (object) {
    case 0:
    case "VIEW_LAYOUT_UNSPECIFIED":
      return ViewLayout.UNSPECIFIED;
    case 1:
    case "VIEW_LAYOUT_TABLE":
      return ViewLayout.TABLE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ViewLayout");
  }
}

export function viewLayoutToJSON(object: ViewLayout): string {
  switch (object) {
    case ViewLayout.UNSPECIFIED:
      return "VIEW_LAYOUT_UNSPECIFIED";
    case ViewLayout.TABLE:
      return "VIEW_LAYOUT_TABLE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ViewLayout");
  }
}

export enum WorkerProfile {
  UNSPECIFIED = 0,
  TINY = 1,
  SMALL = 2,
  MEDIUM = 3,
  LARGE = 4,
  XLARGE_CPU = 5,
  XLARGE_MEM = 6,
}

export function workerProfileFromJSON(object: any): WorkerProfile {
  switch (object) {
    case 0:
    case "WORKER_PROFILE_UNSPECIFIED":
      return WorkerProfile.UNSPECIFIED;
    case 1:
    case "WORKER_PROFILE_TINY":
      return WorkerProfile.TINY;
    case 2:
    case "WORKER_PROFILE_SMALL":
      return WorkerProfile.SMALL;
    case 3:
    case "WORKER_PROFILE_MEDIUM":
      return WorkerProfile.MEDIUM;
    case 4:
    case "WORKER_PROFILE_LARGE":
      return WorkerProfile.LARGE;
    case 5:
    case "WORKER_PROFILE_XLARGE_CPU":
      return WorkerProfile.XLARGE_CPU;
    case 6:
    case "WORKER_PROFILE_XLARGE_MEM":
      return WorkerProfile.XLARGE_MEM;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum WorkerProfile");
  }
}

export function workerProfileToJSON(object: WorkerProfile): string {
  switch (object) {
    case WorkerProfile.UNSPECIFIED:
      return "WORKER_PROFILE_UNSPECIFIED";
    case WorkerProfile.TINY:
      return "WORKER_PROFILE_TINY";
    case WorkerProfile.SMALL:
      return "WORKER_PROFILE_SMALL";
    case WorkerProfile.MEDIUM:
      return "WORKER_PROFILE_MEDIUM";
    case WorkerProfile.LARGE:
      return "WORKER_PROFILE_LARGE";
    case WorkerProfile.XLARGE_CPU:
      return "WORKER_PROFILE_XLARGE_CPU";
    case WorkerProfile.XLARGE_MEM:
      return "WORKER_PROFILE_XLARGE_MEM";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum WorkerProfile");
  }
}

export enum WorkerSetStatus {
  UNSPECIFIED = 0,
  SLEEPING = 1,
  PENDING = 2,
  UPDATING = 3,
  HEALTHY = 4,
  UNHEALTHY = 5,
  UNAVAILABLE = 6,
  UNKNOWN = 7,
}

export function workerSetStatusFromJSON(object: any): WorkerSetStatus {
  switch (object) {
    case 0:
    case "WORKER_SET_STATUS_UNSPECIFIED":
      return WorkerSetStatus.UNSPECIFIED;
    case 1:
    case "WORKER_SET_STATUS_SLEEPING":
      return WorkerSetStatus.SLEEPING;
    case 2:
    case "WORKER_SET_STATUS_PENDING":
      return WorkerSetStatus.PENDING;
    case 3:
    case "WORKER_SET_STATUS_UPDATING":
      return WorkerSetStatus.UPDATING;
    case 4:
    case "WORKER_SET_STATUS_HEALTHY":
      return WorkerSetStatus.HEALTHY;
    case 5:
    case "WORKER_SET_STATUS_UNHEALTHY":
      return WorkerSetStatus.UNHEALTHY;
    case 6:
    case "WORKER_SET_STATUS_UNAVAILABLE":
      return WorkerSetStatus.UNAVAILABLE;
    case 7:
    case "WORKER_SET_STATUS_UNKNOWN":
      return WorkerSetStatus.UNKNOWN;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum WorkerSetStatus");
  }
}

export function workerSetStatusToJSON(object: WorkerSetStatus): string {
  switch (object) {
    case WorkerSetStatus.UNSPECIFIED:
      return "WORKER_SET_STATUS_UNSPECIFIED";
    case WorkerSetStatus.SLEEPING:
      return "WORKER_SET_STATUS_SLEEPING";
    case WorkerSetStatus.PENDING:
      return "WORKER_SET_STATUS_PENDING";
    case WorkerSetStatus.UPDATING:
      return "WORKER_SET_STATUS_UPDATING";
    case WorkerSetStatus.HEALTHY:
      return "WORKER_SET_STATUS_HEALTHY";
    case WorkerSetStatus.UNHEALTHY:
      return "WORKER_SET_STATUS_UNHEALTHY";
    case WorkerSetStatus.UNAVAILABLE:
      return "WORKER_SET_STATUS_UNAVAILABLE";
    case WorkerSetStatus.UNKNOWN:
      return "WORKER_SET_STATUS_UNKNOWN";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum WorkerSetStatus");
  }
}

export interface EnvironmentData {
  metatype: BenchType;
  language: string;
  version: string;
  platform: string;
  packages: { [key: string]: any } | undefined;
}

export interface ExpressionData {
  metatype: BenchType;
  op: ExpressionOp;
  fieldCk: string;
  fieldKey: string;
  clauses: ExpressionData[];
  value: { [key: string]: any } | undefined;
  mode: string;
}

export interface LogEntryData {
  metatype: BenchType;
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
  metatype: BenchType;
  filename: string;
  lineno: number;
  name: string;
  locals: { [key: string]: any } | undefined;
  line: string;
}

export interface RunErrorData {
  metatype: BenchType;
  kind: RunErrorKind;
  type: string;
  message: string;
  statementCk: string;
  traceback: RunCodeFrameData[];
}

export interface WorkerSetData {
  metatype: BenchType;
  id: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  projectId: string;
  region: ProjectRegion;
  profile: WorkerProfile;
  sleeping: boolean;
  status: WorkerSetStatus;
  desiredReplicas: number;
  targetReplicas: number;
  availableReplicas: number;
  readyReplicas: number;
  lastActiveAt: Date | undefined;
}

export interface BlobData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
}

export interface IssueData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
  committed: boolean;
}

export interface BaseNodeData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
}

export interface RecordData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  value: { [key: string]: any } | undefined;
}

export interface ResolvedFieldData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  sessionId: string;
  rootId: string;
  projectId: string;
  workerNodeId: string;
  workerProcessId: string;
  statementCk: string;
  statementPath: string;
  scheduledAt: Date | undefined;
  startedAt: Date | undefined;
  terminatedAt: Date | undefined;
  triggerType: string;
  triggerId: string;
  accessLevel: number;
  status: RunStatus;
  inputs: { [key: string]: any } | undefined;
  outputs: { [key: string]: any } | undefined;
  error: { [key: string]: any } | undefined;
  value: { [key: string]: any } | undefined;
}

export interface SecretData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  sha512: string;
}

export interface SessionData {
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
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
  metatype: BenchType;
  id: string;
  ck: string;
  parentId: string;
  createdAt: Date | undefined;
  updatedAt: Date | undefined;
  deletedAt: Date | undefined;
  lastEditedAt: Date | undefined;
  lastChangedAt: Date | undefined;
  revision: number;
  name: string;
  layout: ViewLayout;
  query: ExpressionData | undefined;
  sort: ExpressionData[];
}

export interface SomeNodeData {
  node?:
    | { $case: "module"; module: ModuleData }
    | { $case: "file"; file: FileData }
    | { $case: "statement"; statement: StatementData }
    | { $case: "trigger"; trigger: TriggerData }
    | { $case: "tagging"; tagging: TaggingData }
    | { $case: "field"; field: FieldData }
    | { $case: "record"; record: RecordData }
    | { $case: "view"; view: ViewData }
    | { $case: "issue"; issue: IssueData }
    | { $case: "resolvedField"; resolvedField: ResolvedFieldData }
    | { $case: "blob"; blob: BlobData }
    | { $case: "secret"; secret: SecretData }
    | { $case: "session"; session: SessionData }
    | { $case: "run"; run: RunData }
    | undefined;
}

export interface SomeStructData {
  struct?:
    | { $case: "expression"; expression: ExpressionData }
    | { $case: "runCodeFrame"; runCodeFrame: RunCodeFrameData }
    | { $case: "runError"; runError: RunErrorData }
    | { $case: "logEntry"; logEntry: LogEntryData }
    | { $case: "workerSet"; workerSet: WorkerSetData }
    | { $case: "environment"; environment: EnvironmentData }
    | undefined;
}

export interface ModuleTreeData {
  module: ModuleData | undefined;
  nodes: SomeNodeData[];
}

function createBaseEnvironmentData(): EnvironmentData {
  return { metatype: 0, language: "", version: "", platform: "", packages: undefined };
}

export const EnvironmentData = {
  encode(message: EnvironmentData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.language !== "") {
      writer.uint32(162).string(message.language);
    }
    if (message.version !== "") {
      writer.uint32(170).string(message.version);
    }
    if (message.platform !== "") {
      writer.uint32(178).string(message.platform);
    }
    if (message.packages !== undefined) {
      Struct.encode(Struct.wrap(message.packages), writer.uint32(186).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EnvironmentData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEnvironmentData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 20:
          if (tag !== 162) {
            break;
          }

          message.language = reader.string();
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.version = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.platform = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.packages = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): EnvironmentData {
    return {
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      language: isSet(object.language) ? globalThis.String(object.language) : "",
      version: isSet(object.version) ? globalThis.String(object.version) : "",
      platform: isSet(object.platform) ? globalThis.String(object.platform) : "",
      packages: isObject(object.packages) ? object.packages : undefined,
    };
  },

  toJSON(message: EnvironmentData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.language !== "") {
      obj.language = message.language;
    }
    if (message.version !== "") {
      obj.version = message.version;
    }
    if (message.platform !== "") {
      obj.platform = message.platform;
    }
    if (message.packages !== undefined) {
      obj.packages = message.packages;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<EnvironmentData>, I>>(base?: I): EnvironmentData {
    return EnvironmentData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<EnvironmentData>, I>>(object: I): EnvironmentData {
    const message = createBaseEnvironmentData();
    message.metatype = object.metatype ?? 0;
    message.language = object.language ?? "";
    message.version = object.version ?? "";
    message.platform = object.platform ?? "";
    message.packages = object.packages ?? undefined;
    return message;
  },
};

function createBaseExpressionData(): ExpressionData {
  return { metatype: 0, op: 0, fieldCk: "", fieldKey: "", clauses: [], value: undefined, mode: "" };
}

export const ExpressionData = {
  encode(message: ExpressionData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
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
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
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
    message.metatype = object.metatype ?? 0;
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
    metatype: 0,
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
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
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
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
    message.metatype = object.metatype ?? 0;
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
  return { metatype: 0, filename: "", lineno: 0, name: "", locals: undefined, line: "" };
}

export const RunCodeFrameData = {
  encode(message: RunCodeFrameData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
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
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      filename: isSet(object.filename) ? globalThis.String(object.filename) : "",
      lineno: isSet(object.lineno) ? globalThis.Number(object.lineno) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      locals: isObject(object.locals) ? object.locals : undefined,
      line: isSet(object.line) ? globalThis.String(object.line) : "",
    };
  },

  toJSON(message: RunCodeFrameData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
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
    message.metatype = object.metatype ?? 0;
    message.filename = object.filename ?? "";
    message.lineno = object.lineno ?? 0;
    message.name = object.name ?? "";
    message.locals = object.locals ?? undefined;
    message.line = object.line ?? "";
    return message;
  },
};

function createBaseRunErrorData(): RunErrorData {
  return { metatype: 0, kind: 0, type: "", message: "", statementCk: "", traceback: [] };
}

export const RunErrorData = {
  encode(message: RunErrorData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
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
      RunCodeFrameData.encode(v!, writer.uint32(194).fork()).ldelim();
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
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
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

          message.traceback.push(RunCodeFrameData.decode(reader, reader.uint32()));
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      kind: isSet(object.kind) ? runErrorKindFromJSON(object.kind) : 0,
      type: isSet(object.type) ? globalThis.String(object.type) : "",
      message: isSet(object.message) ? globalThis.String(object.message) : "",
      statementCk: isSet(object.statementCk) ? globalThis.String(object.statementCk) : "",
      traceback: globalThis.Array.isArray(object?.traceback)
        ? object.traceback.map((e: any) => RunCodeFrameData.fromJSON(e))
        : [],
    };
  },

  toJSON(message: RunErrorData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
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
      obj.traceback = message.traceback.map((e) => RunCodeFrameData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunErrorData>, I>>(base?: I): RunErrorData {
    return RunErrorData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunErrorData>, I>>(object: I): RunErrorData {
    const message = createBaseRunErrorData();
    message.metatype = object.metatype ?? 0;
    message.kind = object.kind ?? 0;
    message.type = object.type ?? "";
    message.message = object.message ?? "";
    message.statementCk = object.statementCk ?? "";
    message.traceback = object.traceback?.map((e) => RunCodeFrameData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseWorkerSetData(): WorkerSetData {
  return {
    metatype: 0,
    id: "",
    createdAt: undefined,
    updatedAt: undefined,
    projectId: "",
    region: 0,
    profile: 0,
    sleeping: false,
    status: 0,
    desiredReplicas: 0,
    targetReplicas: 0,
    availableReplicas: 0,
    readyReplicas: 0,
    lastActiveAt: undefined,
  };
}

export const WorkerSetData = {
  encode(message: WorkerSetData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.createdAt !== undefined) {
      Timestamp.encode(toTimestamp(message.createdAt), writer.uint32(82).fork()).ldelim();
    }
    if (message.updatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.updatedAt), writer.uint32(90).fork()).ldelim();
    }
    if (message.projectId !== "") {
      writer.uint32(162).string(message.projectId);
    }
    if (message.region !== 0) {
      writer.uint32(168).int32(message.region);
    }
    if (message.profile !== 0) {
      writer.uint32(176).int32(message.profile);
    }
    if (message.sleeping === true) {
      writer.uint32(184).bool(message.sleeping);
    }
    if (message.status !== 0) {
      writer.uint32(192).int32(message.status);
    }
    if (message.desiredReplicas !== 0) {
      writer.uint32(200).int64(message.desiredReplicas);
    }
    if (message.targetReplicas !== 0) {
      writer.uint32(208).int64(message.targetReplicas);
    }
    if (message.availableReplicas !== 0) {
      writer.uint32(216).int64(message.availableReplicas);
    }
    if (message.readyReplicas !== 0) {
      writer.uint32(224).int64(message.readyReplicas);
    }
    if (message.lastActiveAt !== undefined) {
      Timestamp.encode(toTimestamp(message.lastActiveAt), writer.uint32(234).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): WorkerSetData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseWorkerSetData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
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
        case 20:
          if (tag !== 162) {
            break;
          }

          message.projectId = reader.string();
          continue;
        case 21:
          if (tag !== 168) {
            break;
          }

          message.region = reader.int32() as any;
          continue;
        case 22:
          if (tag !== 176) {
            break;
          }

          message.profile = reader.int32() as any;
          continue;
        case 23:
          if (tag !== 184) {
            break;
          }

          message.sleeping = reader.bool();
          continue;
        case 24:
          if (tag !== 192) {
            break;
          }

          message.status = reader.int32() as any;
          continue;
        case 25:
          if (tag !== 200) {
            break;
          }

          message.desiredReplicas = longToNumber(reader.int64() as Long);
          continue;
        case 26:
          if (tag !== 208) {
            break;
          }

          message.targetReplicas = longToNumber(reader.int64() as Long);
          continue;
        case 27:
          if (tag !== 216) {
            break;
          }

          message.availableReplicas = longToNumber(reader.int64() as Long);
          continue;
        case 28:
          if (tag !== 224) {
            break;
          }

          message.readyReplicas = longToNumber(reader.int64() as Long);
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.lastActiveAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): WorkerSetData {
    return {
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "",
      region: isSet(object.region) ? projectRegionFromJSON(object.region) : 0,
      profile: isSet(object.profile) ? workerProfileFromJSON(object.profile) : 0,
      sleeping: isSet(object.sleeping) ? globalThis.Boolean(object.sleeping) : false,
      status: isSet(object.status) ? workerSetStatusFromJSON(object.status) : 0,
      desiredReplicas: isSet(object.desiredReplicas) ? globalThis.Number(object.desiredReplicas) : 0,
      targetReplicas: isSet(object.targetReplicas) ? globalThis.Number(object.targetReplicas) : 0,
      availableReplicas: isSet(object.availableReplicas) ? globalThis.Number(object.availableReplicas) : 0,
      readyReplicas: isSet(object.readyReplicas) ? globalThis.Number(object.readyReplicas) : 0,
      lastActiveAt: isSet(object.lastActiveAt) ? fromJsonTimestamp(object.lastActiveAt) : undefined,
    };
  },

  toJSON(message: WorkerSetData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.createdAt !== undefined) {
      obj.createdAt = message.createdAt.toISOString();
    }
    if (message.updatedAt !== undefined) {
      obj.updatedAt = message.updatedAt.toISOString();
    }
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    if (message.region !== 0) {
      obj.region = projectRegionToJSON(message.region);
    }
    if (message.profile !== 0) {
      obj.profile = workerProfileToJSON(message.profile);
    }
    if (message.sleeping === true) {
      obj.sleeping = message.sleeping;
    }
    if (message.status !== 0) {
      obj.status = workerSetStatusToJSON(message.status);
    }
    if (message.desiredReplicas !== 0) {
      obj.desiredReplicas = Math.round(message.desiredReplicas);
    }
    if (message.targetReplicas !== 0) {
      obj.targetReplicas = Math.round(message.targetReplicas);
    }
    if (message.availableReplicas !== 0) {
      obj.availableReplicas = Math.round(message.availableReplicas);
    }
    if (message.readyReplicas !== 0) {
      obj.readyReplicas = Math.round(message.readyReplicas);
    }
    if (message.lastActiveAt !== undefined) {
      obj.lastActiveAt = message.lastActiveAt.toISOString();
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<WorkerSetData>, I>>(base?: I): WorkerSetData {
    return WorkerSetData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<WorkerSetData>, I>>(object: I): WorkerSetData {
    const message = createBaseWorkerSetData();
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.projectId = object.projectId ?? "";
    message.region = object.region ?? 0;
    message.profile = object.profile ?? 0;
    message.sleeping = object.sleeping ?? false;
    message.status = object.status ?? 0;
    message.desiredReplicas = object.desiredReplicas ?? 0;
    message.targetReplicas = object.targetReplicas ?? 0;
    message.availableReplicas = object.availableReplicas ?? 0;
    message.readyReplicas = object.readyReplicas ?? 0;
    message.lastActiveAt = object.lastActiveAt ?? undefined;
    return message;
  },
};

function createBaseBlobData(): BlobData {
  return {
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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

function createBaseBaseNodeData(): BaseNodeData {
  return {
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
  };
}

export const BaseNodeData = {
  encode(message: BaseNodeData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): BaseNodeData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseBaseNodeData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): BaseNodeData {
    return {
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
    };
  },

  toJSON(message: BaseNodeData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    return obj;
  },

  create<I extends Exact<DeepPartial<BaseNodeData>, I>>(base?: I): BaseNodeData {
    return BaseNodeData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<BaseNodeData>, I>>(object: I): BaseNodeData {
    const message = createBaseBaseNodeData();
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    return message;
  },
};

function createBaseRecordData(): RecordData {
  return {
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
    createdAt: undefined,
    updatedAt: undefined,
    deletedAt: undefined,
    lastEditedAt: undefined,
    lastChangedAt: undefined,
    revision: 0,
    sessionId: "",
    rootId: "",
    projectId: "",
    workerNodeId: "",
    workerProcessId: "",
    statementCk: "",
    statementPath: "",
    scheduledAt: undefined,
    startedAt: undefined,
    terminatedAt: undefined,
    triggerType: "",
    triggerId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
    if (message.sessionId !== "") {
      writer.uint32(162).string(message.sessionId);
    }
    if (message.rootId !== "") {
      writer.uint32(170).string(message.rootId);
    }
    if (message.projectId !== "") {
      writer.uint32(178).string(message.projectId);
    }
    if (message.workerNodeId !== "") {
      writer.uint32(186).string(message.workerNodeId);
    }
    if (message.workerProcessId !== "") {
      writer.uint32(194).string(message.workerProcessId);
    }
    if (message.statementCk !== "") {
      writer.uint32(202).string(message.statementCk);
    }
    if (message.statementPath !== "") {
      writer.uint32(210).string(message.statementPath);
    }
    if (message.scheduledAt !== undefined) {
      Timestamp.encode(toTimestamp(message.scheduledAt), writer.uint32(218).fork()).ldelim();
    }
    if (message.startedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.startedAt), writer.uint32(226).fork()).ldelim();
    }
    if (message.terminatedAt !== undefined) {
      Timestamp.encode(toTimestamp(message.terminatedAt), writer.uint32(234).fork()).ldelim();
    }
    if (message.triggerType !== "") {
      writer.uint32(242).string(message.triggerType);
    }
    if (message.triggerId !== "") {
      writer.uint32(250).string(message.triggerId);
    }
    if (message.accessLevel !== 0) {
      writer.uint32(256).int64(message.accessLevel);
    }
    if (message.status !== 0) {
      writer.uint32(264).int32(message.status);
    }
    if (message.inputs !== undefined) {
      Struct.encode(Struct.wrap(message.inputs), writer.uint32(274).fork()).ldelim();
    }
    if (message.outputs !== undefined) {
      Struct.encode(Struct.wrap(message.outputs), writer.uint32(282).fork()).ldelim();
    }
    if (message.error !== undefined) {
      Struct.encode(Struct.wrap(message.error), writer.uint32(290).fork()).ldelim();
    }
    if (message.value !== undefined) {
      Struct.encode(Struct.wrap(message.value), writer.uint32(298).fork()).ldelim();
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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

          message.sessionId = reader.string();
          continue;
        case 21:
          if (tag !== 170) {
            break;
          }

          message.rootId = reader.string();
          continue;
        case 22:
          if (tag !== 178) {
            break;
          }

          message.projectId = reader.string();
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.workerNodeId = reader.string();
          continue;
        case 24:
          if (tag !== 194) {
            break;
          }

          message.workerProcessId = reader.string();
          continue;
        case 25:
          if (tag !== 202) {
            break;
          }

          message.statementCk = reader.string();
          continue;
        case 26:
          if (tag !== 210) {
            break;
          }

          message.statementPath = reader.string();
          continue;
        case 27:
          if (tag !== 218) {
            break;
          }

          message.scheduledAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 28:
          if (tag !== 226) {
            break;
          }

          message.startedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 29:
          if (tag !== 234) {
            break;
          }

          message.terminatedAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 30:
          if (tag !== 242) {
            break;
          }

          message.triggerType = reader.string();
          continue;
        case 31:
          if (tag !== 250) {
            break;
          }

          message.triggerId = reader.string();
          continue;
        case 32:
          if (tag !== 256) {
            break;
          }

          message.accessLevel = longToNumber(reader.int64() as Long);
          continue;
        case 33:
          if (tag !== 264) {
            break;
          }

          message.status = reader.int32() as any;
          continue;
        case 34:
          if (tag !== 274) {
            break;
          }

          message.inputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 35:
          if (tag !== 282) {
            break;
          }

          message.outputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 36:
          if (tag !== 290) {
            break;
          }

          message.error = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 37:
          if (tag !== 298) {
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      sessionId: isSet(object.sessionId) ? globalThis.String(object.sessionId) : "",
      rootId: isSet(object.rootId) ? globalThis.String(object.rootId) : "",
      projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "",
      workerNodeId: isSet(object.workerNodeId) ? globalThis.String(object.workerNodeId) : "",
      workerProcessId: isSet(object.workerProcessId) ? globalThis.String(object.workerProcessId) : "",
      statementCk: isSet(object.statementCk) ? globalThis.String(object.statementCk) : "",
      statementPath: isSet(object.statementPath) ? globalThis.String(object.statementPath) : "",
      scheduledAt: isSet(object.scheduledAt) ? fromJsonTimestamp(object.scheduledAt) : undefined,
      startedAt: isSet(object.startedAt) ? fromJsonTimestamp(object.startedAt) : undefined,
      terminatedAt: isSet(object.terminatedAt) ? fromJsonTimestamp(object.terminatedAt) : undefined,
      triggerType: isSet(object.triggerType) ? globalThis.String(object.triggerType) : "",
      triggerId: isSet(object.triggerId) ? globalThis.String(object.triggerId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    if (message.sessionId !== "") {
      obj.sessionId = message.sessionId;
    }
    if (message.rootId !== "") {
      obj.rootId = message.rootId;
    }
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    if (message.workerNodeId !== "") {
      obj.workerNodeId = message.workerNodeId;
    }
    if (message.workerProcessId !== "") {
      obj.workerProcessId = message.workerProcessId;
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
    if (message.triggerId !== "") {
      obj.triggerId = message.triggerId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.sessionId = object.sessionId ?? "";
    message.rootId = object.rootId ?? "";
    message.projectId = object.projectId ?? "";
    message.workerNodeId = object.workerNodeId ?? "";
    message.workerProcessId = object.workerProcessId ?? "";
    message.statementCk = object.statementCk ?? "";
    message.statementPath = object.statementPath ?? "";
    message.scheduledAt = object.scheduledAt ?? undefined;
    message.startedAt = object.startedAt ?? undefined;
    message.terminatedAt = object.terminatedAt ?? undefined;
    message.triggerType = object.triggerType ?? "";
    message.triggerId = object.triggerId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
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
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
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
    metatype: 0,
    id: "",
    ck: "",
    parentId: "",
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
    if (message.metatype !== 0) {
      writer.uint32(8).int32(message.metatype);
    }
    if (message.id !== "") {
      writer.uint32(18).string(message.id);
    }
    if (message.ck !== "") {
      writer.uint32(26).string(message.ck);
    }
    if (message.parentId !== "") {
      writer.uint32(34).string(message.parentId);
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
      ExpressionData.encode(message.query, writer.uint32(178).fork()).ldelim();
    }
    for (const v of message.sort) {
      ExpressionData.encode(v!, writer.uint32(186).fork()).ldelim();
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
          if (tag !== 8) {
            break;
          }

          message.metatype = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.id = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.ck = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.parentId = reader.string();
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

          message.query = ExpressionData.decode(reader, reader.uint32());
          continue;
        case 23:
          if (tag !== 186) {
            break;
          }

          message.sort.push(ExpressionData.decode(reader, reader.uint32()));
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
      metatype: isSet(object.metatype) ? benchTypeFromJSON(object.metatype) : 0,
      id: isSet(object.id) ? globalThis.String(object.id) : "",
      ck: isSet(object.ck) ? globalThis.String(object.ck) : "",
      parentId: isSet(object.parentId) ? globalThis.String(object.parentId) : "",
      createdAt: isSet(object.createdAt) ? fromJsonTimestamp(object.createdAt) : undefined,
      updatedAt: isSet(object.updatedAt) ? fromJsonTimestamp(object.updatedAt) : undefined,
      deletedAt: isSet(object.deletedAt) ? fromJsonTimestamp(object.deletedAt) : undefined,
      lastEditedAt: isSet(object.lastEditedAt) ? fromJsonTimestamp(object.lastEditedAt) : undefined,
      lastChangedAt: isSet(object.lastChangedAt) ? fromJsonTimestamp(object.lastChangedAt) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      layout: isSet(object.layout) ? viewLayoutFromJSON(object.layout) : 0,
      query: isSet(object.query) ? ExpressionData.fromJSON(object.query) : undefined,
      sort: globalThis.Array.isArray(object?.sort) ? object.sort.map((e: any) => ExpressionData.fromJSON(e)) : [],
    };
  },

  toJSON(message: ViewData): unknown {
    const obj: any = {};
    if (message.metatype !== 0) {
      obj.metatype = benchTypeToJSON(message.metatype);
    }
    if (message.id !== "") {
      obj.id = message.id;
    }
    if (message.ck !== "") {
      obj.ck = message.ck;
    }
    if (message.parentId !== "") {
      obj.parentId = message.parentId;
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
      obj.query = ExpressionData.toJSON(message.query);
    }
    if (message.sort?.length) {
      obj.sort = message.sort.map((e) => ExpressionData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ViewData>, I>>(base?: I): ViewData {
    return ViewData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ViewData>, I>>(object: I): ViewData {
    const message = createBaseViewData();
    message.metatype = object.metatype ?? 0;
    message.id = object.id ?? "";
    message.ck = object.ck ?? "";
    message.parentId = object.parentId ?? "";
    message.createdAt = object.createdAt ?? undefined;
    message.updatedAt = object.updatedAt ?? undefined;
    message.deletedAt = object.deletedAt ?? undefined;
    message.lastEditedAt = object.lastEditedAt ?? undefined;
    message.lastChangedAt = object.lastChangedAt ?? undefined;
    message.revision = object.revision ?? 0;
    message.name = object.name ?? "";
    message.layout = object.layout ?? 0;
    message.query =
      object.query !== undefined && object.query !== null ? ExpressionData.fromPartial(object.query) : undefined;
    message.sort = object.sort?.map((e) => ExpressionData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseSomeNodeData(): SomeNodeData {
  return { node: undefined };
}

export const SomeNodeData = {
  encode(message: SomeNodeData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    switch (message.node?.$case) {
      case "module":
        ModuleData.encode(message.node.module, writer.uint32(10).fork()).ldelim();
        break;
      case "file":
        FileData.encode(message.node.file, writer.uint32(18).fork()).ldelim();
        break;
      case "statement":
        StatementData.encode(message.node.statement, writer.uint32(26).fork()).ldelim();
        break;
      case "trigger":
        TriggerData.encode(message.node.trigger, writer.uint32(34).fork()).ldelim();
        break;
      case "tagging":
        TaggingData.encode(message.node.tagging, writer.uint32(42).fork()).ldelim();
        break;
      case "field":
        FieldData.encode(message.node.field, writer.uint32(50).fork()).ldelim();
        break;
      case "record":
        RecordData.encode(message.node.record, writer.uint32(58).fork()).ldelim();
        break;
      case "view":
        ViewData.encode(message.node.view, writer.uint32(66).fork()).ldelim();
        break;
      case "issue":
        IssueData.encode(message.node.issue, writer.uint32(74).fork()).ldelim();
        break;
      case "resolvedField":
        ResolvedFieldData.encode(message.node.resolvedField, writer.uint32(82).fork()).ldelim();
        break;
      case "blob":
        BlobData.encode(message.node.blob, writer.uint32(90).fork()).ldelim();
        break;
      case "secret":
        SecretData.encode(message.node.secret, writer.uint32(98).fork()).ldelim();
        break;
      case "session":
        SessionData.encode(message.node.session, writer.uint32(106).fork()).ldelim();
        break;
      case "run":
        RunData.encode(message.node.run, writer.uint32(114).fork()).ldelim();
        break;
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SomeNodeData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSomeNodeData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.node = { $case: "module", module: ModuleData.decode(reader, reader.uint32()) };
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.node = { $case: "file", file: FileData.decode(reader, reader.uint32()) };
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.node = { $case: "statement", statement: StatementData.decode(reader, reader.uint32()) };
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.node = { $case: "trigger", trigger: TriggerData.decode(reader, reader.uint32()) };
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.node = { $case: "tagging", tagging: TaggingData.decode(reader, reader.uint32()) };
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.node = { $case: "field", field: FieldData.decode(reader, reader.uint32()) };
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.node = { $case: "record", record: RecordData.decode(reader, reader.uint32()) };
          continue;
        case 8:
          if (tag !== 66) {
            break;
          }

          message.node = { $case: "view", view: ViewData.decode(reader, reader.uint32()) };
          continue;
        case 9:
          if (tag !== 74) {
            break;
          }

          message.node = { $case: "issue", issue: IssueData.decode(reader, reader.uint32()) };
          continue;
        case 10:
          if (tag !== 82) {
            break;
          }

          message.node = { $case: "resolvedField", resolvedField: ResolvedFieldData.decode(reader, reader.uint32()) };
          continue;
        case 11:
          if (tag !== 90) {
            break;
          }

          message.node = { $case: "blob", blob: BlobData.decode(reader, reader.uint32()) };
          continue;
        case 12:
          if (tag !== 98) {
            break;
          }

          message.node = { $case: "secret", secret: SecretData.decode(reader, reader.uint32()) };
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.node = { $case: "session", session: SessionData.decode(reader, reader.uint32()) };
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.node = { $case: "run", run: RunData.decode(reader, reader.uint32()) };
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SomeNodeData {
    return {
      node: isSet(object.module)
        ? { $case: "module", module: ModuleData.fromJSON(object.module) }
        : isSet(object.file)
        ? { $case: "file", file: FileData.fromJSON(object.file) }
        : isSet(object.statement)
        ? { $case: "statement", statement: StatementData.fromJSON(object.statement) }
        : isSet(object.trigger)
        ? { $case: "trigger", trigger: TriggerData.fromJSON(object.trigger) }
        : isSet(object.tagging)
        ? { $case: "tagging", tagging: TaggingData.fromJSON(object.tagging) }
        : isSet(object.field)
        ? { $case: "field", field: FieldData.fromJSON(object.field) }
        : isSet(object.record)
        ? { $case: "record", record: RecordData.fromJSON(object.record) }
        : isSet(object.view)
        ? { $case: "view", view: ViewData.fromJSON(object.view) }
        : isSet(object.issue)
        ? { $case: "issue", issue: IssueData.fromJSON(object.issue) }
        : isSet(object.resolvedField)
        ? { $case: "resolvedField", resolvedField: ResolvedFieldData.fromJSON(object.resolvedField) }
        : isSet(object.blob)
        ? { $case: "blob", blob: BlobData.fromJSON(object.blob) }
        : isSet(object.secret)
        ? { $case: "secret", secret: SecretData.fromJSON(object.secret) }
        : isSet(object.session)
        ? { $case: "session", session: SessionData.fromJSON(object.session) }
        : isSet(object.run)
        ? { $case: "run", run: RunData.fromJSON(object.run) }
        : undefined,
    };
  },

  toJSON(message: SomeNodeData): unknown {
    const obj: any = {};
    if (message.node?.$case === "module") {
      obj.module = ModuleData.toJSON(message.node.module);
    }
    if (message.node?.$case === "file") {
      obj.file = FileData.toJSON(message.node.file);
    }
    if (message.node?.$case === "statement") {
      obj.statement = StatementData.toJSON(message.node.statement);
    }
    if (message.node?.$case === "trigger") {
      obj.trigger = TriggerData.toJSON(message.node.trigger);
    }
    if (message.node?.$case === "tagging") {
      obj.tagging = TaggingData.toJSON(message.node.tagging);
    }
    if (message.node?.$case === "field") {
      obj.field = FieldData.toJSON(message.node.field);
    }
    if (message.node?.$case === "record") {
      obj.record = RecordData.toJSON(message.node.record);
    }
    if (message.node?.$case === "view") {
      obj.view = ViewData.toJSON(message.node.view);
    }
    if (message.node?.$case === "issue") {
      obj.issue = IssueData.toJSON(message.node.issue);
    }
    if (message.node?.$case === "resolvedField") {
      obj.resolvedField = ResolvedFieldData.toJSON(message.node.resolvedField);
    }
    if (message.node?.$case === "blob") {
      obj.blob = BlobData.toJSON(message.node.blob);
    }
    if (message.node?.$case === "secret") {
      obj.secret = SecretData.toJSON(message.node.secret);
    }
    if (message.node?.$case === "session") {
      obj.session = SessionData.toJSON(message.node.session);
    }
    if (message.node?.$case === "run") {
      obj.run = RunData.toJSON(message.node.run);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SomeNodeData>, I>>(base?: I): SomeNodeData {
    return SomeNodeData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SomeNodeData>, I>>(object: I): SomeNodeData {
    const message = createBaseSomeNodeData();
    if (object.node?.$case === "module" && object.node?.module !== undefined && object.node?.module !== null) {
      message.node = { $case: "module", module: ModuleData.fromPartial(object.node.module) };
    }
    if (object.node?.$case === "file" && object.node?.file !== undefined && object.node?.file !== null) {
      message.node = { $case: "file", file: FileData.fromPartial(object.node.file) };
    }
    if (object.node?.$case === "statement" && object.node?.statement !== undefined && object.node?.statement !== null) {
      message.node = { $case: "statement", statement: StatementData.fromPartial(object.node.statement) };
    }
    if (object.node?.$case === "trigger" && object.node?.trigger !== undefined && object.node?.trigger !== null) {
      message.node = { $case: "trigger", trigger: TriggerData.fromPartial(object.node.trigger) };
    }
    if (object.node?.$case === "tagging" && object.node?.tagging !== undefined && object.node?.tagging !== null) {
      message.node = { $case: "tagging", tagging: TaggingData.fromPartial(object.node.tagging) };
    }
    if (object.node?.$case === "field" && object.node?.field !== undefined && object.node?.field !== null) {
      message.node = { $case: "field", field: FieldData.fromPartial(object.node.field) };
    }
    if (object.node?.$case === "record" && object.node?.record !== undefined && object.node?.record !== null) {
      message.node = { $case: "record", record: RecordData.fromPartial(object.node.record) };
    }
    if (object.node?.$case === "view" && object.node?.view !== undefined && object.node?.view !== null) {
      message.node = { $case: "view", view: ViewData.fromPartial(object.node.view) };
    }
    if (object.node?.$case === "issue" && object.node?.issue !== undefined && object.node?.issue !== null) {
      message.node = { $case: "issue", issue: IssueData.fromPartial(object.node.issue) };
    }
    if (
      object.node?.$case === "resolvedField" &&
      object.node?.resolvedField !== undefined &&
      object.node?.resolvedField !== null
    ) {
      message.node = {
        $case: "resolvedField",
        resolvedField: ResolvedFieldData.fromPartial(object.node.resolvedField),
      };
    }
    if (object.node?.$case === "blob" && object.node?.blob !== undefined && object.node?.blob !== null) {
      message.node = { $case: "blob", blob: BlobData.fromPartial(object.node.blob) };
    }
    if (object.node?.$case === "secret" && object.node?.secret !== undefined && object.node?.secret !== null) {
      message.node = { $case: "secret", secret: SecretData.fromPartial(object.node.secret) };
    }
    if (object.node?.$case === "session" && object.node?.session !== undefined && object.node?.session !== null) {
      message.node = { $case: "session", session: SessionData.fromPartial(object.node.session) };
    }
    if (object.node?.$case === "run" && object.node?.run !== undefined && object.node?.run !== null) {
      message.node = { $case: "run", run: RunData.fromPartial(object.node.run) };
    }
    return message;
  },
};

function createBaseSomeStructData(): SomeStructData {
  return { struct: undefined };
}

export const SomeStructData = {
  encode(message: SomeStructData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    switch (message.struct?.$case) {
      case "expression":
        ExpressionData.encode(message.struct.expression, writer.uint32(10).fork()).ldelim();
        break;
      case "runCodeFrame":
        RunCodeFrameData.encode(message.struct.runCodeFrame, writer.uint32(18).fork()).ldelim();
        break;
      case "runError":
        RunErrorData.encode(message.struct.runError, writer.uint32(26).fork()).ldelim();
        break;
      case "logEntry":
        LogEntryData.encode(message.struct.logEntry, writer.uint32(34).fork()).ldelim();
        break;
      case "workerSet":
        WorkerSetData.encode(message.struct.workerSet, writer.uint32(42).fork()).ldelim();
        break;
      case "environment":
        EnvironmentData.encode(message.struct.environment, writer.uint32(50).fork()).ldelim();
        break;
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SomeStructData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSomeStructData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.struct = { $case: "expression", expression: ExpressionData.decode(reader, reader.uint32()) };
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.struct = { $case: "runCodeFrame", runCodeFrame: RunCodeFrameData.decode(reader, reader.uint32()) };
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.struct = { $case: "runError", runError: RunErrorData.decode(reader, reader.uint32()) };
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.struct = { $case: "logEntry", logEntry: LogEntryData.decode(reader, reader.uint32()) };
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.struct = { $case: "workerSet", workerSet: WorkerSetData.decode(reader, reader.uint32()) };
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.struct = { $case: "environment", environment: EnvironmentData.decode(reader, reader.uint32()) };
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SomeStructData {
    return {
      struct: isSet(object.expression)
        ? { $case: "expression", expression: ExpressionData.fromJSON(object.expression) }
        : isSet(object.runCodeFrame)
        ? { $case: "runCodeFrame", runCodeFrame: RunCodeFrameData.fromJSON(object.runCodeFrame) }
        : isSet(object.runError)
        ? { $case: "runError", runError: RunErrorData.fromJSON(object.runError) }
        : isSet(object.logEntry)
        ? { $case: "logEntry", logEntry: LogEntryData.fromJSON(object.logEntry) }
        : isSet(object.workerSet)
        ? { $case: "workerSet", workerSet: WorkerSetData.fromJSON(object.workerSet) }
        : isSet(object.environment)
        ? { $case: "environment", environment: EnvironmentData.fromJSON(object.environment) }
        : undefined,
    };
  },

  toJSON(message: SomeStructData): unknown {
    const obj: any = {};
    if (message.struct?.$case === "expression") {
      obj.expression = ExpressionData.toJSON(message.struct.expression);
    }
    if (message.struct?.$case === "runCodeFrame") {
      obj.runCodeFrame = RunCodeFrameData.toJSON(message.struct.runCodeFrame);
    }
    if (message.struct?.$case === "runError") {
      obj.runError = RunErrorData.toJSON(message.struct.runError);
    }
    if (message.struct?.$case === "logEntry") {
      obj.logEntry = LogEntryData.toJSON(message.struct.logEntry);
    }
    if (message.struct?.$case === "workerSet") {
      obj.workerSet = WorkerSetData.toJSON(message.struct.workerSet);
    }
    if (message.struct?.$case === "environment") {
      obj.environment = EnvironmentData.toJSON(message.struct.environment);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SomeStructData>, I>>(base?: I): SomeStructData {
    return SomeStructData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SomeStructData>, I>>(object: I): SomeStructData {
    const message = createBaseSomeStructData();
    if (
      object.struct?.$case === "expression" &&
      object.struct?.expression !== undefined &&
      object.struct?.expression !== null
    ) {
      message.struct = { $case: "expression", expression: ExpressionData.fromPartial(object.struct.expression) };
    }
    if (
      object.struct?.$case === "runCodeFrame" &&
      object.struct?.runCodeFrame !== undefined &&
      object.struct?.runCodeFrame !== null
    ) {
      message.struct = {
        $case: "runCodeFrame",
        runCodeFrame: RunCodeFrameData.fromPartial(object.struct.runCodeFrame),
      };
    }
    if (
      object.struct?.$case === "runError" &&
      object.struct?.runError !== undefined &&
      object.struct?.runError !== null
    ) {
      message.struct = { $case: "runError", runError: RunErrorData.fromPartial(object.struct.runError) };
    }
    if (
      object.struct?.$case === "logEntry" &&
      object.struct?.logEntry !== undefined &&
      object.struct?.logEntry !== null
    ) {
      message.struct = { $case: "logEntry", logEntry: LogEntryData.fromPartial(object.struct.logEntry) };
    }
    if (
      object.struct?.$case === "workerSet" &&
      object.struct?.workerSet !== undefined &&
      object.struct?.workerSet !== null
    ) {
      message.struct = { $case: "workerSet", workerSet: WorkerSetData.fromPartial(object.struct.workerSet) };
    }
    if (
      object.struct?.$case === "environment" &&
      object.struct?.environment !== undefined &&
      object.struct?.environment !== null
    ) {
      message.struct = { $case: "environment", environment: EnvironmentData.fromPartial(object.struct.environment) };
    }
    return message;
  },
};

function createBaseModuleTreeData(): ModuleTreeData {
  return { module: undefined, nodes: [] };
}

export const ModuleTreeData = {
  encode(message: ModuleTreeData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.module !== undefined) {
      ModuleData.encode(message.module, writer.uint32(10).fork()).ldelim();
    }
    for (const v of message.nodes) {
      SomeNodeData.encode(v!, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ModuleTreeData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseModuleTreeData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.module = ModuleData.decode(reader, reader.uint32());
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.nodes.push(SomeNodeData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ModuleTreeData {
    return {
      module: isSet(object.module) ? ModuleData.fromJSON(object.module) : undefined,
      nodes: globalThis.Array.isArray(object?.nodes) ? object.nodes.map((e: any) => SomeNodeData.fromJSON(e)) : [],
    };
  },

  toJSON(message: ModuleTreeData): unknown {
    const obj: any = {};
    if (message.module !== undefined) {
      obj.module = ModuleData.toJSON(message.module);
    }
    if (message.nodes?.length) {
      obj.nodes = message.nodes.map((e) => SomeNodeData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ModuleTreeData>, I>>(base?: I): ModuleTreeData {
    return ModuleTreeData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ModuleTreeData>, I>>(object: I): ModuleTreeData {
    const message = createBaseModuleTreeData();
    message.module =
      object.module !== undefined && object.module !== null ? ModuleData.fromPartial(object.module) : undefined;
    message.nodes = object.nodes?.map((e) => SomeNodeData.fromPartial(e)) || [];
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
  : T extends { $case: string }
  ? { [K in keyof Omit<T, "$case">]?: DeepPartial<T[K]> } & { $case: T["$case"] }
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
