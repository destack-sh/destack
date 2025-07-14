import { Signal, useComputed, useSignal } from "@preact/signals-react";
import { Node, Query, QueryConnection, activeSession } from "destack";

// nocheckin: reactive TS graphs & querying
// basic reactive keys (in (Reactive)Graphs):
//  - get: snapshot_id + node_id
//  - get_children: snapshot_id + node_id + [node_type]

export function useQuery<T extends Node = Node>(
  query: Signal<Query<T>>,
): { connection: Signal<QueryConnection<T> | null>; nodes: Signal<readonly T[]> } {
  const session = activeSession();
  const connection: Signal<QueryConnection<T> | null> = useSignal(null);
  const nodes = useComputed(() => connection.value?.toList() ?? []);

  // useSignalEffect(() => {
  //   query.value.execute().then((c) => {
  //     connection.value = c;
  //     console.log("query.execute", query.value.name);
  //   });
  // });

  return { connection, nodes };
}
