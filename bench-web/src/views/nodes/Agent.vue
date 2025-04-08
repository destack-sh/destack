<script lang="ts" setup>
import { packSubnode } from "@/language/core/node";
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { newChangeId } from "@/language/core/transaction";
import {
  BenchType,
  ClaimData,
  ClaimType,
  NodeMode,
  NodeReferenceData,
  NodeType,
  PickerVariant,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ACTION_SIZE, FLOW_GRID_STEP } from "@/ui/flow";
import { generateOrderKey } from "@/utils/fractional";
import Grid from "@/views/builtins/Grid.vue";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Claim from "@/views/nodes/Claim.vue";
import { useElementSize } from "@vueuse/core";
import { ComponentPublicInstance, computed, ref, toRef } from "vue";

const CLAIM_SIZE = { width: FLOW_GRID_STEP * 24, height: FLOW_GRID_STEP * 3 };
const GUTTER_WIDTH = 60;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedNodeConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const agentPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.AGENT>);
const preparedConnection = props.preparedConnection ?? useAutoConnection(agentPtr);
const { graph, connection } = preparedConnection;
const agent = graph.getRef(agentPtr);
const claims = graph.getChildrenRef(agentPtr, NodeType.CLAIM);

const containerRef = ref<HTMLElement | null>(null);
const containerSize = useElementSize(containerRef);
const headerRef = ref<InstanceType<typeof InlineHeader> | null>(null);
const gridRef = ref<ComponentPublicInstance<typeof Grid> | null>(null);

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

/** Creates a new Claim. */
function createClaim(claimIn: Partial<ClaimData>) {
  if (agent.value == null) throw new Error("no agent");
  const orderKey = generateOrderKey(claims.value[claims.value.length - 1]?.orderKey ?? null, null);

  // create
  let tx = connection.tx;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Create" } });
  }
  const claim = tx.create({
    metatype: NodeType.CLAIM,
    type: ClaimType.SHARED,
    mode: NodeMode.TEMPLATE,
    ...claimIn,
    parentPtr: agentPtr.value,
    packagePtr: agent.value.packagePtr,
    orderKey,
  });
  canvas.inspect({ node: claim });
  return claim;
}

function createClaimPopover(e: MouseEvent) {
  canvas.pushPopover({
    kind: "view",
    trigger: e.target as HTMLElement,
    reference: e.target as HTMLElement,
    component: ViewType.PICKER,
    title: "Add Claim",
    placement: "bottom-left",
    offset: "referenceWidth",
    props: {
      valueType: makeType({
        kind: TypeKind.NODE,
        constraint: makeTypeConstraint({ nodeTypes: [NodeType.COMPUTER] }),
      }),
      subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, {
        variant: PickerVariant.DROPDOWN_LARGE,
      }),
    },
    onApply: (value) => {
      createClaim({ type: ClaimType.SHARED, mode: NodeMode.TEMPLATE, targetTemplatePtr: value });
    },
  });
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div ref="containerRef">
    <InlineHeader
      ref="headerRef"
      :self="self"
      :node="agent"
      :connection="preparedConnection"
      :node-ptr="agentPtr"
      :prepared-connection="preparedConnection"
      :is-root="isRoot"
      :is-inline="isInline"
      :is-minimal="isMinimal"
      :width="containerSize.width?.value - GUTTER_WIDTH * 2"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    >
      <template #right="{ style }">
        <!-- Create Claim -->
        <button
          class="group/button rounded px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
          :class="style == 'block' ? 'opacity-0 group-hover/block:opacity-100 group-hover/header:opacity-100' : ''"
          @click.stop.prevent="createClaimPopover"
        >
          <i class="fas fa-plus mr-1.5 text-center" />
          <span class="">Claim</span>
        </button>
      </template>
    </InlineHeader>

    <!-- Claim grid -->
    <Grid
      :id="`${id}-grid`"
      ref="gridRef"
      :node-ptr="agentPtr"
      :prepared-connection="preparedConnection"
      :element-type="NodeType.CLAIM"
      :element-size="CLAIM_SIZE"
      :is-root="isRoot"
      :style="{
        marginLeft: isRoot ? `${GUTTER_WIDTH}px` : undefined,
        marginRight: isRoot ? `${GUTTER_WIDTH}px` : undefined,
      }"
      @create="createClaimPopover"
    >
      <template #element="{ element, elementRef, nodePtr, isDragging, elementSize, startDragging }">
        <Claim
          :id="element.id"
          :ref="elementRef"
          :node-ptr="nodePtr"
          :prepared-connection="preparedConnection"
          class="w-full transition-opacity duration-150"
          :class="{ 'opacity-50': isDragging }"
          :style="{ height: elementSize.height + 'px' }"
          data-suppress-drag="select"
          :draggable="true"
          @dragstart.stop="startDragging"
        />
      </template>
    </Grid>
  </div>
</template>
