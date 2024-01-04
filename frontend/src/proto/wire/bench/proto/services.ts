/* eslint-disable */
import { grpc } from "@improbable-eng/grpc-web";
import { BrowserHeaders } from "browser-headers";
import * as _m0 from "protobufjs/minimal";
import { Observable } from "rxjs";
import { share } from "rxjs/operators";
import { Empty } from "../../google/protobuf/empty";
import { Struct } from "../../google/protobuf/struct";
import { Timestamp } from "../../google/protobuf/timestamp";
import {
  BlobData,
  ClientType,
  clientTypeFromJSON,
  clientTypeToJSON,
  EditKind,
  editKindFromJSON,
  editKindToJSON,
  ExpressionData,
  LogEntryData,
  ModuleData,
  NodeType,
  nodeTypeFromJSON,
  nodeTypeToJSON,
  ProjectRegion,
  projectRegionFromJSON,
  projectRegionToJSON,
  RunData,
  SecretData,
  SessionAccessLevel,
  sessionAccessLevelFromJSON,
  sessionAccessLevelToJSON,
  SomeNodeData,
  TriggerType,
  triggerTypeFromJSON,
  triggerTypeToJSON,
  WorkerImageData,
  WorkerProfile,
  workerProfileFromJSON,
  workerProfileToJSON,
  WorkerSetData,
} from "./bench";
import Long = require("long");

export const protobufPackage = "";

export enum ServiceType {
  UNSPECIFIED = 0,
  RUNTIME_SUPERVISOR = 1,
  RUNTIME_HOST = 2,
  WORKER_NODE = 3,
}

export function serviceTypeFromJSON(object: any): ServiceType {
  switch (object) {
    case 0:
    case "SERVICE_TYPE_UNSPECIFIED":
      return ServiceType.UNSPECIFIED;
    case 1:
    case "SERVICE_TYPE_RUNTIME_SUPERVISOR":
      return ServiceType.RUNTIME_SUPERVISOR;
    case 2:
    case "SERVICE_TYPE_RUNTIME_HOST":
      return ServiceType.RUNTIME_HOST;
    case 3:
    case "SERVICE_TYPE_WORKER_NODE":
      return ServiceType.WORKER_NODE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ServiceType");
  }
}

export function serviceTypeToJSON(object: ServiceType): string {
  switch (object) {
    case ServiceType.UNSPECIFIED:
      return "SERVICE_TYPE_UNSPECIFIED";
    case ServiceType.RUNTIME_SUPERVISOR:
      return "SERVICE_TYPE_RUNTIME_SUPERVISOR";
    case ServiceType.RUNTIME_HOST:
      return "SERVICE_TYPE_RUNTIME_HOST";
    case ServiceType.WORKER_NODE:
      return "SERVICE_TYPE_WORKER_NODE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ServiceType");
  }
}

/**
 * Edit describes an edit to a node in a Bench.
 * (Manually defined here since Node properties inside structs aren't supported.)
 */
export interface EditData {
  kind: EditKind;
  moduleId: string;
  node: SomeNodeData | undefined;
  revision: number;
  properties: string[];
}

export interface ClientOrigin {
  clientType: ClientType;
  clientId: string;
  nonce: string;
}

export interface NodePointer {
  nodeType: NodeType;
  node?: { $case: "nodeCk"; nodeCk: string } | { $case: "nodeId"; nodeId: string } | undefined;
}

export interface DidCreateProjectRequest {
  projectId: string;
}

export interface GetNotificationsRequest {}

/** repeated NotificationData notifications = 1; */
export interface GetNotificationsResponse {}

export interface GetWorkerChangesRequest {}

export interface GetWorkerChangesResponse {
  workerSets: WorkerSetData[];
}

export interface ConfigureWorkerSetRequest {
  projectId: string;
  profile: WorkerProfile;
  region: ProjectRegion;
  targetReplicas: number;
}

export interface ConfigureWorkerSetResponse {
  workerSet: WorkerSetData | undefined;
}

export interface RestartWorkerSetRequest {
  projectId: string;
}

export interface GetWorkerImageRequest {
  projectId: string;
}

export interface GetWorkerImageResponse {
  image: WorkerImageData | undefined;
}

export interface PingWorkerSetRequest {
  projectId: string;
}

export interface GetBenchChangesRequest {
  afterChangeMarker: number;
}

export interface GetBenchChangesResponse {
  edits: EditData[];
  snapshot: ModuleData | undefined;
}

export interface GetModuleEditsRequest {
  afterEditMarker: number;
}

export interface GetModuleEditsResponse {
  edits: EditData[];
}

export interface GetLogsRequest {}

export interface GetLogsResponse {
  logs: LogEntryData[];
}

export interface ReadNodesRequest {
  roots: NodePointer[];
  includeDeferredProperties: boolean;
}

export interface ReadNodesResponse {
  nodes: SomeNodeData[];
}

export interface SearchNodesRequest {
  nodeType: NodeType;
  nodeCk: string;
  filter: ExpressionData | undefined;
  sort: ExpressionData[];
  limit: number;
  after: string;
}

export interface SearchNodesResponse {
  nodes: SomeNodeData[];
  cursors: string[];
  startCursor: string;
}

export interface CommitEditsRequest {
  edits: EditData[];
}

export interface CommitEditsResponse {
  changedNodes: SomeNodeData[];
}

export interface UploadBlobRequest {
  blob: BlobData | undefined;
}

export interface UploadBlobResponse {
  postUrl: string;
  expiresAt: Date | undefined;
}

export interface DownloadBlobRequest {
  blob: BlobData | undefined;
}

export interface DownloadBlobResponse {
  getUrl: string;
  expiresAt: Date | undefined;
}

export interface RevealSecretRequest {
  secret: SecretData | undefined;
}

export interface RevealSecretResponse {
  secret: SecretData | undefined;
}

export interface PasteNodesRequest {
  sourceModuleId: string;
  sourceNodes: NodePointer[];
  targetIds: { [key: string]: string };
  targetCks: { [key: string]: string };
  targetParentIds: { [key: string]: string };
  targetOrderKeys: { [key: string]: string };
}

export interface PasteNodesRequest_TargetIdsEntry {
  key: string;
  value: string;
}

export interface PasteNodesRequest_TargetCksEntry {
  key: string;
  value: string;
}

export interface PasteNodesRequest_TargetParentIdsEntry {
  key: string;
  value: string;
}

export interface PasteNodesRequest_TargetOrderKeysEntry {
  key: string;
  value: string;
}

export interface PasteNodesResponse {
  pastedNodes: SomeNodeData[];
}

export interface SnapshotModuleRequest {
  name: string;
  tag: string;
  description: string;
}

export interface SnapshotModuleResponse {
  snapshotProjectVersionId: string;
}

export interface PushWorkerLogsRequest {
  logs: LogEntryData[];
}

export interface RunProxyStatementRequest {
  statement: string;
  inputs: { [key: string]: any } | undefined;
  timeoutMs: number;
  runId: string;
}

export interface RunProxyStatementResponse {
  outputs: { [key: string]: any } | undefined;
  error: { [key: string]: any } | undefined;
}

export interface PullWorkerRunsRequest {
  workerSetId: string;
  workerNodeId: string;
  workerProcessId: string;
}

export interface PullWorkerRunsResponse {
  runs: RunData[];
}

export interface RestartWorkerRequest {}

export interface StartRunRequest {
  triggerType: TriggerType;
  triggerId: string;
  runId: string;
  sessionId: string;
  statement: string;
  scope: string;
  code: string;
  scheduledAt: Date | undefined;
  inputs: { [key: string]: any } | undefined;
  block: boolean;
  keyed: boolean;
  keyedReturn: boolean;
  tags: string[];
  rootValue: { [key: string]: any } | undefined;
  globalValue: { [key: string]: any } | undefined;
  accessLevel: SessionAccessLevel;
}

export interface StartRunResponse {
  errorType: StartRunResponse_ErrorType;
  runId: string;
  run: RunData | undefined;
  logs: LogEntryData[];
}

export enum StartRunResponse_ErrorType {
  START_RUN_UNSPECIFIED = 0,
  START_RUN_UNAVAILABLE = 1,
  START_RUN_INVALID = 2,
  START_RUN_INTERNAL = 3,
  START_RUN_TIMEOUT = 4,
  START_RUN_RUNTIME = 5,
  START_RUN_DUPLICATE = 6,
}

export function startRunResponse_ErrorTypeFromJSON(object: any): StartRunResponse_ErrorType {
  switch (object) {
    case 0:
    case "START_RUN_ERROR_TYPE_UNSPECIFIED":
      return StartRunResponse_ErrorType.START_RUN_UNSPECIFIED;
    case 1:
    case "START_RUN_ERROR_TYPE_UNAVAILABLE":
      return StartRunResponse_ErrorType.START_RUN_UNAVAILABLE;
    case 2:
    case "START_RUN_ERROR_TYPE_INVALID":
      return StartRunResponse_ErrorType.START_RUN_INVALID;
    case 3:
    case "START_RUN_ERROR_TYPE_INTERNAL":
      return StartRunResponse_ErrorType.START_RUN_INTERNAL;
    case 4:
    case "START_RUN_ERROR_TYPE_TIMEOUT":
      return StartRunResponse_ErrorType.START_RUN_TIMEOUT;
    case 5:
    case "START_RUN_ERROR_TYPE_RUNTIME":
      return StartRunResponse_ErrorType.START_RUN_RUNTIME;
    case 6:
    case "START_RUN_ERROR_TYPE_DUPLICATE":
      return StartRunResponse_ErrorType.START_RUN_DUPLICATE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StartRunResponse_ErrorType");
  }
}

export function startRunResponse_ErrorTypeToJSON(object: StartRunResponse_ErrorType): string {
  switch (object) {
    case StartRunResponse_ErrorType.START_RUN_UNSPECIFIED:
      return "START_RUN_ERROR_TYPE_UNSPECIFIED";
    case StartRunResponse_ErrorType.START_RUN_UNAVAILABLE:
      return "START_RUN_ERROR_TYPE_UNAVAILABLE";
    case StartRunResponse_ErrorType.START_RUN_INVALID:
      return "START_RUN_ERROR_TYPE_INVALID";
    case StartRunResponse_ErrorType.START_RUN_INTERNAL:
      return "START_RUN_ERROR_TYPE_INTERNAL";
    case StartRunResponse_ErrorType.START_RUN_TIMEOUT:
      return "START_RUN_ERROR_TYPE_TIMEOUT";
    case StartRunResponse_ErrorType.START_RUN_RUNTIME:
      return "START_RUN_ERROR_TYPE_RUNTIME";
    case StartRunResponse_ErrorType.START_RUN_DUPLICATE:
      return "START_RUN_ERROR_TYPE_DUPLICATE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StartRunResponse_ErrorType");
  }
}

export interface KillRunRequest {
  runId: string;
}

export interface KillRunResponse {
  success: boolean;
}

function createBaseEditData(): EditData {
  return { kind: 0, moduleId: "", node: undefined, revision: 0, properties: [] };
}

