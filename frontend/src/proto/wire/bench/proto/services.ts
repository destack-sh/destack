/* eslint-disable */
import * as _m0 from "protobufjs/minimal";
import { ModuleTreeData } from "./bench";

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

export interface ClientOrigin {
  clientType: ClientType;
  clientId: string;
  nonce: string;
}

export interface PingRuntimeRequest {}

export interface PingRuntimeResponse {}

export interface ReadModuleRequest {}

export interface ReadModuleResponse {
  module: ModuleTreeData | undefined;
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

function createBasePingRuntimeRequest(): PingRuntimeRequest {
  return {};
}

export const PingRuntimeRequest = {
  encode(_: PingRuntimeRequest, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingRuntimeRequest {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingRuntimeRequest();
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

  fromJSON(_: any): PingRuntimeRequest {
    return {};
  },

  toJSON(_: PingRuntimeRequest): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<PingRuntimeRequest>, I>>(base?: I): PingRuntimeRequest {
    return PingRuntimeRequest.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PingRuntimeRequest>, I>>(_: I): PingRuntimeRequest {
    const message = createBasePingRuntimeRequest();
    return message;
  },
};

function createBasePingRuntimeResponse(): PingRuntimeResponse {
  return {};
}

export const PingRuntimeResponse = {
  encode(_: PingRuntimeResponse, writer: _m0.Writer = _m0.Writer.create()): _m0.Writer {
    return writer;
  },

  decode(input: _m0.Reader | Uint8Array, length?: number): PingRuntimeResponse {
    const reader = input instanceof _m0.Reader ? input : _m0.Reader.create(input);
    let end = length === undefined ? reader.len : reader.pos + length;
    const message = createBasePingRuntimeResponse();
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

  fromJSON(_: any): PingRuntimeResponse {
    return {};
  },

  toJSON(_: PingRuntimeResponse): unknown {
    const obj: any = {};
    return obj;
  },

  create<I extends Exact<DeepPartial<PingRuntimeResponse>, I>>(base?: I): PingRuntimeResponse {
    return PingRuntimeResponse.fromPartial(base ?? ({} as any));
  },
  fromPartial<I extends Exact<DeepPartial<PingRuntimeResponse>, I>>(_: I): PingRuntimeResponse {
    const message = createBasePingRuntimeResponse();
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
    message.module =
      object.module !== undefined && object.module !== null ? ModuleTreeData.fromPartial(object.module) : undefined;
    return message;
  },
};

export interface RuntimeHost {
  PingRuntime(request: PingRuntimeRequest): Promise<PingRuntimeResponse>;
  /** read/write module */
  ReadModule(request: ReadModuleRequest): Promise<ReadModuleResponse>;
}

export const RuntimeHostServiceName = "RuntimeHost";
export class RuntimeHostClientImpl implements RuntimeHost {
  private readonly rpc: Rpc;
  private readonly service: string;
  constructor(rpc: Rpc, opts?: { service?: string }) {
    this.service = opts?.service || RuntimeHostServiceName;
    this.rpc = rpc;
    this.PingRuntime = this.PingRuntime.bind(this);
    this.ReadModule = this.ReadModule.bind(this);
  }
  PingRuntime(request: PingRuntimeRequest): Promise<PingRuntimeResponse> {
    const data = PingRuntimeRequest.encode(request).finish();
    const promise = this.rpc.request(this.service, "PingRuntime", data);
    return promise.then((data) => PingRuntimeResponse.decode(_m0.Reader.create(data)));
  }

  ReadModule(request: ReadModuleRequest): Promise<ReadModuleResponse> {
    const data = ReadModuleRequest.encode(request).finish();
    const promise = this.rpc.request(this.service, "ReadModule", data);
    return promise.then((data) => ReadModuleResponse.decode(_m0.Reader.create(data)));
  }
}

/**
 * worker management
 *  rpc RestartWorker(RestartWorkerRequest) returns (RestartWorkerResponse);
 */
export interface WorkerNode {}

export const WorkerNodeServiceName = "WorkerNode";
export class WorkerNodeClientImpl implements WorkerNode {
  private readonly rpc: Rpc;
  private readonly service: string;
  constructor(rpc: Rpc, opts?: { service?: string }) {
    this.service = opts?.service || WorkerNodeServiceName;
    this.rpc = rpc;
  }
}

interface Rpc {
  request(service: string, method: string, data: Uint8Array): Promise<Uint8Array>;
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

function isSet(value: any): boolean {
  return value !== null && value !== undefined;
}
