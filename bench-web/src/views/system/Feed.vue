<script lang="ts" setup>
import {
  BlockData,
  BoxData,
  EditType,
  ExpressionOp,
  IconData,
  LogData,
  LogProperty,
  NodeType,
  ObjectType,
  RunData,
  StepData,
  StructType,
  Timestamp,
  UserData,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { makeStruct, propertyReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { supergraph, useSearchConnection } from "@/system/connection";
import { makeExpression } from "@/system/expression";
import { ICON_BY_EDIT_TYPE, ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { toCamelName } from "@/system/lang";
import { canvas, pkgGraph } from "@/system/space";
import { TimeUpdateInterval, formatRelativeDate } from "@/utils/time";
import { DEFAULT_HEADER_HEIGHT } from "@/views/canvas";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

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

type LogEditItem = FeedItemBase & {
  kind: "log-edit";
  it: LogData;
  node: AnyNodeData | null;
  subject: UserData | RunData | null;
};

type LogChangeItem = FeedItemBase & {
  kind: "log-change";
  it: LogData[];
  nodes: AnyNodeData[];
  subject: UserData | RunData | null;
};

type RunItem = FeedItemBase & {
  kind: "run";
  it: RunData;
  node: BlockData | StepData | null;
};
type FeedItem = LogEditItem | LogChangeItem | RunItem;

// TODO :Incomplete!: store Feed query (and View-type-specific data) in view node
const nodeType = NodeType.LOG;
const { roots, graph, connection, page } = useSearchConnection(
  { name: `feed.${toCamelName(NodeType, nodeType).toLowerCase()}`, live: true },
  {
    scope: PACKAGE_SCOPE.value,
    // nocheckin: parameterize Feed search
    nodeType: nodeType,
    first: 10,
    sort: [
      makeExpression({
        op: ExpressionOp.DESCENDING,
        propertyPtr: propertyReference(nodeType as unknown as ObjectType, LogProperty.createdAt),
      }),
    ],
  },
);

const items = computed<FeedItem[]>(() => {
  const items: FeedItem[] = [];
  if (nodeType == NodeType.LOG) {
    for (const it of roots.value) {
      const item: LogEditItem = {
        kind: "log-edit",
        id: it.id,
        it,
        icon: ICON_BY_EDIT_TYPE[it.type as unknown as EditType] ?? ICON_BY_NODE_TYPE[NodeType.LOG]!,
        node: it.nodePtr != null ? supergraph.get(it.nodePtr) : null,
        subject: it.createdByPtr != null ? supergraph.get(it.createdByPtr) : null,
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
        <div class="flex flex-row items-center gap-x-1.5">
          <!-- nocheckin -->
          <!-- 'Title' -->
          <IconInline v-bind="item.icon" class="text-gray-700" />
          <!-- Log -->
          <template v-if="item.kind == 'log-edit'">
            <!-- Subject -->
            <button v-if="item.subject">{{ item.subject.name }}</button>
            <span v-else class="italic text-gray-900">System</span>
            <!-- Verb -->
            {{ toCamelName(EditType, item.it.type).toLowerCase() }}
            <!-- Object -->
            <button
              v-if="item.node"
              class="px-1 hover:bg-primary-100 hover:text-primary-900"
              @click="canvas.goToNode(item.node)"
            >
              <IconInline v-bind="getNodeIcon(item.node)" class="mr-1 text-gray-700" />
              <span class="underline decoration-gray-300 underline-offset-3">
                {{ (item.node as any)?.name ?? toCamelName(NodeType, item.node.metatype) }}
              </span>
            </button>
            <span v-else>
              <IconInline v-bind="ICON_BY_NODE_TYPE[item.it.nodePtr!.type]" class="text-gray-700" />
              <span class="ml-1 italic">Unavailable</span>
            </span>
          </template>
          <!-- Run -->
          <template v-else-if="item.kind == 'run'"> run! </template>
          <span v-else class="text-danger-500">???</span>
          <!-- Extra stuff -->
          <div class="ml-auto">
            <!-- Time -->
            <span class="text-gray-400">
              {{ formatRelativeDate(item.createdAt, { minUnit: "m", minValue: 1 }) }}
            </span>
            <!-- Actions -->
            <!-- ... -->
          </div>
        </div>
        <!-- Item body -->
        <!-- ...? -->
      </li>
    </ul>
    <!-- Loading -->
    <div v-else class="flex h-full min-h-20 w-full flex-col text-center align-middle">
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
