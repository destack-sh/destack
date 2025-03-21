import { makeScope, nodeReference } from "@/proto/wiring";
import { NodeType } from "@/proto/wire";

// builtin benches :Builtins
export const BENCH_BENCH_SLUG = "bench";
export const BENCH_BENCH_ID = "11111111-1111-1111-1111-000000000000";
export const BENCH_BENCH_PTR = nodeReference(NodeType.BENCH, BENCH_BENCH_ID, {
  benchId: BENCH_BENCH_ID,
});
export const BENCH_BUILTIN_PACKAGE_SLUG = "builtin";
export const BENCH_BUILTIN_PACKAGE_ID = "11111111-1111-1111-1111-000000000001";
export const BENCH_BUILTIN_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, BENCH_BUILTIN_PACKAGE_ID, {
  benchId: BENCH_BENCH_ID,
});
export const BENCH_BUILTIN_SCOPE = makeScope(BENCH_BUILTIN_PACKAGE_PTR);

export const SYSTEM_BENCH_SLUG = "system";
export const SYSTEM_BENCH_ID = "22222222-2222-2222-2222-000000000000";
export const SYSTEM_BENCH_PTR = nodeReference(NodeType.BENCH, SYSTEM_BENCH_ID, {
  benchId: SYSTEM_BENCH_ID,
});
export const SYSTEM_PACKAGE_SLUG = "system";
export const SYSTEM_PACKAGE_ID = "22222222-2222-2222-2222-000000000001";
export const SYSTEM_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, SYSTEM_PACKAGE_ID, {
  benchId: SYSTEM_BENCH_ID,
});
export const SYSTEM_SCOPE = makeScope(SYSTEM_PACKAGE_PTR);

// :Builtins
export const BENCH_BUILTIN_IDENTITY_ID = "c75cff62-0470-5cbf-9d73-ea8f0e995c0c";
export const BENCH_BUILTIN_IDENTITY_PTR = nodeReference(NodeType.IDENTITY, BENCH_BUILTIN_IDENTITY_ID, {
  benchId: BENCH_BENCH_ID,
});
