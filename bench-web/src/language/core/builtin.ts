import { makeScope, nodeReference } from "@/proto/wiring";
import { NodeType } from "@/proto/wire";

// builtin benches :Builtins
export const BENCH_SLUG = "bench";
export const BENCH_ID = "11111111-1111-1111-1111-000000000000";
export const BENCH_PTR = nodeReference(NodeType.BENCH, BENCH_ID, {
  benchId: BENCH_ID,
});
export const BENCH_BENCH_PACKAGE_SLUG = "bench";
export const BENCH_BENCH_PACKAGE_ID = "11111111-1111-1111-1111-000000000001";
export const BENCH_BENCH_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, BENCH_BENCH_PACKAGE_ID, {
  benchId: BENCH_ID,
});
export const BENCH_SCOPE = makeScope(BENCH_PTR);

export const SYSTEM_SLUG = "system";
export const SYSTEM_ID = "22222222-2222-2222-2222-000000000000";
export const SYSTEM_PTR = nodeReference(NodeType.BENCH, SYSTEM_ID, {
  benchId: SYSTEM_ID,
});
export const SYSTEM_SYSTEM_PACKAGE_SLUG = "system";
export const SYSTEM_SYSTEM_PACKAGE_ID = "22222222-2222-2222-2222-000000000001";
export const SYSTEM_SYSTEM_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, SYSTEM_SYSTEM_PACKAGE_ID, {
  benchId: SYSTEM_ID,
});
export const SYSTEM_SCOPE = makeScope(SYSTEM_SYSTEM_PACKAGE_PTR);

// :Builtins
export const BENCH_BUILTIN_AGENT_ID = "9269e60d-1a58-5366-907b-cf8390a2116a";
export const BENCH_BUILTIN_AGENT_PTR = nodeReference(NodeType.AGENT, BENCH_BUILTIN_AGENT_ID, {
  benchId: BENCH_ID,
});
