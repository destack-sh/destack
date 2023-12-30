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
  LogEntryData,
  ModuleTreeData,
  RunData,
  SessionAccessLevel,
  sessionAccessLevelFromJSON,
  sessionAccessLevelToJSON,
  TriggerType,
  triggerTypeFromJSON,
  triggerTypeToJSON,
} from "./bench";
import Long = require("long");

export const protobufPackage = "";

export enum ClientType {
  WEB = 0,
  WORKER = 1,
}

export function clientTypeFromJSON(object: any): ClientType {
  switch (object) {
    case 0:
    case "WEB":
      return ClientType.WEB;
    case 1:
    case "WORKER":
      return ClientType.WORKER;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ClientType");
  }
}

export function clientTypeToJSON(object: ClientType): string {
  switch (object) {
    case ClientType.WEB:
      return "WEB";
    case ClientType.WORKER:
      return "WORKER";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum ClientType");
  }
}

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

export enum StartRunErrorType {
  UNSPECIFIED = 0,
  UNAVAILABLE = 1,
  INVALID = 2,
  INTERNAL = 3,
  TIMEOUT = 4,
  RUNTIME = 5,
  DUPLICATE = 6,
}

export function startRunErrorTypeFromJSON(object: any): StartRunErrorType {
  switch (object) {
    case 0:
    case "START_RUN_ERROR_TYPE_UNSPECIFIED":
      return StartRunErrorType.UNSPECIFIED;
    case 1:
    case "START_RUN_ERROR_TYPE_UNAVAILABLE":
      return StartRunErrorType.UNAVAILABLE;
    case 2:
    case "START_RUN_ERROR_TYPE_INVALID":
      return StartRunErrorType.INVALID;
    case 3:
    case "START_RUN_ERROR_TYPE_INTERNAL":
      return StartRunErrorType.INTERNAL;
    case 4:
    case "START_RUN_ERROR_TYPE_TIMEOUT":
      return StartRunErrorType.TIMEOUT;
    case 5:
    case "START_RUN_ERROR_TYPE_RUNTIME":
      return StartRunErrorType.RUNTIME;
    case 6:
    case "START_RUN_ERROR_TYPE_DUPLICATE":
      return StartRunErrorType.DUPLICATE;
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StartRunErrorType");
  }
}

export function startRunErrorTypeToJSON(object: StartRunErrorType): string {
  switch (object) {
    case StartRunErrorType.UNSPECIFIED:
      return "START_RUN_ERROR_TYPE_UNSPECIFIED";
    case StartRunErrorType.UNAVAILABLE:
      return "START_RUN_ERROR_TYPE_UNAVAILABLE";
    case StartRunErrorType.INVALID:
      return "START_RUN_ERROR_TYPE_INVALID";
    case StartRunErrorType.INTERNAL:
      return "START_RUN_ERROR_TYPE_INTERNAL";
    case StartRunErrorType.TIMEOUT:
      return "START_RUN_ERROR_TYPE_TIMEOUT";
    case StartRunErrorType.RUNTIME:
      return "START_RUN_ERROR_TYPE_RUNTIME";
    case StartRunErrorType.DUPLICATE:
      return "START_RUN_ERROR_TYPE_DUPLICATE";
    default:
      throw new globalThis.Error("Unrecognized enum value " + object + " for enum StartRunErrorType");
  }
}

export interface ClientOrigin {
  clientType: ClientType;
  clientId: string;
  nonce: string;
}

export interface DidCreateProjectRequest {
  projectId: string;
}

export interface GetBenchChangesRequest {
  afterChangeMarker: number;
}

export interface GetBenchChangesResponse {
}

export interface GetModuleEditsRequest {
  afterEditMarker: number;
}

/** repeated EditData edits = 1; */
export interface GetModuleEditsResponse {
}

export interface GetLogsRequest {
}

export interface GetLogsResponse {
  logs: LogEntryData[];
}

export interface ReadModuleRequest {
}

export interface ReadModuleResponse {
  module: ModuleTreeData | undefined;
}

export interface RestartWorkerRequest {
}

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
  rootValues: { [key: string]: any } | undefined;
  globalValue: { [key: string]: any } | undefined;
  accessLevel: SessionAccessLevel;
}

export interface StartRunResponse {
  errorType: StartRunErrorType;
  runId: string;
  run: RunData | undefined;
  logs: LogEntryData[];
}

export interface KillRunRequest {
  runId: string;
}

export interface KillRunResponse {
  success: boolean;
}

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
  return {};
}

