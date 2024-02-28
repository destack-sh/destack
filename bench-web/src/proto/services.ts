import { SupervisorClient } from "@/proto/wire";
import { SUPERVISOR_URL } from "@/utils/globals";
import { GrpcWebFetchTransport } from "@protobuf-ts/grpcweb-transport";

const supervisorTransport = new GrpcWebFetchTransport({
  baseUrl: SUPERVISOR_URL,
  fetchInit: { credentials: "include" },
});
const supervisor = new SupervisorClient(supervisorTransport);