export const EditData = {
  encode(message: EditData, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.kind !== 0) {
      writer.uint32(8).int32(message.kind);
    }
    if (message.moduleId !== "") {
      writer.uint32(18).string(message.moduleId);
    }
    if (message.node !== undefined) {
      SomeNodeData.encode(message.node, writer.uint32(26).fork()).ldelim();
    }
    if (message.revision !== 0) {
      writer.uint32(32).int64(message.revision);
    }
    for (const v of message.properties) {
      writer.uint32(42).string(v!);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): EditData {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseEditData();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.kind = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.moduleId = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.node = SomeNodeData.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.revision = longToNumber(reader.int64() as Long);
          continue;
        case 5:
          if (tag !== 42) {
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

  fromJSON(object: any): EditData {
    return {
      kind: isSet(object.kind) ? editKindFromJSON(object.kind) : 0,
      moduleId: isSet(object.moduleId) ? globalThis.String(object.moduleId) : "",
      node: isSet(object.node) ? SomeNodeData.fromJSON(object.node) : undefined,
      revision: isSet(object.revision) ? globalThis.Number(object.revision) : 0,
      properties: globalThis.Array.isArray(object?.properties)
        ? object.properties.map((e: any) => globalThis.String(e))
        : [],
    };
  },

  toJSON(message: EditData): unknown {
    const obj: any = {};
    if (message.kind !== 0) {
      obj.kind = editKindToJSON(message.kind);
    }
    if (message.moduleId !== "") {
      obj.moduleId = message.moduleId;
    }
    if (message.node !== undefined) {
      obj.node = SomeNodeData.toJSON(message.node);
    }
    if (message.revision !== 0) {
      obj.revision = Math.round(message.revision);
    }
    if (message.properties?.length) {
      obj.properties = message.properties;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<EditData>, I>>(base?: I): EditData {
    return EditData.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<EditData>, I>>(object: I): EditData {
    const message = createBaseEditData();
    message.kind = object.kind ?? 0;
    message.moduleId = object.moduleId ?? "";
    message.node =
      object.node !== undefined && object.node !== null ? SomeNodeData.fromPartial(object.node) : undefined;
    message.revision = object.revision ?? 0;
    message.properties = object.properties?.map((e) => e) || [];
    return message;
  },
};

function createBaseClientOrigin(): ClientOrigin {
  return { clientType: 0, clientId: "", nonce: "" };
}

export const ClientOrigin = {
  encode(message: ClientOrigin, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.clientType !== 0) {
      writer.uint32(8).int32(message.clientType);
    }
    if (message.clientId !== "") {
      writer.uint32(18).string(message.clientId);
    }
    if (message.nonce !== "") {
      writer.uint32(26).string(message.nonce);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ClientOrigin {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseClientOrigin();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.clientType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.clientId = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.nonce = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ClientOrigin {
    return {
      clientType: isSet(object.clientType) ? clientTypeFromJSON(object.clientType) : 0,
      clientId: isSet(object.clientId) ? globalThis.String(object.clientId) : "",
      nonce: isSet(object.nonce) ? globalThis.String(object.nonce) : "",
    };
  },

  toJSON(message: ClientOrigin): unknown {
    const obj: any = {};
    if (message.clientType !== 0) {
      obj.clientType = clientTypeToJSON(message.clientType);
    }
    if (message.clientId !== "") {
      obj.clientId = message.clientId;
    }
    if (message.nonce !== "") {
      obj.nonce = message.nonce;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ClientOrigin>, I>>(base?: I): ClientOrigin {
    return ClientOrigin.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ClientOrigin>, I>>(object: I): ClientOrigin {
    const message = createBaseClientOrigin();
    message.clientType = object.clientType ?? 0;
    message.clientId = object.clientId ?? "";
    message.nonce = object.nonce ?? "";
    return message;
  },
};

function createBaseNodePointer(): NodePointer {
  return { nodeType: 0, node: undefined };
}

export const NodePointer = {
  encode(message: NodePointer, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.nodeType !== 0) {
      writer.uint32(8).int32(message.nodeType);
    }
    switch (message.node?.$case) {
      case "nodeCk":
        writer.uint32(18).string(message.node.nodeCk);
        break;
      case "nodeId":
        writer.uint32(26).string(message.node.nodeId);
        break;
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): NodePointer {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseNodePointer();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.nodeType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.node = { $case: "nodeCk", nodeCk: reader.string() };
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.node = { $case: "nodeId", nodeId: reader.string() };
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): NodePointer {
    return {
      nodeType: isSet(object.nodeType) ? nodeTypeFromJSON(object.nodeType) : 0,
      node: isSet(object.nodeCk)
        ? { $case: "nodeCk", nodeCk: globalThis.String(object.nodeCk) }
        : isSet(object.nodeId)
        ? { $case: "nodeId", nodeId: globalThis.String(object.nodeId) }
        : undefined,
    };
  },

  toJSON(message: NodePointer): unknown {
    const obj: any = {};
    if (message.nodeType !== 0) {
      obj.nodeType = nodeTypeToJSON(message.nodeType);
    }
    if (message.node?.$case === "nodeCk") {
      obj.nodeCk = message.node.nodeCk;
    }
    if (message.node?.$case === "nodeId") {
      obj.nodeId = message.node.nodeId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<NodePointer>, I>>(base?: I): NodePointer {
    return NodePointer.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<NodePointer>, I>>(object: I): NodePointer {
    const message = createBaseNodePointer();
    message.nodeType = object.nodeType ?? 0;
    if (object.node?.$case === "nodeCk" && object.node?.nodeCk !== undefined && object.node?.nodeCk !== null) {
      message.node = { $case: "nodeCk", nodeCk: object.node.nodeCk };
    }
    if (object.node?.$case === "nodeId" && object.node?.nodeId !== undefined && object.node?.nodeId !== null) {
      message.node = { $case: "nodeId", nodeId: object.node.nodeId };
    }
    return message;
  },
};

function createBaseDidCreateProjectRequest(): DidCreateProjectRequest {
  return { projectId: "" };
}

export const DidCreateProjectRequest = {
  encode(message: DidCreateProjectRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.projectId !== "") {
      writer.uint32(10).string(message.projectId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DidCreateProjectRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDidCreateProjectRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.projectId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): DidCreateProjectRequest {
    return { projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "" };
  },

  toJSON(message: DidCreateProjectRequest): unknown {
    const obj: any = {};
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<DidCreateProjectRequest>, I>>(base?: I): DidCreateProjectRequest {
    return DidCreateProjectRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<DidCreateProjectRequest>, I>>(object: I): DidCreateProjectRequest {
    const message = createBaseDidCreateProjectRequest();
    message.projectId = object.projectId ?? "";
    return message;
  },
};

function createBaseGetNotificationsRequest(): GetNotificationsRequest {
  return {};
}

export const GetNotificationsRequest = {
  encode(_: GetNotificationsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetNotificationsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetNotificationsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(_: any): GetNotificationsRequest {
    return {};
  },

  toJSON(_: GetNotificationsRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetNotificationsRequest>, I>>(base?: I): GetNotificationsRequest {
    return GetNotificationsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetNotificationsRequest>, I>>(_: I): GetNotificationsRequest {
    const message = createBaseGetNotificationsRequest();
    return message;
  },
};

function createBaseGetNotificationsResponse(): GetNotificationsResponse {
  return {};
}

export const GetNotificationsResponse = {
  encode(_: GetNotificationsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetNotificationsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetNotificationsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(_: any): GetNotificationsResponse {
    return {};
  },

  toJSON(_: GetNotificationsResponse): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetNotificationsResponse>, I>>(base?: I): GetNotificationsResponse {
    return GetNotificationsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetNotificationsResponse>, I>>(_: I): GetNotificationsResponse {
    const message = createBaseGetNotificationsResponse();
    return message;
  },
};

function createBaseGetWorkerChangesRequest(): GetWorkerChangesRequest {
  return {};
}

export const GetWorkerChangesRequest = {
  encode(_: GetWorkerChangesRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetWorkerChangesRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetWorkerChangesRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(_: any): GetWorkerChangesRequest {
    return {};
  },

  toJSON(_: GetWorkerChangesRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetWorkerChangesRequest>, I>>(base?: I): GetWorkerChangesRequest {
    return GetWorkerChangesRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetWorkerChangesRequest>, I>>(_: I): GetWorkerChangesRequest {
    const message = createBaseGetWorkerChangesRequest();
    return message;
  },
};

function createBaseGetWorkerChangesResponse(): GetWorkerChangesResponse {
  return { workerSets: [] };
}

export const GetWorkerChangesResponse = {
  encode(message: GetWorkerChangesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.workerSets) {
      WorkerSetData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetWorkerChangesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetWorkerChangesResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.workerSets.push(WorkerSetData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetWorkerChangesResponse {
    return {
      workerSets: globalThis.Array.isArray(object?.workerSets)
        ? object.workerSets.map((e: any) => WorkerSetData.fromJSON(e))
        : [],
    };
  },

  toJSON(message: GetWorkerChangesResponse): unknown {
    const obj: any = {};
    if (message.workerSets?.length) {
      obj.workerSets = message.workerSets.map((e) => WorkerSetData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetWorkerChangesResponse>, I>>(base?: I): GetWorkerChangesResponse {
    return GetWorkerChangesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetWorkerChangesResponse>, I>>(object: I): GetWorkerChangesResponse {
    const message = createBaseGetWorkerChangesResponse();
    message.workerSets = object.workerSets?.map((e) => WorkerSetData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseConfigureWorkerSetRequest(): ConfigureWorkerSetRequest {
  return { projectId: "", profile: 0, region: 0, targetReplicas: 0 };
}

export const ConfigureWorkerSetRequest = {
  encode(message: ConfigureWorkerSetRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.projectId !== "") {
      writer.uint32(10).string(message.projectId);
    }
    if (message.profile !== 0) {
      writer.uint32(16).int32(message.profile);
    }
    if (message.region !== 0) {
      writer.uint32(24).int32(message.region);
    }
    if (message.targetReplicas !== 0) {
      writer.uint32(32).int32(message.targetReplicas);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ConfigureWorkerSetRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseConfigureWorkerSetRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.projectId = reader.string();
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.profile = reader.int32() as any;
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.region = reader.int32() as any;
          continue;
        case 4:
          if (tag !== 32) {
            break;
          }

          message.targetReplicas = reader.int32();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ConfigureWorkerSetRequest {
    return {
      projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "",
      profile: isSet(object.profile) ? workerProfileFromJSON(object.profile) : 0,
      region: isSet(object.region) ? projectRegionFromJSON(object.region) : 0,
      targetReplicas: isSet(object.targetReplicas) ? globalThis.Number(object.targetReplicas) : 0,
    };
  },

  toJSON(message: ConfigureWorkerSetRequest): unknown {
    const obj: any = {};
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    if (message.profile !== 0) {
      obj.profile = workerProfileToJSON(message.profile);
    }
    if (message.region !== 0) {
      obj.region = projectRegionToJSON(message.region);
    }
    if (message.targetReplicas !== 0) {
      obj.targetReplicas = Math.round(message.targetReplicas);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ConfigureWorkerSetRequest>, I>>(base?: I): ConfigureWorkerSetRequest {
    return ConfigureWorkerSetRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ConfigureWorkerSetRequest>, I>>(object: I): ConfigureWorkerSetRequest {
    const message = createBaseConfigureWorkerSetRequest();
    message.projectId = object.projectId ?? "";
    message.profile = object.profile ?? 0;
    message.region = object.region ?? 0;
    message.targetReplicas = object.targetReplicas ?? 0;
    return message;
  },
};

function createBaseConfigureWorkerSetResponse(): ConfigureWorkerSetResponse {
  return { workerSet: undefined };
}

export const ConfigureWorkerSetResponse = {
  encode(message: ConfigureWorkerSetResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.workerSet !== undefined) {
      WorkerSetData.encode(message.workerSet, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ConfigureWorkerSetResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseConfigureWorkerSetResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.workerSet = WorkerSetData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ConfigureWorkerSetResponse {
    return { workerSet: isSet(object.workerSet) ? WorkerSetData.fromJSON(object.workerSet) : undefined };
  },

  toJSON(message: ConfigureWorkerSetResponse): unknown {
    const obj: any = {};
    if (message.workerSet !== undefined) {
      obj.workerSet = WorkerSetData.toJSON(message.workerSet);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ConfigureWorkerSetResponse>, I>>(base?: I): ConfigureWorkerSetResponse {
    return ConfigureWorkerSetResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ConfigureWorkerSetResponse>, I>>(object: I): ConfigureWorkerSetResponse {
    const message = createBaseConfigureWorkerSetResponse();
    message.workerSet =
      object.workerSet !== undefined && object.workerSet !== null
        ? WorkerSetData.fromPartial(object.workerSet)
        : undefined;
    return message;
  },
};

function createBaseRestartWorkerSetRequest(): RestartWorkerSetRequest {
  return { projectId: "" };
}

export const RestartWorkerSetRequest = {
  encode(message: RestartWorkerSetRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.projectId !== "") {
      writer.uint32(10).string(message.projectId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RestartWorkerSetRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRestartWorkerSetRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.projectId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RestartWorkerSetRequest {
    return { projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "" };
  },

  toJSON(message: RestartWorkerSetRequest): unknown {
    const obj: any = {};
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RestartWorkerSetRequest>, I>>(base?: I): RestartWorkerSetRequest {
    return RestartWorkerSetRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RestartWorkerSetRequest>, I>>(object: I): RestartWorkerSetRequest {
    const message = createBaseRestartWorkerSetRequest();
    message.projectId = object.projectId ?? "";
    return message;
  },
};

function createBaseGetWorkerImageRequest(): GetWorkerImageRequest {
  return { projectId: "" };
}

export const GetWorkerImageRequest = {
  encode(message: GetWorkerImageRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.projectId !== "") {
      writer.uint32(10).string(message.projectId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetWorkerImageRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetWorkerImageRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.projectId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetWorkerImageRequest {
    return { projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "" };
  },

  toJSON(message: GetWorkerImageRequest): unknown {
    const obj: any = {};
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetWorkerImageRequest>, I>>(base?: I): GetWorkerImageRequest {
    return GetWorkerImageRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetWorkerImageRequest>, I>>(object: I): GetWorkerImageRequest {
    const message = createBaseGetWorkerImageRequest();
    message.projectId = object.projectId ?? "";
    return message;
  },
};

function createBaseGetWorkerImageResponse(): GetWorkerImageResponse {
  return { image: undefined };
}

export const GetWorkerImageResponse = {
  encode(message: GetWorkerImageResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.image !== undefined) {
      WorkerImageData.encode(message.image, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetWorkerImageResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetWorkerImageResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.image = WorkerImageData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetWorkerImageResponse {
    return { image: isSet(object.image) ? WorkerImageData.fromJSON(object.image) : undefined };
  },

  toJSON(message: GetWorkerImageResponse): unknown {
    const obj: any = {};
    if (message.image !== undefined) {
      obj.image = WorkerImageData.toJSON(message.image);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetWorkerImageResponse>, I>>(base?: I): GetWorkerImageResponse {
    return GetWorkerImageResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetWorkerImageResponse>, I>>(object: I): GetWorkerImageResponse {
    const message = createBaseGetWorkerImageResponse();
    message.image =
      object.image !== undefined && object.image !== null ? WorkerImageData.fromPartial(object.image) : undefined;
    return message;
  },
};

function createBasePingWorkerSetRequest(): PingWorkerSetRequest {
  return { projectId: "" };
}

export const PingWorkerSetRequest = {
  encode(message: PingWorkerSetRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.projectId !== "") {
      writer.uint32(10).string(message.projectId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingWorkerSetRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingWorkerSetRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.projectId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PingWorkerSetRequest {
    return { projectId: isSet(object.projectId) ? globalThis.String(object.projectId) : "" };
  },

  toJSON(message: PingWorkerSetRequest): unknown {
    const obj: any = {};
    if (message.projectId !== "") {
      obj.projectId = message.projectId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PingWorkerSetRequest>, I>>(base?: I): PingWorkerSetRequest {
    return PingWorkerSetRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PingWorkerSetRequest>, I>>(object: I): PingWorkerSetRequest {
    const message = createBasePingWorkerSetRequest();
    message.projectId = object.projectId ?? "";
    return message;
  },
};

function createBaseGetBenchChangesRequest(): GetBenchChangesRequest {
  return { afterChangeMarker: 0 };
}

export const GetBenchChangesRequest = {
  encode(message: GetBenchChangesRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.afterChangeMarker !== 0) {
      writer.uint32(8).int64(message.afterChangeMarker);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetBenchChangesRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetBenchChangesRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.afterChangeMarker = longToNumber(reader.int64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetBenchChangesRequest {
    return { afterChangeMarker: isSet(object.afterChangeMarker) ? globalThis.Number(object.afterChangeMarker) : 0 };
  },

  toJSON(message: GetBenchChangesRequest): unknown {
    const obj: any = {};
    if (message.afterChangeMarker !== 0) {
      obj.afterChangeMarker = Math.round(message.afterChangeMarker);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetBenchChangesRequest>, I>>(base?: I): GetBenchChangesRequest {
    return GetBenchChangesRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetBenchChangesRequest>, I>>(object: I): GetBenchChangesRequest {
    const message = createBaseGetBenchChangesRequest();
    message.afterChangeMarker = object.afterChangeMarker ?? 0;
    return message;
  },
};

function createBaseGetBenchChangesResponse(): GetBenchChangesResponse {
  return { edits: [], snapshot: undefined };
}

export const GetBenchChangesResponse = {
  encode(message: GetBenchChangesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.edits) {
      EditData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    if (message.snapshot !== undefined) {
      ModuleData.encode(message.snapshot, writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetBenchChangesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetBenchChangesResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.edits.push(EditData.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.snapshot = ModuleData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetBenchChangesResponse {
    return {
      edits: globalThis.Array.isArray(object?.edits) ? object.edits.map((e: any) => EditData.fromJSON(e)) : [],
      snapshot: isSet(object.snapshot) ? ModuleData.fromJSON(object.snapshot) : undefined,
    };
  },

  toJSON(message: GetBenchChangesResponse): unknown {
    const obj: any = {};
    if (message.edits?.length) {
      obj.edits = message.edits.map((e) => EditData.toJSON(e));
    }
    if (message.snapshot !== undefined) {
      obj.snapshot = ModuleData.toJSON(message.snapshot);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetBenchChangesResponse>, I>>(base?: I): GetBenchChangesResponse {
    return GetBenchChangesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetBenchChangesResponse>, I>>(object: I): GetBenchChangesResponse {
    const message = createBaseGetBenchChangesResponse();
    message.edits = object.edits?.map((e) => EditData.fromPartial(e)) || [];
    message.snapshot =
      object.snapshot !== undefined && object.snapshot !== null ? ModuleData.fromPartial(object.snapshot) : undefined;
    return message;
  },
};

function createBaseGetModuleEditsRequest(): GetModuleEditsRequest {
  return { afterEditMarker: 0 };
}

export const GetModuleEditsRequest = {
  encode(message: GetModuleEditsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.afterEditMarker !== 0) {
      writer.uint32(8).int64(message.afterEditMarker);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetModuleEditsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetModuleEditsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.afterEditMarker = longToNumber(reader.int64() as Long);
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetModuleEditsRequest {
    return { afterEditMarker: isSet(object.afterEditMarker) ? globalThis.Number(object.afterEditMarker) : 0 };
  },

  toJSON(message: GetModuleEditsRequest): unknown {
    const obj: any = {};
    if (message.afterEditMarker !== 0) {
      obj.afterEditMarker = Math.round(message.afterEditMarker);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetModuleEditsRequest>, I>>(base?: I): GetModuleEditsRequest {
    return GetModuleEditsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetModuleEditsRequest>, I>>(object: I): GetModuleEditsRequest {
    const message = createBaseGetModuleEditsRequest();
    message.afterEditMarker = object.afterEditMarker ?? 0;
    return message;
  },
};

function createBaseGetModuleEditsResponse(): GetModuleEditsResponse {
  return { edits: [] };
}

export const GetModuleEditsResponse = {
  encode(message: GetModuleEditsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.edits) {
      EditData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetModuleEditsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetModuleEditsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.edits.push(EditData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetModuleEditsResponse {
    return { edits: globalThis.Array.isArray(object?.edits) ? object.edits.map((e: any) => EditData.fromJSON(e)) : [] };
  },

  toJSON(message: GetModuleEditsResponse): unknown {
    const obj: any = {};
    if (message.edits?.length) {
      obj.edits = message.edits.map((e) => EditData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetModuleEditsResponse>, I>>(base?: I): GetModuleEditsResponse {
    return GetModuleEditsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetModuleEditsResponse>, I>>(object: I): GetModuleEditsResponse {
    const message = createBaseGetModuleEditsResponse();
    message.edits = object.edits?.map((e) => EditData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseGetLogsRequest(): GetLogsRequest {
  return {};
}

export const GetLogsRequest = {
  encode(_: GetLogsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetLogsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetLogsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(_: any): GetLogsRequest {
    return {};
  },

  toJSON(_: GetLogsRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetLogsRequest>, I>>(base?: I): GetLogsRequest {
    return GetLogsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetLogsRequest>, I>>(_: I): GetLogsRequest {
    const message = createBaseGetLogsRequest();
    return message;
  },
};

function createBaseGetLogsResponse(): GetLogsResponse {
  return { logs: [] };
}

export const GetLogsResponse = {
  encode(message: GetLogsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.logs) {
      LogEntryData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetLogsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetLogsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.logs.push(LogEntryData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): GetLogsResponse {
    return {
      logs: globalThis.Array.isArray(object?.logs) ? object.logs.map((e: any) => LogEntryData.fromJSON(e)) : [],
    };
  },

  toJSON(message: GetLogsResponse): unknown {
    const obj: any = {};
    if (message.logs?.length) {
      obj.logs = message.logs.map((e) => LogEntryData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<GetLogsResponse>, I>>(base?: I): GetLogsResponse {
    return GetLogsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetLogsResponse>, I>>(object: I): GetLogsResponse {
    const message = createBaseGetLogsResponse();
    message.logs = object.logs?.map((e) => LogEntryData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseReadNodesRequest(): ReadNodesRequest {
  return { roots: [], includeDeferredProperties: false };
}

export const ReadNodesRequest = {
  encode(message: ReadNodesRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.roots) {
      NodePointer.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    if (message.includeDeferredProperties === true) {
      writer.uint32(16).bool(message.includeDeferredProperties);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ReadNodesRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseReadNodesRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.roots.push(NodePointer.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 16) {
            break;
          }

          message.includeDeferredProperties = reader.bool();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ReadNodesRequest {
    return {
      roots: globalThis.Array.isArray(object?.roots) ? object.roots.map((e: any) => NodePointer.fromJSON(e)) : [],
      includeDeferredProperties: isSet(object.includeDeferredProperties)
        ? globalThis.Boolean(object.includeDeferredProperties)
        : false,
    };
  },

  toJSON(message: ReadNodesRequest): unknown {
    const obj: any = {};
    if (message.roots?.length) {
      obj.roots = message.roots.map((e) => NodePointer.toJSON(e));
    }
    if (message.includeDeferredProperties === true) {
      obj.includeDeferredProperties = message.includeDeferredProperties;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ReadNodesRequest>, I>>(base?: I): ReadNodesRequest {
    return ReadNodesRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ReadNodesRequest>, I>>(object: I): ReadNodesRequest {
    const message = createBaseReadNodesRequest();
    message.roots = object.roots?.map((e) => NodePointer.fromPartial(e)) || [];
    message.includeDeferredProperties = object.includeDeferredProperties ?? false;
    return message;
  },
};

function createBaseReadNodesResponse(): ReadNodesResponse {
  return { nodes: [] };
}

export const ReadNodesResponse = {
  encode(message: ReadNodesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.nodes) {
      SomeNodeData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ReadNodesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseReadNodesResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
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

  fromJSON(object: any): ReadNodesResponse {
    return {
      nodes: globalThis.Array.isArray(object?.nodes) ? object.nodes.map((e: any) => SomeNodeData.fromJSON(e)) : [],
    };
  },

  toJSON(message: ReadNodesResponse): unknown {
    const obj: any = {};
    if (message.nodes?.length) {
      obj.nodes = message.nodes.map((e) => SomeNodeData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ReadNodesResponse>, I>>(base?: I): ReadNodesResponse {
    return ReadNodesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ReadNodesResponse>, I>>(object: I): ReadNodesResponse {
    const message = createBaseReadNodesResponse();
    message.nodes = object.nodes?.map((e) => SomeNodeData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseSearchNodesRequest(): SearchNodesRequest {
  return { nodeType: 0, nodeCk: "", filter: undefined, sort: [], limit: 0, after: "" };
}

export const SearchNodesRequest = {
  encode(message: SearchNodesRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.nodeType !== 0) {
      writer.uint32(8).int32(message.nodeType);
    }
    if (message.nodeCk !== "") {
      writer.uint32(18).string(message.nodeCk);
    }
    if (message.filter !== undefined) {
      ExpressionData.encode(message.filter, writer.uint32(26).fork()).ldelim();
    }
    for (const v of message.sort) {
      ExpressionData.encode(v!, writer.uint32(34).fork()).ldelim();
    }
    if (message.limit !== 0) {
      writer.uint32(40).int32(message.limit);
    }
    if (message.after !== "") {
      writer.uint32(50).string(message.after);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SearchNodesRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSearchNodesRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.nodeType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.nodeCk = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.filter = ExpressionData.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.sort.push(ExpressionData.decode(reader, reader.uint32()));
          continue;
        case 5:
          if (tag !== 40) {
            break;
          }

          message.limit = reader.int32();
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.after = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SearchNodesRequest {
    return {
      nodeType: isSet(object.nodeType) ? nodeTypeFromJSON(object.nodeType) : 0,
      nodeCk: isSet(object.nodeCk) ? globalThis.String(object.nodeCk) : "",
      filter: isSet(object.filter) ? ExpressionData.fromJSON(object.filter) : undefined,
      sort: globalThis.Array.isArray(object?.sort) ? object.sort.map((e: any) => ExpressionData.fromJSON(e)) : [],
      limit: isSet(object.limit) ? globalThis.Number(object.limit) : 0,
      after: isSet(object.after) ? globalThis.String(object.after) : "",
    };
  },

  toJSON(message: SearchNodesRequest): unknown {
    const obj: any = {};
    if (message.nodeType !== 0) {
      obj.nodeType = nodeTypeToJSON(message.nodeType);
    }
    if (message.nodeCk !== "") {
      obj.nodeCk = message.nodeCk;
    }
    if (message.filter !== undefined) {
      obj.filter = ExpressionData.toJSON(message.filter);
    }
    if (message.sort?.length) {
      obj.sort = message.sort.map((e) => ExpressionData.toJSON(e));
    }
    if (message.limit !== 0) {
      obj.limit = Math.round(message.limit);
    }
    if (message.after !== "") {
      obj.after = message.after;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SearchNodesRequest>, I>>(base?: I): SearchNodesRequest {
    return SearchNodesRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SearchNodesRequest>, I>>(object: I): SearchNodesRequest {
    const message = createBaseSearchNodesRequest();
    message.nodeType = object.nodeType ?? 0;
    message.nodeCk = object.nodeCk ?? "";
    message.filter =
      object.filter !== undefined && object.filter !== null ? ExpressionData.fromPartial(object.filter) : undefined;
    message.sort = object.sort?.map((e) => ExpressionData.fromPartial(e)) || [];
    message.limit = object.limit ?? 0;
    message.after = object.after ?? "";
    return message;
  },
};

function createBaseSearchNodesResponse(): SearchNodesResponse {
  return { nodes: [], cursors: [], startCursor: "" };
}

export const SearchNodesResponse = {
  encode(message: SearchNodesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.nodes) {
      SomeNodeData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    for (const v of message.cursors) {
      writer.uint32(18).string(v!);
    }
    if (message.startCursor !== "") {
      writer.uint32(26).string(message.startCursor);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SearchNodesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSearchNodesResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.nodes.push(SomeNodeData.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.cursors.push(reader.string());
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.startCursor = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SearchNodesResponse {
    return {
      nodes: globalThis.Array.isArray(object?.nodes) ? object.nodes.map((e: any) => SomeNodeData.fromJSON(e)) : [],
      cursors: globalThis.Array.isArray(object?.cursors) ? object.cursors.map((e: any) => globalThis.String(e)) : [],
      startCursor: isSet(object.startCursor) ? globalThis.String(object.startCursor) : "",
    };
  },

  toJSON(message: SearchNodesResponse): unknown {
    const obj: any = {};
    if (message.nodes?.length) {
      obj.nodes = message.nodes.map((e) => SomeNodeData.toJSON(e));
    }
    if (message.cursors?.length) {
      obj.cursors = message.cursors;
    }
    if (message.startCursor !== "") {
      obj.startCursor = message.startCursor;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SearchNodesResponse>, I>>(base?: I): SearchNodesResponse {
    return SearchNodesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SearchNodesResponse>, I>>(object: I): SearchNodesResponse {
    const message = createBaseSearchNodesResponse();
    message.nodes = object.nodes?.map((e) => SomeNodeData.fromPartial(e)) || [];
    message.cursors = object.cursors?.map((e) => e) || [];
    message.startCursor = object.startCursor ?? "";
    return message;
  },
};

function createBaseCommitEditsRequest(): CommitEditsRequest {
  return { edits: [] };
}

export const CommitEditsRequest = {
  encode(message: CommitEditsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.edits) {
      EditData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): CommitEditsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseCommitEditsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.edits.push(EditData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): CommitEditsRequest {
    return { edits: globalThis.Array.isArray(object?.edits) ? object.edits.map((e: any) => EditData.fromJSON(e)) : [] };
  },

  toJSON(message: CommitEditsRequest): unknown {
    const obj: any = {};
    if (message.edits?.length) {
      obj.edits = message.edits.map((e) => EditData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<CommitEditsRequest>, I>>(base?: I): CommitEditsRequest {
    return CommitEditsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<CommitEditsRequest>, I>>(object: I): CommitEditsRequest {
    const message = createBaseCommitEditsRequest();
    message.edits = object.edits?.map((e) => EditData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseCommitEditsResponse(): CommitEditsResponse {
  return { changedNodes: [] };
}

export const CommitEditsResponse = {
  encode(message: CommitEditsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.changedNodes) {
      SomeNodeData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): CommitEditsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseCommitEditsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.changedNodes.push(SomeNodeData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): CommitEditsResponse {
    return {
      changedNodes: globalThis.Array.isArray(object?.changedNodes)
        ? object.changedNodes.map((e: any) => SomeNodeData.fromJSON(e))
        : [],
    };
  },

  toJSON(message: CommitEditsResponse): unknown {
    const obj: any = {};
    if (message.changedNodes?.length) {
      obj.changedNodes = message.changedNodes.map((e) => SomeNodeData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<CommitEditsResponse>, I>>(base?: I): CommitEditsResponse {
    return CommitEditsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<CommitEditsResponse>, I>>(object: I): CommitEditsResponse {
    const message = createBaseCommitEditsResponse();
    message.changedNodes = object.changedNodes?.map((e) => SomeNodeData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseUploadBlobRequest(): UploadBlobRequest {
  return { blob: undefined };
}

export const UploadBlobRequest = {
  encode(message: UploadBlobRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.blob !== undefined) {
      BlobData.encode(message.blob, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UploadBlobRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUploadBlobRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.blob = BlobData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): UploadBlobRequest {
    return { blob: isSet(object.blob) ? BlobData.fromJSON(object.blob) : undefined };
  },

  toJSON(message: UploadBlobRequest): unknown {
    const obj: any = {};
    if (message.blob !== undefined) {
      obj.blob = BlobData.toJSON(message.blob);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<UploadBlobRequest>, I>>(base?: I): UploadBlobRequest {
    return UploadBlobRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<UploadBlobRequest>, I>>(object: I): UploadBlobRequest {
    const message = createBaseUploadBlobRequest();
    message.blob = object.blob !== undefined && object.blob !== null ? BlobData.fromPartial(object.blob) : undefined;
    return message;
  },
};

function createBaseUploadBlobResponse(): UploadBlobResponse {
  return { postUrl: "", expiresAt: undefined };
}

export const UploadBlobResponse = {
  encode(message: UploadBlobResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.postUrl !== "") {
      writer.uint32(10).string(message.postUrl);
    }
    if (message.expiresAt !== undefined) {
      Timestamp.encode(toTimestamp(message.expiresAt), writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): UploadBlobResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseUploadBlobResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.postUrl = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.expiresAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): UploadBlobResponse {
    return {
      postUrl: isSet(object.postUrl) ? globalThis.String(object.postUrl) : "",
      expiresAt: isSet(object.expiresAt) ? fromJsonTimestamp(object.expiresAt) : undefined,
    };
  },

  toJSON(message: UploadBlobResponse): unknown {
    const obj: any = {};
    if (message.postUrl !== "") {
      obj.postUrl = message.postUrl;
    }
    if (message.expiresAt !== undefined) {
      obj.expiresAt = message.expiresAt.toISOString();
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<UploadBlobResponse>, I>>(base?: I): UploadBlobResponse {
    return UploadBlobResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<UploadBlobResponse>, I>>(object: I): UploadBlobResponse {
    const message = createBaseUploadBlobResponse();
    message.postUrl = object.postUrl ?? "";
    message.expiresAt = object.expiresAt ?? undefined;
    return message;
  },
};

function createBaseDownloadBlobRequest(): DownloadBlobRequest {
  return { blob: undefined };
}

export const DownloadBlobRequest = {
  encode(message: DownloadBlobRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.blob !== undefined) {
      BlobData.encode(message.blob, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DownloadBlobRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDownloadBlobRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.blob = BlobData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): DownloadBlobRequest {
    return { blob: isSet(object.blob) ? BlobData.fromJSON(object.blob) : undefined };
  },

  toJSON(message: DownloadBlobRequest): unknown {
    const obj: any = {};
    if (message.blob !== undefined) {
      obj.blob = BlobData.toJSON(message.blob);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<DownloadBlobRequest>, I>>(base?: I): DownloadBlobRequest {
    return DownloadBlobRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<DownloadBlobRequest>, I>>(object: I): DownloadBlobRequest {
    const message = createBaseDownloadBlobRequest();
    message.blob = object.blob !== undefined && object.blob !== null ? BlobData.fromPartial(object.blob) : undefined;
    return message;
  },
};

function createBaseDownloadBlobResponse(): DownloadBlobResponse {
  return { getUrl: "", expiresAt: undefined };
}

export const DownloadBlobResponse = {
  encode(message: DownloadBlobResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.getUrl !== "") {
      writer.uint32(10).string(message.getUrl);
    }
    if (message.expiresAt !== undefined) {
      Timestamp.encode(toTimestamp(message.expiresAt), writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): DownloadBlobResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseDownloadBlobResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.getUrl = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.expiresAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): DownloadBlobResponse {
    return {
      getUrl: isSet(object.getUrl) ? globalThis.String(object.getUrl) : "",
      expiresAt: isSet(object.expiresAt) ? fromJsonTimestamp(object.expiresAt) : undefined,
    };
  },

  toJSON(message: DownloadBlobResponse): unknown {
    const obj: any = {};
    if (message.getUrl !== "") {
      obj.getUrl = message.getUrl;
    }
    if (message.expiresAt !== undefined) {
      obj.expiresAt = message.expiresAt.toISOString();
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<DownloadBlobResponse>, I>>(base?: I): DownloadBlobResponse {
    return DownloadBlobResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<DownloadBlobResponse>, I>>(object: I): DownloadBlobResponse {
    const message = createBaseDownloadBlobResponse();
    message.getUrl = object.getUrl ?? "";
    message.expiresAt = object.expiresAt ?? undefined;
    return message;
  },
};

function createBaseRevealSecretRequest(): RevealSecretRequest {
  return { secret: undefined };
}

export const RevealSecretRequest = {
  encode(message: RevealSecretRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.secret !== undefined) {
      SecretData.encode(message.secret, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RevealSecretRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRevealSecretRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.secret = SecretData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RevealSecretRequest {
    return { secret: isSet(object.secret) ? SecretData.fromJSON(object.secret) : undefined };
  },

  toJSON(message: RevealSecretRequest): unknown {
    const obj: any = {};
    if (message.secret !== undefined) {
      obj.secret = SecretData.toJSON(message.secret);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RevealSecretRequest>, I>>(base?: I): RevealSecretRequest {
    return RevealSecretRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RevealSecretRequest>, I>>(object: I): RevealSecretRequest {
    const message = createBaseRevealSecretRequest();
    message.secret =
      object.secret !== undefined && object.secret !== null ? SecretData.fromPartial(object.secret) : undefined;
    return message;
  },
};

function createBaseRevealSecretResponse(): RevealSecretResponse {
  return { secret: undefined };
}

export const RevealSecretResponse = {
  encode(message: RevealSecretResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.secret !== undefined) {
      SecretData.encode(message.secret, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RevealSecretResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRevealSecretResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.secret = SecretData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RevealSecretResponse {
    return { secret: isSet(object.secret) ? SecretData.fromJSON(object.secret) : undefined };
  },

  toJSON(message: RevealSecretResponse): unknown {
    const obj: any = {};
    if (message.secret !== undefined) {
      obj.secret = SecretData.toJSON(message.secret);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RevealSecretResponse>, I>>(base?: I): RevealSecretResponse {
    return RevealSecretResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RevealSecretResponse>, I>>(object: I): RevealSecretResponse {
    const message = createBaseRevealSecretResponse();
    message.secret =
      object.secret !== undefined && object.secret !== null ? SecretData.fromPartial(object.secret) : undefined;
    return message;
  },
};

function createBasePasteNodesRequest(): PasteNodesRequest {
  return {
    sourceModuleId: "",
    sourceNodes: [],
    targetIds: {},
    targetCks: {},
    targetParentIds: {},
    targetOrderKeys: {},
  };
}

export const PasteNodesRequest = {
  encode(message: PasteNodesRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.sourceModuleId !== "") {
      writer.uint32(10).string(message.sourceModuleId);
    }
    for (const v of message.sourceNodes) {
      NodePointer.encode(v!, writer.uint32(18).fork()).ldelim();
    }
    Object.entries(message.targetIds).forEach(([key, value]) => {
      PasteNodesRequest_TargetIdsEntry.encode({ key: key as any, value }, writer.uint32(26).fork()).ldelim();
    });
    Object.entries(message.targetCks).forEach(([key, value]) => {
      PasteNodesRequest_TargetCksEntry.encode({ key: key as any, value }, writer.uint32(34).fork()).ldelim();
    });
    Object.entries(message.targetParentIds).forEach(([key, value]) => {
      PasteNodesRequest_TargetParentIdsEntry.encode({ key: key as any, value }, writer.uint32(42).fork()).ldelim();
    });
    Object.entries(message.targetOrderKeys).forEach(([key, value]) => {
      PasteNodesRequest_TargetOrderKeysEntry.encode({ key: key as any, value }, writer.uint32(50).fork()).ldelim();
    });
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.sourceModuleId = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.sourceNodes.push(NodePointer.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          const entry3 = PasteNodesRequest_TargetIdsEntry.decode(reader, reader.uint32());
          if (entry3.value !== undefined) {
            message.targetIds[entry3.key] = entry3.value;
          }
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          const entry4 = PasteNodesRequest_TargetCksEntry.decode(reader, reader.uint32());
          if (entry4.value !== undefined) {
            message.targetCks[entry4.key] = entry4.value;
          }
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          const entry5 = PasteNodesRequest_TargetParentIdsEntry.decode(reader, reader.uint32());
          if (entry5.value !== undefined) {
            message.targetParentIds[entry5.key] = entry5.value;
          }
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          const entry6 = PasteNodesRequest_TargetOrderKeysEntry.decode(reader, reader.uint32());
          if (entry6.value !== undefined) {
            message.targetOrderKeys[entry6.key] = entry6.value;
          }
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesRequest {
    return {
      sourceModuleId: isSet(object.sourceModuleId) ? globalThis.String(object.sourceModuleId) : "",
      sourceNodes: globalThis.Array.isArray(object?.sourceNodes)
        ? object.sourceNodes.map((e: any) => NodePointer.fromJSON(e))
        : [],
      targetIds: isObject(object.targetIds)
        ? Object.entries(object.targetIds).reduce<{ [key: string]: string }>((acc, [key, value]) => {
            acc[key] = String(value);
            return acc;
          }, {})
        : {},
      targetCks: isObject(object.targetCks)
        ? Object.entries(object.targetCks).reduce<{ [key: string]: string }>((acc, [key, value]) => {
            acc[key] = String(value);
            return acc;
          }, {})
        : {},
      targetParentIds: isObject(object.targetParentIds)
        ? Object.entries(object.targetParentIds).reduce<{ [key: string]: string }>((acc, [key, value]) => {
            acc[key] = String(value);
            return acc;
          }, {})
        : {},
      targetOrderKeys: isObject(object.targetOrderKeys)
        ? Object.entries(object.targetOrderKeys).reduce<{ [key: string]: string }>((acc, [key, value]) => {
            acc[key] = String(value);
            return acc;
          }, {})
        : {},
    };
  },

  toJSON(message: PasteNodesRequest): unknown {
    const obj: any = {};
    if (message.sourceModuleId !== "") {
      obj.sourceModuleId = message.sourceModuleId;
    }
    if (message.sourceNodes?.length) {
      obj.sourceNodes = message.sourceNodes.map((e) => NodePointer.toJSON(e));
    }
    if (message.targetIds) {
      const entries = Object.entries(message.targetIds);
      if (entries.length > 0) {
        obj.targetIds = {};
        entries.forEach(([k, v]) => {
          obj.targetIds[k] = v;
        });
      }
    }
    if (message.targetCks) {
      const entries = Object.entries(message.targetCks);
      if (entries.length > 0) {
        obj.targetCks = {};
        entries.forEach(([k, v]) => {
          obj.targetCks[k] = v;
        });
      }
    }
    if (message.targetParentIds) {
      const entries = Object.entries(message.targetParentIds);
      if (entries.length > 0) {
        obj.targetParentIds = {};
        entries.forEach(([k, v]) => {
          obj.targetParentIds[k] = v;
        });
      }
    }
    if (message.targetOrderKeys) {
      const entries = Object.entries(message.targetOrderKeys);
      if (entries.length > 0) {
        obj.targetOrderKeys = {};
        entries.forEach(([k, v]) => {
          obj.targetOrderKeys[k] = v;
        });
      }
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesRequest>, I>>(base?: I): PasteNodesRequest {
    return PasteNodesRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesRequest>, I>>(object: I): PasteNodesRequest {
    const message = createBasePasteNodesRequest();
    message.sourceModuleId = object.sourceModuleId ?? "";
    message.sourceNodes = object.sourceNodes?.map((e) => NodePointer.fromPartial(e)) || [];
    message.targetIds = Object.entries(object.targetIds ?? {}).reduce<{ [key: string]: string }>(
      (acc, [key, value]) => {
        if (value !== undefined) {
          acc[key] = globalThis.String(value);
        }
        return acc;
      },
      {}
    );
    message.targetCks = Object.entries(object.targetCks ?? {}).reduce<{ [key: string]: string }>(
      (acc, [key, value]) => {
        if (value !== undefined) {
          acc[key] = globalThis.String(value);
        }
        return acc;
      },
      {}
    );
    message.targetParentIds = Object.entries(object.targetParentIds ?? {}).reduce<{ [key: string]: string }>(
      (acc, [key, value]) => {
        if (value !== undefined) {
          acc[key] = globalThis.String(value);
        }
        return acc;
      },
      {}
    );
    message.targetOrderKeys = Object.entries(object.targetOrderKeys ?? {}).reduce<{ [key: string]: string }>(
      (acc, [key, value]) => {
        if (value !== undefined) {
          acc[key] = globalThis.String(value);
        }
        return acc;
      },
      {}
    );
    return message;
  },
};

function createBasePasteNodesRequest_TargetIdsEntry(): PasteNodesRequest_TargetIdsEntry {
  return { key: "", value: "" };
}

export const PasteNodesRequest_TargetIdsEntry = {
  encode(message: PasteNodesRequest_TargetIdsEntry, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.value !== "") {
      writer.uint32(18).string(message.value);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesRequest_TargetIdsEntry {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesRequest_TargetIdsEntry();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.value = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesRequest_TargetIdsEntry {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isSet(object.value) ? globalThis.String(object.value) : "",
    };
  },

  toJSON(message: PasteNodesRequest_TargetIdsEntry): unknown {
    const obj: any = {};
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== "") {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesRequest_TargetIdsEntry>, I>>(
    base?: I
  ): PasteNodesRequest_TargetIdsEntry {
    return PasteNodesRequest_TargetIdsEntry.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesRequest_TargetIdsEntry>, I>>(
    object: I
  ): PasteNodesRequest_TargetIdsEntry {
    const message = createBasePasteNodesRequest_TargetIdsEntry();
    message.key = object.key ?? "";
    message.value = object.value ?? "";
    return message;
  },
};

function createBasePasteNodesRequest_TargetCksEntry(): PasteNodesRequest_TargetCksEntry {
  return { key: "", value: "" };
}

export const PasteNodesRequest_TargetCksEntry = {
  encode(message: PasteNodesRequest_TargetCksEntry, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.value !== "") {
      writer.uint32(18).string(message.value);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesRequest_TargetCksEntry {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesRequest_TargetCksEntry();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.value = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesRequest_TargetCksEntry {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isSet(object.value) ? globalThis.String(object.value) : "",
    };
  },

  toJSON(message: PasteNodesRequest_TargetCksEntry): unknown {
    const obj: any = {};
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== "") {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesRequest_TargetCksEntry>, I>>(
    base?: I
  ): PasteNodesRequest_TargetCksEntry {
    return PasteNodesRequest_TargetCksEntry.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesRequest_TargetCksEntry>, I>>(
    object: I
  ): PasteNodesRequest_TargetCksEntry {
    const message = createBasePasteNodesRequest_TargetCksEntry();
    message.key = object.key ?? "";
    message.value = object.value ?? "";
    return message;
  },
};

function createBasePasteNodesRequest_TargetParentIdsEntry(): PasteNodesRequest_TargetParentIdsEntry {
  return { key: "", value: "" };
}

export const PasteNodesRequest_TargetParentIdsEntry = {
  encode(message: PasteNodesRequest_TargetParentIdsEntry, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.value !== "") {
      writer.uint32(18).string(message.value);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesRequest_TargetParentIdsEntry {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesRequest_TargetParentIdsEntry();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.value = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesRequest_TargetParentIdsEntry {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isSet(object.value) ? globalThis.String(object.value) : "",
    };
  },

  toJSON(message: PasteNodesRequest_TargetParentIdsEntry): unknown {
    const obj: any = {};
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== "") {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesRequest_TargetParentIdsEntry>, I>>(
    base?: I
  ): PasteNodesRequest_TargetParentIdsEntry {
    return PasteNodesRequest_TargetParentIdsEntry.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesRequest_TargetParentIdsEntry>, I>>(
    object: I
  ): PasteNodesRequest_TargetParentIdsEntry {
    const message = createBasePasteNodesRequest_TargetParentIdsEntry();
    message.key = object.key ?? "";
    message.value = object.value ?? "";
    return message;
  },
};

function createBasePasteNodesRequest_TargetOrderKeysEntry(): PasteNodesRequest_TargetOrderKeysEntry {
  return { key: "", value: "" };
}

export const PasteNodesRequest_TargetOrderKeysEntry = {
  encode(message: PasteNodesRequest_TargetOrderKeysEntry, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.key !== "") {
      writer.uint32(10).string(message.key);
    }
    if (message.value !== "") {
      writer.uint32(18).string(message.value);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesRequest_TargetOrderKeysEntry {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesRequest_TargetOrderKeysEntry();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.key = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.value = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesRequest_TargetOrderKeysEntry {
    return {
      key: isSet(object.key) ? globalThis.String(object.key) : "",
      value: isSet(object.value) ? globalThis.String(object.value) : "",
    };
  },

  toJSON(message: PasteNodesRequest_TargetOrderKeysEntry): unknown {
    const obj: any = {};
    if (message.key !== "") {
      obj.key = message.key;
    }
    if (message.value !== "") {
      obj.value = message.value;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesRequest_TargetOrderKeysEntry>, I>>(
    base?: I
  ): PasteNodesRequest_TargetOrderKeysEntry {
    return PasteNodesRequest_TargetOrderKeysEntry.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesRequest_TargetOrderKeysEntry>, I>>(
    object: I
  ): PasteNodesRequest_TargetOrderKeysEntry {
    const message = createBasePasteNodesRequest_TargetOrderKeysEntry();
    message.key = object.key ?? "";
    message.value = object.value ?? "";
    return message;
  },
};

function createBasePasteNodesResponse(): PasteNodesResponse {
  return { pastedNodes: [] };
}

export const PasteNodesResponse = {
  encode(message: PasteNodesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.pastedNodes) {
      SomeNodeData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PasteNodesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePasteNodesResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.pastedNodes.push(SomeNodeData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PasteNodesResponse {
    return {
      pastedNodes: globalThis.Array.isArray(object?.pastedNodes)
        ? object.pastedNodes.map((e: any) => SomeNodeData.fromJSON(e))
        : [],
    };
  },

  toJSON(message: PasteNodesResponse): unknown {
    const obj: any = {};
    if (message.pastedNodes?.length) {
      obj.pastedNodes = message.pastedNodes.map((e) => SomeNodeData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PasteNodesResponse>, I>>(base?: I): PasteNodesResponse {
    return PasteNodesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PasteNodesResponse>, I>>(object: I): PasteNodesResponse {
    const message = createBasePasteNodesResponse();
    message.pastedNodes = object.pastedNodes?.map((e) => SomeNodeData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseSnapshotModuleRequest(): SnapshotModuleRequest {
  return { name: "", tag: "", description: "" };
}

export const SnapshotModuleRequest = {
  encode(message: SnapshotModuleRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.name !== "") {
      writer.uint32(10).string(message.name);
    }
    if (message.tag !== "") {
      writer.uint32(18).string(message.tag);
    }
    if (message.description !== "") {
      writer.uint32(26).string(message.description);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SnapshotModuleRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSnapshotModuleRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.name = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.tag = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.description = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SnapshotModuleRequest {
    return {
      name: isSet(object.name) ? globalThis.String(object.name) : "",
      tag: isSet(object.tag) ? globalThis.String(object.tag) : "",
      description: isSet(object.description) ? globalThis.String(object.description) : "",
    };
  },

  toJSON(message: SnapshotModuleRequest): unknown {
    const obj: any = {};
    if (message.name !== "") {
      obj.name = message.name;
    }
    if (message.tag !== "") {
      obj.tag = message.tag;
    }
    if (message.description !== "") {
      obj.description = message.description;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SnapshotModuleRequest>, I>>(base?: I): SnapshotModuleRequest {
    return SnapshotModuleRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SnapshotModuleRequest>, I>>(object: I): SnapshotModuleRequest {
    const message = createBaseSnapshotModuleRequest();
    message.name = object.name ?? "";
    message.tag = object.tag ?? "";
    message.description = object.description ?? "";
    return message;
  },
};

function createBaseSnapshotModuleResponse(): SnapshotModuleResponse {
  return { snapshotProjectVersionId: "" };
}

export const SnapshotModuleResponse = {
  encode(message: SnapshotModuleResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.snapshotProjectVersionId !== "") {
      writer.uint32(10).string(message.snapshotProjectVersionId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): SnapshotModuleResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseSnapshotModuleResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.snapshotProjectVersionId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): SnapshotModuleResponse {
    return {
      snapshotProjectVersionId: isSet(object.snapshotProjectVersionId)
        ? globalThis.String(object.snapshotProjectVersionId)
        : "",
    };
  },

  toJSON(message: SnapshotModuleResponse): unknown {
    const obj: any = {};
    if (message.snapshotProjectVersionId !== "") {
      obj.snapshotProjectVersionId = message.snapshotProjectVersionId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<SnapshotModuleResponse>, I>>(base?: I): SnapshotModuleResponse {
    return SnapshotModuleResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<SnapshotModuleResponse>, I>>(object: I): SnapshotModuleResponse {
    const message = createBaseSnapshotModuleResponse();
    message.snapshotProjectVersionId = object.snapshotProjectVersionId ?? "";
    return message;
  },
};

function createBasePushWorkerLogsRequest(): PushWorkerLogsRequest {
  return { logs: [] };
}

export const PushWorkerLogsRequest = {
  encode(message: PushWorkerLogsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.logs) {
      LogEntryData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PushWorkerLogsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePushWorkerLogsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.logs.push(LogEntryData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PushWorkerLogsRequest {
    return {
      logs: globalThis.Array.isArray(object?.logs) ? object.logs.map((e: any) => LogEntryData.fromJSON(e)) : [],
    };
  },

  toJSON(message: PushWorkerLogsRequest): unknown {
    const obj: any = {};
    if (message.logs?.length) {
      obj.logs = message.logs.map((e) => LogEntryData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PushWorkerLogsRequest>, I>>(base?: I): PushWorkerLogsRequest {
    return PushWorkerLogsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PushWorkerLogsRequest>, I>>(object: I): PushWorkerLogsRequest {
    const message = createBasePushWorkerLogsRequest();
    message.logs = object.logs?.map((e) => LogEntryData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseRunProxyStatementRequest(): RunProxyStatementRequest {
  return { statement: "", inputs: undefined, timeoutMs: 0, runId: "" };
}

export const RunProxyStatementRequest = {
  encode(message: RunProxyStatementRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.statement !== "") {
      writer.uint32(10).string(message.statement);
    }
    if (message.inputs !== undefined) {
      Struct.encode(Struct.wrap(message.inputs), writer.uint32(18).fork()).ldelim();
    }
    if (message.timeoutMs !== 0) {
      writer.uint32(24).int32(message.timeoutMs);
    }
    if (message.runId !== "") {
      writer.uint32(34).string(message.runId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RunProxyStatementRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRunProxyStatementRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.statement = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.inputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 3:
          if (tag !== 24) {
            break;
          }

          message.timeoutMs = reader.int32();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.runId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RunProxyStatementRequest {
    return {
      statement: isSet(object.statement) ? globalThis.String(object.statement) : "",
      inputs: isObject(object.inputs) ? object.inputs : undefined,
      timeoutMs: isSet(object.timeoutMs) ? globalThis.Number(object.timeoutMs) : 0,
      runId: isSet(object.runId) ? globalThis.String(object.runId) : "",
    };
  },

  toJSON(message: RunProxyStatementRequest): unknown {
    const obj: any = {};
    if (message.statement !== "") {
      obj.statement = message.statement;
    }
    if (message.inputs !== undefined) {
      obj.inputs = message.inputs;
    }
    if (message.timeoutMs !== 0) {
      obj.timeoutMs = Math.round(message.timeoutMs);
    }
    if (message.runId !== "") {
      obj.runId = message.runId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunProxyStatementRequest>, I>>(base?: I): RunProxyStatementRequest {
    return RunProxyStatementRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunProxyStatementRequest>, I>>(object: I): RunProxyStatementRequest {
    const message = createBaseRunProxyStatementRequest();
    message.statement = object.statement ?? "";
    message.inputs = object.inputs ?? undefined;
    message.timeoutMs = object.timeoutMs ?? 0;
    message.runId = object.runId ?? "";
    return message;
  },
};

function createBaseRunProxyStatementResponse(): RunProxyStatementResponse {
  return { outputs: undefined, error: undefined };
}

export const RunProxyStatementResponse = {
  encode(message: RunProxyStatementResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.outputs !== undefined) {
      Struct.encode(Struct.wrap(message.outputs), writer.uint32(10).fork()).ldelim();
    }
    if (message.error !== undefined) {
      Struct.encode(Struct.wrap(message.error), writer.uint32(18).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RunProxyStatementResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRunProxyStatementResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.outputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.error = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): RunProxyStatementResponse {
    return {
      outputs: isObject(object.outputs) ? object.outputs : undefined,
      error: isObject(object.error) ? object.error : undefined,
    };
  },

  toJSON(message: RunProxyStatementResponse): unknown {
    const obj: any = {};
    if (message.outputs !== undefined) {
      obj.outputs = message.outputs;
    }
    if (message.error !== undefined) {
      obj.error = message.error;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<RunProxyStatementResponse>, I>>(base?: I): RunProxyStatementResponse {
    return RunProxyStatementResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RunProxyStatementResponse>, I>>(object: I): RunProxyStatementResponse {
    const message = createBaseRunProxyStatementResponse();
    message.outputs = object.outputs ?? undefined;
    message.error = object.error ?? undefined;
    return message;
  },
};

function createBasePullWorkerRunsRequest(): PullWorkerRunsRequest {
  return { workerSetId: "", workerNodeId: "", workerProcessId: "" };
}

export const PullWorkerRunsRequest = {
  encode(message: PullWorkerRunsRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.workerSetId !== "") {
      writer.uint32(10).string(message.workerSetId);
    }
    if (message.workerNodeId !== "") {
      writer.uint32(18).string(message.workerNodeId);
    }
    if (message.workerProcessId !== "") {
      writer.uint32(26).string(message.workerProcessId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PullWorkerRunsRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePullWorkerRunsRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.workerSetId = reader.string();
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.workerNodeId = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.workerProcessId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PullWorkerRunsRequest {
    return {
      workerSetId: isSet(object.workerSetId) ? globalThis.String(object.workerSetId) : "",
      workerNodeId: isSet(object.workerNodeId) ? globalThis.String(object.workerNodeId) : "",
      workerProcessId: isSet(object.workerProcessId) ? globalThis.String(object.workerProcessId) : "",
    };
  },

  toJSON(message: PullWorkerRunsRequest): unknown {
    const obj: any = {};
    if (message.workerSetId !== "") {
      obj.workerSetId = message.workerSetId;
    }
    if (message.workerNodeId !== "") {
      obj.workerNodeId = message.workerNodeId;
    }
    if (message.workerProcessId !== "") {
      obj.workerProcessId = message.workerProcessId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PullWorkerRunsRequest>, I>>(base?: I): PullWorkerRunsRequest {
    return PullWorkerRunsRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PullWorkerRunsRequest>, I>>(object: I): PullWorkerRunsRequest {
    const message = createBasePullWorkerRunsRequest();
    message.workerSetId = object.workerSetId ?? "";
    message.workerNodeId = object.workerNodeId ?? "";
    message.workerProcessId = object.workerProcessId ?? "";
    return message;
  },
};

function createBasePullWorkerRunsResponse(): PullWorkerRunsResponse {
  return { runs: [] };
}

export const PullWorkerRunsResponse = {
  encode(message: PullWorkerRunsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    for (const v of message.runs) {
      RunData.encode(v!, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PullWorkerRunsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePullWorkerRunsResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.runs.push(RunData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): PullWorkerRunsResponse {
    return { runs: globalThis.Array.isArray(object?.runs) ? object.runs.map((e: any) => RunData.fromJSON(e)) : [] };
  },

  toJSON(message: PullWorkerRunsResponse): unknown {
    const obj: any = {};
    if (message.runs?.length) {
      obj.runs = message.runs.map((e) => RunData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<PullWorkerRunsResponse>, I>>(base?: I): PullWorkerRunsResponse {
    return PullWorkerRunsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PullWorkerRunsResponse>, I>>(object: I): PullWorkerRunsResponse {
    const message = createBasePullWorkerRunsResponse();
    message.runs = object.runs?.map((e) => RunData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseRestartWorkerRequest(): RestartWorkerRequest {
  return {};
}

export const RestartWorkerRequest = {
  encode(_: RestartWorkerRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): RestartWorkerRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseRestartWorkerRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(_: any): RestartWorkerRequest {
    return {};
  },

  toJSON(_: RestartWorkerRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<RestartWorkerRequest>, I>>(base?: I): RestartWorkerRequest {
    return RestartWorkerRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<RestartWorkerRequest>, I>>(_: I): RestartWorkerRequest {
    const message = createBaseRestartWorkerRequest();
    return message;
  },
};

function createBaseStartRunRequest(): StartRunRequest {
  return {
    triggerType: 0,
    triggerId: "",
    runId: "",
    sessionId: "",
    statement: "",
    scope: "",
    code: "",
    scheduledAt: undefined,
    inputs: undefined,
    block: false,
    keyed: false,
    keyedReturn: false,
    tags: [],
    rootValue: undefined,
    globalValue: undefined,
    accessLevel: 0,
  };
}

export const StartRunRequest = {
  encode(message: StartRunRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.triggerType !== 0) {
      writer.uint32(8).int32(message.triggerType);
    }
    if (message.triggerId !== "") {
      writer.uint32(18).string(message.triggerId);
    }
    if (message.runId !== "") {
      writer.uint32(26).string(message.runId);
    }
    if (message.sessionId !== "") {
      writer.uint32(34).string(message.sessionId);
    }
    if (message.statement !== "") {
      writer.uint32(42).string(message.statement);
    }
    if (message.scope !== "") {
      writer.uint32(50).string(message.scope);
    }
    if (message.code !== "") {
      writer.uint32(58).string(message.code);
    }
    if (message.scheduledAt !== undefined) {
      Timestamp.encode(toTimestamp(message.scheduledAt), writer.uint32(66).fork()).ldelim();
    }
    if (message.inputs !== undefined) {
      Struct.encode(Struct.wrap(message.inputs), writer.uint32(74).fork()).ldelim();
    }
    if (message.block === true) {
      writer.uint32(80).bool(message.block);
    }
    if (message.keyed === true) {
      writer.uint32(88).bool(message.keyed);
    }
    if (message.keyedReturn === true) {
      writer.uint32(96).bool(message.keyedReturn);
    }
    for (const v of message.tags) {
      writer.uint32(106).string(v!);
    }
    if (message.rootValue !== undefined) {
      Struct.encode(Struct.wrap(message.rootValue), writer.uint32(114).fork()).ldelim();
    }
    if (message.globalValue !== undefined) {
      Struct.encode(Struct.wrap(message.globalValue), writer.uint32(122).fork()).ldelim();
    }
    if (message.accessLevel !== 0) {
      writer.uint32(128).int32(message.accessLevel);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StartRunRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStartRunRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.triggerType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.triggerId = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.runId = reader.string();
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.sessionId = reader.string();
          continue;
        case 5:
          if (tag !== 42) {
            break;
          }

          message.statement = reader.string();
          continue;
        case 6:
          if (tag !== 50) {
            break;
          }

          message.scope = reader.string();
          continue;
        case 7:
          if (tag !== 58) {
            break;
          }

          message.code = reader.string();
          continue;
        case 8:
          if (tag !== 66) {
            break;
          }

          message.scheduledAt = fromTimestamp(Timestamp.decode(reader, reader.uint32()));
          continue;
        case 9:
          if (tag !== 74) {
            break;
          }

          message.inputs = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 10:
          if (tag !== 80) {
            break;
          }

          message.block = reader.bool();
          continue;
        case 11:
          if (tag !== 88) {
            break;
          }

          message.keyed = reader.bool();
          continue;
        case 12:
          if (tag !== 96) {
            break;
          }

          message.keyedReturn = reader.bool();
          continue;
        case 13:
          if (tag !== 106) {
            break;
          }

          message.tags.push(reader.string());
          continue;
        case 14:
          if (tag !== 114) {
            break;
          }

          message.rootValue = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 15:
          if (tag !== 122) {
            break;
          }

          message.globalValue = Struct.unwrap(Struct.decode(reader, reader.uint32()));
          continue;
        case 16:
          if (tag !== 128) {
            break;
          }

          message.accessLevel = reader.int32() as any;
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): StartRunRequest {
    return {
      triggerType: isSet(object.triggerType) ? triggerTypeFromJSON(object.triggerType) : 0,
      triggerId: isSet(object.triggerId) ? globalThis.String(object.triggerId) : "",
      runId: isSet(object.runId) ? globalThis.String(object.runId) : "",
      sessionId: isSet(object.sessionId) ? globalThis.String(object.sessionId) : "",
      statement: isSet(object.statement) ? globalThis.String(object.statement) : "",
      scope: isSet(object.scope) ? globalThis.String(object.scope) : "",
      code: isSet(object.code) ? globalThis.String(object.code) : "",
      scheduledAt: isSet(object.scheduledAt) ? fromJsonTimestamp(object.scheduledAt) : undefined,
      inputs: isObject(object.inputs) ? object.inputs : undefined,
      block: isSet(object.block) ? globalThis.Boolean(object.block) : false,
      keyed: isSet(object.keyed) ? globalThis.Boolean(object.keyed) : false,
      keyedReturn: isSet(object.keyedReturn) ? globalThis.Boolean(object.keyedReturn) : false,
      tags: globalThis.Array.isArray(object?.tags) ? object.tags.map((e: any) => globalThis.String(e)) : [],
      rootValue: isObject(object.rootValue) ? object.rootValue : undefined,
      globalValue: isObject(object.globalValue) ? object.globalValue : undefined,
      accessLevel: isSet(object.accessLevel) ? sessionAccessLevelFromJSON(object.accessLevel) : 0,
    };
  },

  toJSON(message: StartRunRequest): unknown {
    const obj: any = {};
    if (message.triggerType !== 0) {
      obj.triggerType = triggerTypeToJSON(message.triggerType);
    }
    if (message.triggerId !== "") {
      obj.triggerId = message.triggerId;
    }
    if (message.runId !== "") {
      obj.runId = message.runId;
    }
    if (message.sessionId !== "") {
      obj.sessionId = message.sessionId;
    }
    if (message.statement !== "") {
      obj.statement = message.statement;
    }
    if (message.scope !== "") {
      obj.scope = message.scope;
    }
    if (message.code !== "") {
      obj.code = message.code;
    }
    if (message.scheduledAt !== undefined) {
      obj.scheduledAt = message.scheduledAt.toISOString();
    }
    if (message.inputs !== undefined) {
      obj.inputs = message.inputs;
    }
    if (message.block === true) {
      obj.block = message.block;
    }
    if (message.keyed === true) {
      obj.keyed = message.keyed;
    }
    if (message.keyedReturn === true) {
      obj.keyedReturn = message.keyedReturn;
    }
    if (message.tags?.length) {
      obj.tags = message.tags;
    }
    if (message.rootValue !== undefined) {
      obj.rootValue = message.rootValue;
    }
    if (message.globalValue !== undefined) {
      obj.globalValue = message.globalValue;
    }
    if (message.accessLevel !== 0) {
      obj.accessLevel = sessionAccessLevelToJSON(message.accessLevel);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<StartRunRequest>, I>>(base?: I): StartRunRequest {
    return StartRunRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<StartRunRequest>, I>>(object: I): StartRunRequest {
    const message = createBaseStartRunRequest();
    message.triggerType = object.triggerType ?? 0;
    message.triggerId = object.triggerId ?? "";
    message.runId = object.runId ?? "";
    message.sessionId = object.sessionId ?? "";
    message.statement = object.statement ?? "";
    message.scope = object.scope ?? "";
    message.code = object.code ?? "";
    message.scheduledAt = object.scheduledAt ?? undefined;
    message.inputs = object.inputs ?? undefined;
    message.block = object.block ?? false;
    message.keyed = object.keyed ?? false;
    message.keyedReturn = object.keyedReturn ?? false;
    message.tags = object.tags?.map((e) => e) || [];
    message.rootValue = object.rootValue ?? undefined;
    message.globalValue = object.globalValue ?? undefined;
    message.accessLevel = object.accessLevel ?? 0;
    return message;
  },
};

function createBaseStartRunResponse(): StartRunResponse {
  return { errorType: 0, runId: "", run: undefined, logs: [] };
}

export const StartRunResponse = {
  encode(message: StartRunResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.errorType !== 0) {
      writer.uint32(8).int32(message.errorType);
    }
    if (message.runId !== "") {
      writer.uint32(18).string(message.runId);
    }
    if (message.run !== undefined) {
      RunData.encode(message.run, writer.uint32(26).fork()).ldelim();
    }
    for (const v of message.logs) {
      LogEntryData.encode(v!, writer.uint32(34).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): StartRunResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseStartRunResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.errorType = reader.int32() as any;
          continue;
        case 2:
          if (tag !== 18) {
            break;
          }

          message.runId = reader.string();
          continue;
        case 3:
          if (tag !== 26) {
            break;
          }

          message.run = RunData.decode(reader, reader.uint32());
          continue;
        case 4:
          if (tag !== 34) {
            break;
          }

          message.logs.push(LogEntryData.decode(reader, reader.uint32()));
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): StartRunResponse {
    return {
      errorType: isSet(object.errorType) ? startRunResponse_ErrorTypeFromJSON(object.errorType) : 0,
      runId: isSet(object.runId) ? globalThis.String(object.runId) : "",
      run: isSet(object.run) ? RunData.fromJSON(object.run) : undefined,
      logs: globalThis.Array.isArray(object?.logs) ? object.logs.map((e: any) => LogEntryData.fromJSON(e)) : [],
    };
  },

  toJSON(message: StartRunResponse): unknown {
    const obj: any = {};
    if (message.errorType !== 0) {
      obj.errorType = startRunResponse_ErrorTypeToJSON(message.errorType);
    }
    if (message.runId !== "") {
      obj.runId = message.runId;
    }
    if (message.run !== undefined) {
      obj.run = RunData.toJSON(message.run);
    }
    if (message.logs?.length) {
      obj.logs = message.logs.map((e) => LogEntryData.toJSON(e));
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<StartRunResponse>, I>>(base?: I): StartRunResponse {
    return StartRunResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<StartRunResponse>, I>>(object: I): StartRunResponse {
    const message = createBaseStartRunResponse();
    message.errorType = object.errorType ?? 0;
    message.runId = object.runId ?? "";
    message.run = object.run !== undefined && object.run !== null ? RunData.fromPartial(object.run) : undefined;
    message.logs = object.logs?.map((e) => LogEntryData.fromPartial(e)) || [];
    return message;
  },
};

function createBaseKillRunRequest(): KillRunRequest {
  return { runId: "" };
}

export const KillRunRequest = {
  encode(message: KillRunRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.runId !== "") {
      writer.uint32(10).string(message.runId);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): KillRunRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseKillRunRequest();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.runId = reader.string();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): KillRunRequest {
    return { runId: isSet(object.runId) ? globalThis.String(object.runId) : "" };
  },

  toJSON(message: KillRunRequest): unknown {
    const obj: any = {};
    if (message.runId !== "") {
      obj.runId = message.runId;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<KillRunRequest>, I>>(base?: I): KillRunRequest {
    return KillRunRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<KillRunRequest>, I>>(object: I): KillRunRequest {
    const message = createBaseKillRunRequest();
    message.runId = object.runId ?? "";
    return message;
  },
};

function createBaseKillRunResponse(): KillRunResponse {
  return { success: false };
}

export const KillRunResponse = {
  encode(message: KillRunResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.success === true) {
      writer.uint32(8).bool(message.success);
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): KillRunResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseKillRunResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 8) {
            break;
          }

          message.success = reader.bool();
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): KillRunResponse {
    return { success: isSet(object.success) ? globalThis.Boolean(object.success) : false };
  },

  toJSON(message: KillRunResponse): unknown {
    const obj: any = {};
    if (message.success === true) {
      obj.success = message.success;
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<KillRunResponse>, I>>(base?: I): KillRunResponse {
    return KillRunResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<KillRunResponse>, I>>(object: I): KillRunResponse {
    const message = createBaseKillRunResponse();
    message.success = object.success ?? false;
    return message;
  },
};

/**
 * Runtime supervisor orchestrating all the runtime hosts in its realm (not yet sharded).
 * Also orchestrates the corresponding worker sets/nodes.
 * Frontend connects to this directly.
 */
export interface RuntimeSupervisor {
  /** Notify supervisor of a new Bench. */
  DidCreateProject(request: DeepPartial<DidCreateProjectRequest>, metadata?: grpc.Metadata): Promise<Empty>;
  /** Get notifications for a client. (Not sure yet where this belongs.) */
  GetNotifications(
    request: DeepPartial<GetNotificationsRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetNotificationsResponse>;
  /** Get all changes to the worker sets for a Bench. */
  GetWorkerChanges(
    request: DeepPartial<GetWorkerChangesRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetWorkerChangesResponse>;
  /** Configure the worker set for a Bench. */
  ConfigureWorkerSet(
    request: DeepPartial<ConfigureWorkerSetRequest>,
    metadata?: grpc.Metadata
  ): Promise<ConfigureWorkerSetResponse>;
  /** Force restart the worker set for a Bench. */
  RestartWorkerSet(request: DeepPartial<RestartWorkerSetRequest>, metadata?: grpc.Metadata): Promise<Empty>;
  /** Gets the installed environment info from a worker set running a Bench. */
  GetWorkerImage(
    request: DeepPartial<GetWorkerImageRequest>,
    metadata?: grpc.Metadata
  ): Promise<GetWorkerImageResponse>;
  /** Ensure the worker set for a Bench is running. */
  PingWorkerSet(request: DeepPartial<PingWorkerSetRequest>, metadata?: grpc.Metadata): Promise<Empty>;
}

export class RuntimeSupervisorClientImpl implements RuntimeSupervisor {
  private readonly rpc: Rpc;

  constructor(rpc: Rpc) {
    this.rpc = rpc;
    this.DidCreateProject = this.DidCreateProject.bind(this);
    this.GetNotifications = this.GetNotifications.bind(this);
    this.GetWorkerChanges = this.GetWorkerChanges.bind(this);
    this.ConfigureWorkerSet = this.ConfigureWorkerSet.bind(this);
    this.RestartWorkerSet = this.RestartWorkerSet.bind(this);
    this.GetWorkerImage = this.GetWorkerImage.bind(this);
    this.PingWorkerSet = this.PingWorkerSet.bind(this);
  }

  DidCreateProject(request: DeepPartial<DidCreateProjectRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(
      RuntimeSupervisorDidCreateProjectDesc,
      DidCreateProjectRequest.fromPartial(request),
      metadata
    );
  }

  GetNotifications(
    request: DeepPartial<GetNotificationsRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetNotificationsResponse> {
    return this.rpc.invoke(
      RuntimeSupervisorGetNotificationsDesc,
      GetNotificationsRequest.fromPartial(request),
      metadata
    );
  }

  GetWorkerChanges(
    request: DeepPartial<GetWorkerChangesRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetWorkerChangesResponse> {
    return this.rpc.invoke(
      RuntimeSupervisorGetWorkerChangesDesc,
      GetWorkerChangesRequest.fromPartial(request),
      metadata
    );
  }

  ConfigureWorkerSet(
    request: DeepPartial<ConfigureWorkerSetRequest>,
    metadata?: grpc.Metadata
  ): Promise<ConfigureWorkerSetResponse> {
    return this.rpc.unary(
      RuntimeSupervisorConfigureWorkerSetDesc,
      ConfigureWorkerSetRequest.fromPartial(request),
      metadata
    );
  }

  RestartWorkerSet(request: DeepPartial<RestartWorkerSetRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(
      RuntimeSupervisorRestartWorkerSetDesc,
      RestartWorkerSetRequest.fromPartial(request),
      metadata
    );
  }

  GetWorkerImage(
    request: DeepPartial<GetWorkerImageRequest>,
    metadata?: grpc.Metadata
  ): Promise<GetWorkerImageResponse> {
    return this.rpc.unary(RuntimeSupervisorGetWorkerImageDesc, GetWorkerImageRequest.fromPartial(request), metadata);
  }

  PingWorkerSet(request: DeepPartial<PingWorkerSetRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(RuntimeSupervisorPingWorkerSetDesc, PingWorkerSetRequest.fromPartial(request), metadata);
  }
}

export const RuntimeSupervisorDesc = { serviceName: "RuntimeSupervisor" };

export const RuntimeSupervisorDidCreateProjectDesc: UnaryMethodDefinitionish = {
  methodName: "DidCreateProject",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return DidCreateProjectRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = Empty.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorGetNotificationsDesc: UnaryMethodDefinitionish = {
  methodName: "GetNotifications",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: true,
  requestType: {
    serializeBinary() {
      return GetNotificationsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetNotificationsResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorGetWorkerChangesDesc: UnaryMethodDefinitionish = {
  methodName: "GetWorkerChanges",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: true,
  requestType: {
    serializeBinary() {
      return GetWorkerChangesRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetWorkerChangesResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorConfigureWorkerSetDesc: UnaryMethodDefinitionish = {
  methodName: "ConfigureWorkerSet",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return ConfigureWorkerSetRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = ConfigureWorkerSetResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorRestartWorkerSetDesc: UnaryMethodDefinitionish = {
  methodName: "RestartWorkerSet",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return RestartWorkerSetRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = Empty.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorGetWorkerImageDesc: UnaryMethodDefinitionish = {
  methodName: "GetWorkerImage",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return GetWorkerImageRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetWorkerImageResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeSupervisorPingWorkerSetDesc: UnaryMethodDefinitionish = {
  methodName: "PingWorkerSet",
  service: RuntimeSupervisorDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return PingWorkerSetRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = Empty.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

/**
 * The Bench runtime host for a specific module.
 * Service is scoped to project_id/module_id.
 * Frontend connects to this directly.
 * Not sure yet how branching will work here (maybe 'virtual' modules on top of main/env modules).
 */
export interface RuntimeHost {
  /** Receive relevant outside-of-module changes to a Bench. */
  GetBenchChanges(
    request: DeepPartial<GetBenchChangesRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetBenchChangesResponse>;
  /** Receive any future edits to this module. */
  GetModuleEdits(
    request: DeepPartial<GetModuleEditsRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetModuleEditsResponse>;
  /** Reads the entire module tree. */
  ReadNodes(request: DeepPartial<ReadNodesRequest>, metadata?: grpc.Metadata): Promise<ReadNodesResponse>;
  /** Searches out-of-line nodes in the module. */
  SearchNodes(request: DeepPartial<SearchNodesRequest>, metadata?: grpc.Metadata): Promise<SearchNodesResponse>;
  /** Commits a set of edits to the module. */
  CommitEdits(request: DeepPartial<CommitEditsRequest>, metadata?: grpc.Metadata): Promise<CommitEditsResponse>;
  /** Get a signed URL to upload a blob. */
  UploadBlob(request: DeepPartial<UploadBlobRequest>, metadata?: grpc.Metadata): Promise<UploadBlobResponse>;
  /** Get a signed URL to download a blob. */
  DownloadBlob(request: DeepPartial<DownloadBlobRequest>, metadata?: grpc.Metadata): Promise<DownloadBlobResponse>;
  /** Reveal the deferred/secret 'value' of a secret. (Not sure if this should be bundled into something else?) */
  RevealSecret(request: DeepPartial<RevealSecretRequest>, metadata?: grpc.Metadata): Promise<RevealSecretResponse>;
  /** Paste specific inline nodes (and only those nodes) from this or another module. */
  PasteNodes(request: DeepPartial<PasteNodesRequest>, metadata?: grpc.Metadata): Promise<PasteNodesResponse>;
  /** Create a full snapshot of this Bench module (copy to a new Bench module as specified). */
  SnapshotModule(
    request: DeepPartial<SnapshotModuleRequest>,
    metadata?: grpc.Metadata
  ): Promise<SnapshotModuleResponse>;
  /** Forwards all matching logs received from the workers. */
  GetLogs(request: DeepPartial<GetLogsRequest>, metadata?: grpc.Metadata): Observable<GetLogsResponse>;
  /** Starts a run in an appropriate worker (same request/response as for WorkerNode). */
  StartRun(request: DeepPartial<StartRunRequest>, metadata?: grpc.Metadata): Promise<StartRunResponse>;
  /** Kills a run in the appropriate worker (same request/response as for WorkerNode). */
  KillRun(request: DeepPartial<KillRunRequest>, metadata?: grpc.Metadata): Promise<KillRunResponse>;
  /** Runs a well-known internal statement in the host with our credentials. */
  RunProxyStatement(
    request: DeepPartial<RunProxyStatementRequest>,
    metadata?: grpc.Metadata
  ): Promise<RunProxyStatementResponse>;
  /**
   * Pulls the runs a worker should run immediately after starting (scheduled).
   *  (Maybe merge this into ReadNodes with a query later? search_records too? Not sure.)
   */
  PullWorkerRuns(
    request: DeepPartial<PullWorkerRunsRequest>,
    metadata?: grpc.Metadata
  ): Promise<PullWorkerRunsResponse>;
  /** Pushes logs from a worker *that are already stored* to notify frontend users connected to this host. */
  PushWorkerLogs(request: DeepPartial<PushWorkerLogsRequest>, metadata?: grpc.Metadata): Promise<Empty>;
}

export class RuntimeHostClientImpl implements RuntimeHost {
  private readonly rpc: Rpc;

  constructor(rpc: Rpc) {
    this.rpc = rpc;
    this.GetBenchChanges = this.GetBenchChanges.bind(this);
    this.GetModuleEdits = this.GetModuleEdits.bind(this);
    this.ReadNodes = this.ReadNodes.bind(this);
    this.SearchNodes = this.SearchNodes.bind(this);
    this.CommitEdits = this.CommitEdits.bind(this);
    this.UploadBlob = this.UploadBlob.bind(this);
    this.DownloadBlob = this.DownloadBlob.bind(this);
    this.RevealSecret = this.RevealSecret.bind(this);
    this.PasteNodes = this.PasteNodes.bind(this);
    this.SnapshotModule = this.SnapshotModule.bind(this);
    this.GetLogs = this.GetLogs.bind(this);
    this.StartRun = this.StartRun.bind(this);
    this.KillRun = this.KillRun.bind(this);
    this.RunProxyStatement = this.RunProxyStatement.bind(this);
    this.PullWorkerRuns = this.PullWorkerRuns.bind(this);
    this.PushWorkerLogs = this.PushWorkerLogs.bind(this);
  }

  GetBenchChanges(
    request: DeepPartial<GetBenchChangesRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetBenchChangesResponse> {
    return this.rpc.invoke(RuntimeHostGetBenchChangesDesc, GetBenchChangesRequest.fromPartial(request), metadata);
  }

  GetModuleEdits(
    request: DeepPartial<GetModuleEditsRequest>,
    metadata?: grpc.Metadata
  ): Observable<GetModuleEditsResponse> {
    return this.rpc.invoke(RuntimeHostGetModuleEditsDesc, GetModuleEditsRequest.fromPartial(request), metadata);
  }

  ReadNodes(request: DeepPartial<ReadNodesRequest>, metadata?: grpc.Metadata): Promise<ReadNodesResponse> {
    return this.rpc.unary(RuntimeHostReadNodesDesc, ReadNodesRequest.fromPartial(request), metadata);
  }

  SearchNodes(request: DeepPartial<SearchNodesRequest>, metadata?: grpc.Metadata): Promise<SearchNodesResponse> {
    return this.rpc.unary(RuntimeHostSearchNodesDesc, SearchNodesRequest.fromPartial(request), metadata);
  }

  CommitEdits(request: DeepPartial<CommitEditsRequest>, metadata?: grpc.Metadata): Promise<CommitEditsResponse> {
    return this.rpc.unary(RuntimeHostCommitEditsDesc, CommitEditsRequest.fromPartial(request), metadata);
  }

  UploadBlob(request: DeepPartial<UploadBlobRequest>, metadata?: grpc.Metadata): Promise<UploadBlobResponse> {
    return this.rpc.unary(RuntimeHostUploadBlobDesc, UploadBlobRequest.fromPartial(request), metadata);
  }

  DownloadBlob(request: DeepPartial<DownloadBlobRequest>, metadata?: grpc.Metadata): Promise<DownloadBlobResponse> {
    return this.rpc.unary(RuntimeHostDownloadBlobDesc, DownloadBlobRequest.fromPartial(request), metadata);
  }

  RevealSecret(request: DeepPartial<RevealSecretRequest>, metadata?: grpc.Metadata): Promise<RevealSecretResponse> {
    return this.rpc.unary(RuntimeHostRevealSecretDesc, RevealSecretRequest.fromPartial(request), metadata);
  }

  PasteNodes(request: DeepPartial<PasteNodesRequest>, metadata?: grpc.Metadata): Promise<PasteNodesResponse> {
    return this.rpc.unary(RuntimeHostPasteNodesDesc, PasteNodesRequest.fromPartial(request), metadata);
  }

  SnapshotModule(
    request: DeepPartial<SnapshotModuleRequest>,
    metadata?: grpc.Metadata
  ): Promise<SnapshotModuleResponse> {
    return this.rpc.unary(RuntimeHostSnapshotModuleDesc, SnapshotModuleRequest.fromPartial(request), metadata);
  }

  GetLogs(request: DeepPartial<GetLogsRequest>, metadata?: grpc.Metadata): Observable<GetLogsResponse> {
    return this.rpc.invoke(RuntimeHostGetLogsDesc, GetLogsRequest.fromPartial(request), metadata);
  }

  StartRun(request: DeepPartial<StartRunRequest>, metadata?: grpc.Metadata): Promise<StartRunResponse> {
    return this.rpc.unary(RuntimeHostStartRunDesc, StartRunRequest.fromPartial(request), metadata);
  }

  KillRun(request: DeepPartial<KillRunRequest>, metadata?: grpc.Metadata): Promise<KillRunResponse> {
    return this.rpc.unary(RuntimeHostKillRunDesc, KillRunRequest.fromPartial(request), metadata);
  }

  RunProxyStatement(
    request: DeepPartial<RunProxyStatementRequest>,
    metadata?: grpc.Metadata
  ): Promise<RunProxyStatementResponse> {
    return this.rpc.unary(RuntimeHostRunProxyStatementDesc, RunProxyStatementRequest.fromPartial(request), metadata);
  }

  PullWorkerRuns(
    request: DeepPartial<PullWorkerRunsRequest>,
    metadata?: grpc.Metadata
  ): Promise<PullWorkerRunsResponse> {
    return this.rpc.unary(RuntimeHostPullWorkerRunsDesc, PullWorkerRunsRequest.fromPartial(request), metadata);
  }

  PushWorkerLogs(request: DeepPartial<PushWorkerLogsRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(RuntimeHostPushWorkerLogsDesc, PushWorkerLogsRequest.fromPartial(request), metadata);
  }
}

export const RuntimeHostDesc = { serviceName: "RuntimeHost" };

export const RuntimeHostGetBenchChangesDesc: UnaryMethodDefinitionish = {
  methodName: "GetBenchChanges",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: true,
  requestType: {
    serializeBinary() {
      return GetBenchChangesRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetBenchChangesResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostGetModuleEditsDesc: UnaryMethodDefinitionish = {
  methodName: "GetModuleEdits",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: true,
  requestType: {
    serializeBinary() {
      return GetModuleEditsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetModuleEditsResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostReadNodesDesc: UnaryMethodDefinitionish = {
  methodName: "ReadNodes",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return ReadNodesRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = ReadNodesResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostSearchNodesDesc: UnaryMethodDefinitionish = {
  methodName: "SearchNodes",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return SearchNodesRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = SearchNodesResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostCommitEditsDesc: UnaryMethodDefinitionish = {
  methodName: "CommitEdits",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return CommitEditsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = CommitEditsResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostUploadBlobDesc: UnaryMethodDefinitionish = {
  methodName: "UploadBlob",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return UploadBlobRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = UploadBlobResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostDownloadBlobDesc: UnaryMethodDefinitionish = {
  methodName: "DownloadBlob",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return DownloadBlobRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = DownloadBlobResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostRevealSecretDesc: UnaryMethodDefinitionish = {
  methodName: "RevealSecret",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return RevealSecretRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = RevealSecretResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostPasteNodesDesc: UnaryMethodDefinitionish = {
  methodName: "PasteNodes",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return PasteNodesRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = PasteNodesResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostSnapshotModuleDesc: UnaryMethodDefinitionish = {
  methodName: "SnapshotModule",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return SnapshotModuleRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = SnapshotModuleResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostGetLogsDesc: UnaryMethodDefinitionish = {
  methodName: "GetLogs",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: true,
  requestType: {
    serializeBinary() {
      return GetLogsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = GetLogsResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostStartRunDesc: UnaryMethodDefinitionish = {
  methodName: "StartRun",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return StartRunRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = StartRunResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostKillRunDesc: UnaryMethodDefinitionish = {
  methodName: "KillRun",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return KillRunRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = KillRunResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostRunProxyStatementDesc: UnaryMethodDefinitionish = {
  methodName: "RunProxyStatement",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return RunProxyStatementRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = RunProxyStatementResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostPullWorkerRunsDesc: UnaryMethodDefinitionish = {
  methodName: "PullWorkerRuns",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return PullWorkerRunsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = PullWorkerRunsResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const RuntimeHostPushWorkerLogsDesc: UnaryMethodDefinitionish = {
  methodName: "PushWorkerLogs",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return PushWorkerLogsRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = Empty.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

/**
 * The actual worker node running a Bench for a user.
 * Service is scoped to project_id/worker_set_id.
 * For internal connections only.
 */
export interface WorkerNode {
  /** Restart this worker immediately. */
  RestartWorker(request: DeepPartial<RestartWorkerRequest>, metadata?: grpc.Metadata): Promise<Empty>;
  /** Starts a run in this worker (if not already known). */
  StartRun(request: DeepPartial<StartRunRequest>, metadata?: grpc.Metadata): Promise<StartRunResponse>;
  /** Kills a run in this worker. */
  KillRun(request: DeepPartial<KillRunRequest>, metadata?: grpc.Metadata): Promise<KillRunResponse>;
}

export class WorkerNodeClientImpl implements WorkerNode {
  private readonly rpc: Rpc;

  constructor(rpc: Rpc) {
    this.rpc = rpc;
    this.RestartWorker = this.RestartWorker.bind(this);
    this.StartRun = this.StartRun.bind(this);
    this.KillRun = this.KillRun.bind(this);
  }

  RestartWorker(request: DeepPartial<RestartWorkerRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(WorkerNodeRestartWorkerDesc, RestartWorkerRequest.fromPartial(request), metadata);
  }

  StartRun(request: DeepPartial<StartRunRequest>, metadata?: grpc.Metadata): Promise<StartRunResponse> {
    return this.rpc.unary(WorkerNodeStartRunDesc, StartRunRequest.fromPartial(request), metadata);
  }

  KillRun(request: DeepPartial<KillRunRequest>, metadata?: grpc.Metadata): Promise<KillRunResponse> {
    return this.rpc.unary(WorkerNodeKillRunDesc, KillRunRequest.fromPartial(request), metadata);
  }
}

export const WorkerNodeDesc = { serviceName: "WorkerNode" };

export const WorkerNodeRestartWorkerDesc: UnaryMethodDefinitionish = {
  methodName: "RestartWorker",
  service: WorkerNodeDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return RestartWorkerRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = Empty.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const WorkerNodeStartRunDesc: UnaryMethodDefinitionish = {
  methodName: "StartRun",
  service: WorkerNodeDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return StartRunRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = StartRunResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

export const WorkerNodeKillRunDesc: UnaryMethodDefinitionish = {
  methodName: "KillRun",
  service: WorkerNodeDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return KillRunRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = KillRunResponse.decode(data);
      return {
        ...value,
        toObject() {
          return value;
        },
      };
    },
  } as any,
};

interface UnaryMethodDefinitionishR extends grpc.UnaryMethodDefinition<any, any> {
  requestStream: any;
  responseStream: any;
}

type UnaryMethodDefinitionish = UnaryMethodDefinitionishR;

interface Rpc {
  unary<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    request: any,
    metadata: grpc.Metadata | undefined
  ): Promise<any>;
  invoke<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    request: any,
    metadata: grpc.Metadata | undefined
  ): Observable<any>;
}

export class GrpcWebImpl {
  private host: string;
  private options: {
    transport?: grpc.TransportFactory;
    streamingTransport?: grpc.TransportFactory;
    debug?: boolean;
    metadata?: grpc.Metadata;
    upStreamRetryCodes?: number[];
  };

  constructor(
    host: string,
    options: {
      transport?: grpc.TransportFactory;
      streamingTransport?: grpc.TransportFactory;
      debug?: boolean;
      metadata?: grpc.Metadata;
      upStreamRetryCodes?: number[];
    }
  ) {
    this.host = host;
    this.options = options;
  }

  unary<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    _request: any,
    metadata: grpc.Metadata | undefined
  ): Promise<any> {
    const request = { ..._request, ...methodDesc.requestType };
    const maybeCombinedMetadata =
      metadata && this.options.metadata
        ? new BrowserHeaders({ ...this.options?.metadata.headersMap, ...metadata?.headersMap })
        : metadata ?? this.options.metadata;
    return new Promise((resolve, reject) => {
      grpc.unary(methodDesc, {
        request,
        host: this.host,
        metadata: maybeCombinedMetadata ?? {},
        ...(this.options.transport !== undefined ? { transport: this.options.transport } : {}),
        debug: this.options.debug ?? false,
        onEnd: function (response) {
          if (response.status === grpc.Code.OK) {
            resolve(response.message!.toObject());
          } else {
            const err = new GrpcWebError(response.statusMessage, response.status, response.trailers);
            reject(err);
          }
        },
      });
    });
  }

  invoke<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    _request: any,
    metadata: grpc.Metadata | undefined
  ): Observable<any> {
    const upStreamCodes = this.options.upStreamRetryCodes ?? [];
    const DEFAULT_TIMEOUT_TIME: number = 3_000;
    const request = { ..._request, ...methodDesc.requestType };
    const transport = this.options.streamingTransport ?? this.options.transport;
    const maybeCombinedMetadata =
      metadata && this.options.metadata
        ? new BrowserHeaders({ ...this.options?.metadata.headersMap, ...metadata?.headersMap })
        : metadata ?? this.options.metadata;
    return new Observable((observer) => {
      const upStream = () => {
        const client = grpc.invoke(methodDesc, {
          host: this.host,
          request,
          ...(transport !== undefined ? { transport } : {}),
          metadata: maybeCombinedMetadata ?? {},
          debug: this.options.debug ?? false,
          onMessage: (next) => observer.next(next),
          onEnd: (code: grpc.Code, message: string, trailers: grpc.Metadata) => {
            if (code === 0) {
              observer.complete();
            } else if (upStreamCodes.includes(code)) {
              setTimeout(upStream, DEFAULT_TIMEOUT_TIME);
            } else {
              const err = new Error(message) as any;
              err.code = code;
              err.metadata = trailers;
              observer.error(err);
            }
          },
        });
        observer.add(() => client.close());
      };
      upStream();
    }).pipe(share());
  }
}

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

export class GrpcWebError extends globalThis.Error {
  constructor(message: string, public code: grpc.Code, public metadata: grpc.Metadata) {
    super(message);
  }
}
