<script lang="ts" setup>
import {
  ViewData,
  NodeType,
  BoxData,
  IconData,
  RunStatus,
  LogData,
  RunData,
  UserData,
  type AnyNodeData,
  BlockData,
  StepData,
  EditType,
  Timestamp,
  StructType,
  ExpressionOp,
  ObjectType,
  LogProperty,
} from "@/proto/wire";
import { makeStruct, propertyReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, pkgGraph } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useSearchConnection } from "@/system/connection";
import { DEFAULT_HEADER_HEIGHT } from "@/views/canvas";
import { toCamelName } from "@/system/lang";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { ICON_BY_EDIT_TYPE, ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { DateTime } from "luxon";
import { formatRelativeDate, tsToDt } from "@/utils/time";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 800;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size?: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "variant" | "isInput" | "isInline" | "valueType" | "valuePacked">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

type FeedItemBase = {
  kind: string;
  id: string;
  icon: IconData;
  createdAt: Timestamp;
};

type LogItem = FeedItemBase & {
  kind: "log";
  it: LogData;
  node: AnyNodeData | null;
  subject: UserData | RunData | null;
};

type RunItem = FeedItemBase & {
  kind: "run";
  it: RunData;
  node: BlockData | StepData | null;
};
type FeedItem = LogItem | RunItem;

// TODO :Incomplete!: store Feed query (and View-type-specific data) in view node
const nodeType = NodeType.LOG;
const { roots, graph, connection, page } = useSearchConnection(
  { name: `feed.${toCamelName(NodeType, nodeType).toLowerCase()}` },
  {
    nodeType: nodeType, // nocheckin: parameterize Feed search
    first: 10,
    sort: [
      makeStruct({
        metatype: StructType.EXPRESSION,
        op: ExpressionOp.DESCENDING,
        propertyPtr: propertyReference(nodeType as unknown as ObjectType, LogProperty.createdAt),
        clauses: [],
      }),
    ],
  },
);

const items = computed<FeedItem[]>(() => {
  const items: FeedItem[] = [];
  if (nodeType == NodeType.LOG) {
    for (const it of roots.value) {
      const item: LogItem = {
        kind: "log",
        id: it.id,
        it,
        icon: ICON_BY_EDIT_TYPE[it.type as unknown as EditType] ?? ICON_BY_NODE_TYPE[NodeType.LOG]!,
        node: it.nodePtr != null ? pkgGraph.get(it.nodePtr) : null,
        subject: null,
        createdAt: it.createdAt!,
      };
      items.push(item);
    }
  } else {
    throw new Error(`unsupported feed node type: ${nodeType}`);
  }
  return items;
});
const itemRefs: Ref<Record<string, HTMLElement>> = ref({});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- Header -->
    <div class="group mx-auto py-2">nocheckin feed filter pills</div>
    <!-- Content -->
    <ul v-if="connection.isConnected.value" class="mt-1 flex flex-col gap-y-0.5 py-1">
      <!-- Feed item -->
      <li
        v-for="item in items"
        :key="item.id"
        :ref="(ref: any) => (ref != null ? (itemRefs[item.id] = ref) : delete itemRefs[item.id])"
        class="py-1"
      >
        <!-- Item header -->
        <div class="flex flex-row items-center gap-x-1">
          <!-- nocheckin -->
          <!-- 'Title' -->
          <IconInline v-bind="item.icon" class="text-gray-700" />
          <template v-if="item.kind == 'log'">
            <!-- Subject -->
            <button v-if="item.subject"></button>
            <span v-else class="text-gray-900">System</span>
            <!-- Verb -->
            {{ toCamelName(EditType, item.it.type).toLowerCase() }}
            <!-- Object -->
            <button v-if="item.node">
              <IconInline v-bind="getNodeIcon(item.node)" class="mr-1 text-gray-700" />
              {{ (item.node as any)?.name ?? toCamelName(NodeType, item.node.metatype) }}
            </button>
            <span v-else>???</span>
          </template>
          <template v-else-if="item.kind == 'run'"> run! </template>
          <span v-else class="text-danger-500">???</span>
          <!-- Extra stuff -->
          <div class="ml-auto">
            <!-- Time -->
            <span>{{ formatRelativeDate(item.createdAt) }}</span>
            <!-- Actions -->
            <!-- ... -->
          </div>
        </div>
        <!-- Item body -->
        <!-- ...? -->
      </li>
    </ul>
    <!-- Loading -->
    <div v-else class="flex h-full w-full flex-col text-center align-middle">
      <Transition
        enter-from-class="opacity-0"
        enter-active-class="transition-opacity duration-200"
        enter-to-class="opacity-100"
        appear
      >
        <i class="fas fa-spinner-third animate-spin text-gray-400" />
      </Transition>
    </div>
  </div>
</template>