export const GetBenchChangesResponse = {
  encode(_: GetBenchChangesResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetBenchChangesResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetBenchChangesResponse();
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

  fromJSON(_: any): GetBenchChangesResponse {
    return {};
  },

  toJSON(_: GetBenchChangesResponse): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetBenchChangesResponse>, I>>(base?: I): GetBenchChangesResponse {
    return GetBenchChangesResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetBenchChangesResponse>, I>>(_: I): GetBenchChangesResponse {
    const message = createBaseGetBenchChangesResponse();
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
  return {};
}

export const GetModuleEditsResponse = {
  encode(_: GetModuleEditsResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): GetModuleEditsResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseGetModuleEditsResponse();
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

  fromJSON(_: any): GetModuleEditsResponse {
    return {};
  },

  toJSON(_: GetModuleEditsResponse): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<GetModuleEditsResponse>, I>>(base?: I): GetModuleEditsResponse {
    return GetModuleEditsResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<GetModuleEditsResponse>, I>>(_: I): GetModuleEditsResponse {
    const message = createBaseGetModuleEditsResponse();
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

function createBaseReadModuleRequest(): ReadModuleRequest {
  return {};
}

export const ReadModuleRequest = {
  encode(_: ReadModuleRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ReadModuleRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseReadModuleRequest();
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

  fromJSON(_: any): ReadModuleRequest {
    return {};
  },

  toJSON(_: ReadModuleRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<ReadModuleRequest>, I>>(base?: I): ReadModuleRequest {
    return ReadModuleRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ReadModuleRequest>, I>>(_: I): ReadModuleRequest {
    const message = createBaseReadModuleRequest();
    return message;
  },
};

function createBaseReadModuleResponse(): ReadModuleResponse {
  return { module: undefined };
}

export const ReadModuleResponse = {
  encode(message: ReadModuleResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    if (message.module !== undefined) {
      ModuleTreeData.encode(message.module, writer.uint32(10).fork()).ldelim();
    }
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): ReadModuleResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBaseReadModuleResponse();
    while (reader.pos < end) {
      const tag = reader.uint32();
      switch (tag >>> 3) {
        case 1:
          if (tag !== 10) {
            break;
          }

          message.module = ModuleTreeData.decode(reader, reader.uint32());
          continue;
      }
      if ((tag & 7) === 4 || tag === 0) {
        break;
      }
      reader.skipType(tag & 7);
    }
    return message;
  },

  fromJSON(object: any): ReadModuleResponse {
    return { module: isSet(object.module) ? ModuleTreeData.fromJSON(object.module) : undefined };
  },

  toJSON(message: ReadModuleResponse): unknown {
    const obj: any = {};
    if (message.module !== undefined) {
      obj.module = ModuleTreeData.toJSON(message.module);
    }
    return obj;
  },

  create<I extends Exact<DeepPartial<ReadModuleResponse>, I>>(base?: I): ReadModuleResponse {
    return ReadModuleResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<ReadModuleResponse>, I>>(object: I): ReadModuleResponse {
    const message = createBaseReadModuleResponse();
    message.module = (object.module !== undefined && object.module !== null)
      ? ModuleTreeData.fromPartial(object.module)
      : undefined;
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
    rootValues: undefined,
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
    if (message.rootValues !== undefined) {
      Struct.encode(Struct.wrap(message.rootValues), writer.uint32(114).fork()).ldelim();
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

          message.rootValues = Struct.unwrap(Struct.decode(reader, reader.uint32()));
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
      rootValues: isObject(object.rootValues) ? object.rootValues : undefined,
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
    if (message.rootValues !== undefined) {
      obj.rootValues = message.rootValues;
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
    message.rootValues = object.rootValues ?? undefined;
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
      errorType: isSet(object.errorType) ? startRunErrorTypeFromJSON(object.errorType) : 0,
      runId: isSet(object.runId) ? globalThis.String(object.runId) : "",
      run: isSet(object.run) ? RunData.fromJSON(object.run) : undefined,
      logs: globalThis.Array.isArray(object?.logs) ? object.logs.map((e: any) => LogEntryData.fromJSON(e)) : [],
    };
  },

  toJSON(message: StartRunResponse): unknown {
    const obj: any = {};
    if (message.errorType !== 0) {
      obj.errorType = startRunErrorTypeToJSON(message.errorType);
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
    message.run = (object.run !== undefined && object.run !== null) ? RunData.fromPartial(object.run) : undefined;
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
 */
export interface RuntimeSupervisor {
  /** Notify supervisor of a new Bench. */
  DidCreateProject(request: DeepPartial<DidCreateProjectRequest>, metadata?: grpc.Metadata): Promise<Empty>;
}

export class RuntimeSupervisorClientImpl implements RuntimeSupervisor {
  private readonly rpc: Rpc;

  constructor(rpc: Rpc) {
    this.rpc = rpc;
    this.DidCreateProject = this.DidCreateProject.bind(this);
  }

  DidCreateProject(request: DeepPartial<DidCreateProjectRequest>, metadata?: grpc.Metadata): Promise<Empty> {
    return this.rpc.unary(
      RuntimeSupervisorDidCreateProjectDesc,
      DidCreateProjectRequest.fromPartial(request),
      metadata,
    );
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

/**
 * The Bench runtime host for a specific module.
 * Service is scoped to project_id/module_id.
 * Not sure yet how branching will work here (maybe 'virtual' modules on top of main/env modules).
 */
export interface RuntimeHost {
  /**
   * subscriptions
   * Receive larger-than-module changes to a Bench.
   */
  GetBenchChanges(
    request: DeepPartial<GetBenchChangesRequest>,
    metadata?: grpc.Metadata,
  ): Observable<GetBenchChangesResponse>;
  /** Receive any future edits to this module. */
  GetModuleEdits(
    request: DeepPartial<GetModuleEditsRequest>,
    metadata?: grpc.Metadata,
  ): Observable<GetModuleEditsResponse>;
  /** Forwards all matching logs received from the workers. */
  GetLogs(request: DeepPartial<GetLogsRequest>, metadata?: grpc.Metadata): Observable<GetLogsResponse>;
  /** Reads the entire module tree. */
  ReadModule(request: DeepPartial<ReadModuleRequest>, metadata?: grpc.Metadata): Promise<ReadModuleResponse>;
}

export class RuntimeHostClientImpl implements RuntimeHost {
  private readonly rpc: Rpc;

  constructor(rpc: Rpc) {
    this.rpc = rpc;
    this.GetBenchChanges = this.GetBenchChanges.bind(this);
    this.GetModuleEdits = this.GetModuleEdits.bind(this);
    this.GetLogs = this.GetLogs.bind(this);
    this.ReadModule = this.ReadModule.bind(this);
  }

  GetBenchChanges(
    request: DeepPartial<GetBenchChangesRequest>,
    metadata?: grpc.Metadata,
  ): Observable<GetBenchChangesResponse> {
    return this.rpc.invoke(RuntimeHostGetBenchChangesDesc, GetBenchChangesRequest.fromPartial(request), metadata);
  }

  GetModuleEdits(
    request: DeepPartial<GetModuleEditsRequest>,
    metadata?: grpc.Metadata,
  ): Observable<GetModuleEditsResponse> {
    return this.rpc.invoke(RuntimeHostGetModuleEditsDesc, GetModuleEditsRequest.fromPartial(request), metadata);
  }

  GetLogs(request: DeepPartial<GetLogsRequest>, metadata?: grpc.Metadata): Observable<GetLogsResponse> {
    return this.rpc.invoke(RuntimeHostGetLogsDesc, GetLogsRequest.fromPartial(request), metadata);
  }

  ReadModule(request: DeepPartial<ReadModuleRequest>, metadata?: grpc.Metadata): Promise<ReadModuleResponse> {
    return this.rpc.unary(RuntimeHostReadModuleDesc, ReadModuleRequest.fromPartial(request), metadata);
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

export const RuntimeHostReadModuleDesc: UnaryMethodDefinitionish = {
  methodName: "ReadModule",
  service: RuntimeHostDesc,
  requestStream: false,
  responseStream: false,
  requestType: {
    serializeBinary() {
      return ReadModuleRequest.encode(this).finish();
    },
  } as any,
  responseType: {
    deserializeBinary(data: Uint8Array) {
      const value = ReadModuleResponse.decode(data);
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
    metadata: grpc.Metadata | undefined,
  ): Promise<any>;
  invoke<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    request: any,
    metadata: grpc.Metadata | undefined,
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
    },
  ) {
    this.host = host;
    this.options = options;
  }

  unary<T extends UnaryMethodDefinitionish>(
    methodDesc: T,
    _request: any,
    metadata: grpc.Metadata | undefined,
  ): Promise<any> {
    const request = { ..._request, ...methodDesc.requestType };
    const maybeCombinedMetadata = metadata && this.options.metadata
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
    metadata: grpc.Metadata | undefined,
  ): Observable<any> {
    const upStreamCodes = this.options.upStreamRetryCodes ?? [];
    const DEFAULT_TIMEOUT_TIME: number = 3_000;
    const request = { ..._request, ...methodDesc.requestType };
    const transport = this.options.streamingTransport ?? this.options.transport;
    const maybeCombinedMetadata = metadata && this.options.metadata
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

export type DeepPartial<T> = T extends Builtin ? T
  : T extends globalThis.Array<infer U> ? globalThis.Array<DeepPartial<U>>
  : T extends ReadonlyArray<infer U> ? ReadonlyArray<DeepPartial<U>>
  : T extends { $case: string } ? { [K in keyof Omit<T, "$case">]?: DeepPartial<T[K]> } & { $case: T["$case"] }
  : T extends {} ? { [K in keyof T]?: DeepPartial<T[K]> }
  : Partial<T>;

type KeysOfUnion<T> = T extends T ? keyof T : never;
export type Exact<P, I extends P> = P extends Builtin ? P
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
