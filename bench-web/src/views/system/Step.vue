<script lang="ts" setup>
import { NAME_CONSTRAINT, toCamelName, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, FIELD_CONTEXT_ACTIONS, makeTypeInfo, type TypeIdentity } from "@/language/field";
import {
  FLOW_GRID_STEP_Y,
  FLOW_PORT_SIZE,
  portIdEquals,
  STEP_CONTEXT_ACTIONS,
  STEP_HEADER_HEIGHT,
  useFlowContext,
  type Port,
} from "@/language/flow";
import { cloneNode } from "@/language/node";
import {
  BenchType,
  FieldData,
  NodeType,
  Orientation,
  PortSide,
  PortType,
  StepType,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import {
  startDragging,
  startDraggingIfAllowed,
  useMultiDropZone,
  type DraggedContent,
  type MultiAnchor,
} from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { menuActionsLike, pushPopover, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { getColorHex } from "@/ui/style";
import type { TooltipInfo } from "@/ui/tooltip";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import Text from "@/views/content/Text.vue";
import Field from "@/views/system/Field.vue";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const stepPtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.STEP>);
const flowCtx = useFlowContext();
const stepState = flowCtx.stepsStates.value[stepPtr.value.id!]; // must exist
const { step, fields, nodePtr, node, nodeFields, ports } = stepState;

const nameRef: Ref<HTMLInputElement | null> = ref(null);
const containerRef: Ref<HTMLElement | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const bodySize = useElementSize(bodyRef, undefined, { box: "border-box" });
const paddingHeight = computed(
  () => FLOW_GRID_STEP_Y - ((bodySize.height.value + STEP_HEADER_HEIGHT) % FLOW_GRID_STEP_Y),
);

function toPortId(port: Port): string {
  return `${port.side}-${port.idx}`;
}

//
// Interaction
//

const incomingZoneRef: Ref<HTMLElement | null> = ref(null);
const outgoingZoneRef: Ref<HTMLElement | null> = ref(null);
const portRefs: Ref<Record<string, HTMLElement>> = ref({});

// dragging
// NOTE: port dragging only supports field ports for now
function allowDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent) {
  if (dragged.kind != "node") return false;
  const node = flowCtx.graph.get(dragged.node);
  return isNode(node, NodeType.FIELD) || (isNode(node, NodeType.BLOCK) && TYPE_BLOCK_TYPES.includes(node.type));
}
function onDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind != "node") return;
  const node = flowCtx.graph.getOrError(dragged.node);
  const port =
    ports.value.incoming.find((p) => toPortId(p) == targetId) ??
    ports.value.outgoing.find((p) => toPortId(p) == targetId) ??
    null;
  if (port == null) return; // there should always be a port since every active port zone has >=1 port
  // nocheckin: create/move fields on drop (also for create button)
}
const { activeDropZone: activeIncomingDropZone } = useMultiDropZone({
  name: "step.incoming",
  container: incomingZoneRef,
  targets: portRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});
const { activeDropZone: activeOutgoingDropZone } = useMultiDropZone({
  name: "step.outgoing",
  container: outgoingZoneRef,
  targets: portRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});
const activeDropZone = computed(() => activeIncomingDropZone.value ?? activeOutgoingDropZone.value);
const activeDropZoneSide = computed(() => {
  if (activeIncomingDropZone.value != null) return PortSide.INCOMING;
  else if (activeOutgoingDropZone.value != null) return PortSide.OUTGOING;
  else return null;
});

// actions (some of these actions also only work for field ports)
const getFieldFromContext = (ctx: ActionContext | undefined): { field: FieldData | null } => {
  let field = fields.value.find((f) => f.id == ctx?.triggerNode?.id);
  if (field == null) field = nodeFields.value.find((f) => f.id == ctx?.triggerNode?.id); // related fields
  if (field == null) field = flowCtx.fields.value.find((f) => f.id == ctx?.triggerNode?.id); // related fields
  return { field: field ?? null };
};
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"step"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => nameRef.value!.focus());
    },
  },
  "common.create.above": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(flowCtx.tx, flowCtx.graph, { anchor: "before", target: field });
  },
  "common.create.below": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(flowCtx.tx, flowCtx.graph, { anchor: "after", target: field });
  },
  "common.edit.duplicate": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    const duplicate = cloneNode(flowCtx.tx, flowCtx.graph, field, { includeChildren: true });
  },
  "common.edit.delete": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    flowCtx.tx.delete(field);
  },
  "common.edit.archive": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    flowCtx.tx.archive(field);
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="step"
    ref="containerRef"
    class="group/step rounded border bg-white transition-colors duration-75"
    :class="[stepPtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200 hover:border-gray-300']"
    @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step', step: step! })"
  >
    <!-- Header -->
    <div
      ref="headerRef"
      class="flex w-full flex-row items-center border-b border-gray-200 px-2 transition-colors duration-75 group-hover/step:border-gray-300"
      :style="{ height: STEP_HEADER_HEIGHT + 'px' }"
    >
      <!-- Icon/Name -->
      <div class="flex-shrink-0">
        <IconInline
          v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
          v-menu="
            (): PopoverInfoIn => ({
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: step!.icon, isInput: true },
              onApply: (newIcon) => flowCtx.tx.update(step!, { icon: newIcon }),
            })
          "
          v-bind="getNodeIcon(step)"
          class="w-5 rounded py-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
        />
        <input
          ref="nameRef"
          type="text"
          class="ml-0.5 w-fit min-w-fit max-w-fit rounded border-0 px-1 font-medium text-gray-700 outline-none ring-0 transition-colors duration-75 hover:bg-gray-100 focus:ring-0"
          spellcheck="false"
          data-suppress-drag="true"
          :value="step.name"
          :size="step.name.length + 3"
          v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
          @input="
            guardNativeInput(NAME_CONSTRAINT, $event, step!.name, (newValue) =>
              flowCtx.tx.update(step!, { name: newValue }, { debounce: 'long' }),
            )
          "
        />
      </div>
      <!-- Controls -->
      <div class="ml-auto flex flex-row pl-2 pr-0.5">
        <!-- Add field -->
        <button
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
          @click="
            (e) =>
              pushPopover({
                trigger: (e.target as HTMLElement).closest('button')!,
                reference: (e.target as HTMLElement).closest('button')!,
                info: {
                  component: ViewType.PICKER,
                  placement: 'bottom-left',
                  offset: 'referenceWidth',
                  props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
                  onApply: (typeInfo: TypeIdentity) => {
                    createField(flowCtx.tx, flowCtx.graph, { anchor: 'inside', target: step!, field: typeInfo });
                  },
                },
              })
          "
        >
          <i class="fas fa-plus w-5 text-center" />
        </button>
        <!-- Menu -->
        <button
          v-menu="
            (): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-left',
              offset: 'referenceWidth',
              items: menuActionsLike(STEP_CONTEXT_ACTIONS, { context: { triggerNode: stepPtr } }),
            })
          "
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
        >
          <i class="fas fa-ellipsis-v w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div
      ref="bodyRef"
      class="relative"
      :style="{
        // ensure ports are aligned with grid (offset by half a step to connect lines :FlowGrid)
        paddingTop: FLOW_GRID_STEP_Y - (STEP_HEADER_HEIGHT % FLOW_GRID_STEP_Y) - FLOW_GRID_STEP_Y / 2 + 'px',
      }"
    >
      <!-- Ports -->
      <div
        class="relative flex w-full flex-row"
        :style="{
          height: FLOW_GRID_STEP_Y * Math.max(ports.incoming.length, ports.outgoing.length) + 'px',
        }"
      >
        <!-- Incoming/outgoing port zones -->
        <div
          v-for="{ side, ports: sidePorts } in [
            { side: PortSide.INCOMING, ports: ports.incoming },
            { side: PortSide.OUTGOING, ports: ports.outgoing },
          ]"
          :ref="(ref: any) => (side == PortSide.INCOMING ? (incomingZoneRef = ref) : (outgoingZoneRef = ref))"
          :key="side"
          class="flex-1 rounded"
          :class="[activeDropZoneSide == side ? 'outline outline-2 outline-primary-900' : '']"
        >
          <!-- Ports -->
          <div
            v-for="port in sidePorts"
            :ref="(ref?: any) => (ref != null ? (portRefs[toPortId(port)] = ref) : delete portRefs[toPortId(port)])"
            :key="port.idx"
            class="absolute flex w-20 items-center"
            :class="[port.side == PortSide.INCOMING ? 'justify-start' : 'justify-end']"
            :style="{
              height: FLOW_GRID_STEP_Y + 'px',
              left: port.side == PortSide.INCOMING ? '0px' : undefined,
              right: port.side == PortSide.OUTGOING ? '0px' : undefined,
              top: port.idx * FLOW_GRID_STEP_Y - 1 + 'px', // NOTE :Cleanup: why do the ports need -1px offset?
            }"
          >
            <!-- Actual 'port' -->
            <button
              class="absolute cursor-crosshair rounded-sm border bg-white transition-colors duration-75 focus:outline-none"
              :class="[
                stepPtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-gray-200 hover:bg-gray-100 ',
                'hover:border-primary-900 hover:bg-gray-100',
              ]"
              :style="{
                height: FLOW_PORT_SIZE + 'px',
                width: FLOW_PORT_SIZE + 'px',
                left: port.side == PortSide.INCOMING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
                right: port.side == PortSide.OUTGOING ? -FLOW_PORT_SIZE / 2 + 'px' : undefined,
                top: FLOW_GRID_STEP_Y / 2 - FLOW_PORT_SIZE / 2 + 'px',
              }"
              data-suppress-drag="true"
              @mousedown="(e) => flowCtx.startDragging(e, { kind: 'step-port', step: step!, port })"
              @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step-port', step: step!, port })"
            >
              <!-- Fill with port colors if connected -->
              <!-- NOTE :Performance: steps/ports/pipes querying should be centralized/cached better -->
              <div
                v-if="flowCtx.getPipesAtPort(port).length > 0"
                class="absolute rounded-sm bg-gray-600"
                :style="{
                  backgroundColor: flowCtx.getPipeColorHex(flowCtx.getPipesAtPort(port)[0]),
                  width: FLOW_PORT_SIZE - 4 + 'px',
                  height: FLOW_PORT_SIZE - 4 + 'px',
                  top: 1 + 'px',
                  left: 1 + 'px',
                }"
              />
            </button>
            <!-- Port content -->
            <div class="cursor-default" data-suppress-drag="true">
              <div v-if="port.type == PortType.RUN" class="px-2.5">
                <i class="fas fa-play w-5 text-center text-gray-700" />
              </div>
              <div v-else-if="port.type == PortType.FIELD" class="px-1">
                <Field
                  v-contextmenu="
                    (context: PopoverContext): PopoverInfo => ({
                      kind: 'menu',
                      placement: 'bottom-right',
                      items: menuActionsLike(FIELD_CONTEXT_ACTIONS, {
                        context: { ...context, triggerNode: port.field },
                      }),
                    })
                  "
                  class="data-[dragging=true]:bg-gray-100 data-[dragging=true]:opacity-50"
                  :node-ptr="toNodeRefOneOf(port.field!)"
                  :variant="Variant.STEALTH"
                  :orientation="
                    port.side == PortSide.INCOMING ? Orientation.HORIZONTAL : Orientation.HORIZONTAL_REVERSED
                  "
                  :draggable="true"
                  @dragstart.stop="(e: DragEvent) => startDragging(e, flowCtx.graph, port.field!)"
                />
              </div>
              <!-- Drop indicator -->
              <div
                v-if="activeDropZone?.targetId == toPortId(port)"
                class="absolute left-0 z-10 h-1 w-full rounded-sm bg-primary-900"
              />
            </div>
          </div>
        </div>
      </div>

      <!-- Content -->
      <div
        v-if="[StepType.TEXT, StepType.CODE].includes(step.type)"
        class="mt-1 border-t border-gray-200 pt-1 transition-colors duration-75 group-hover/step:border-gray-300"
      >
        <Text
          v-if="step.type == StepType.TEXT"
          class="px-3"
          is-input
          :variant="Variant.STEALTH"
          :model-value="step.text"
          @update:model-value="(newText) => flowCtx.tx.update(step!, { text: newText }, { debounce: 'long' })"
        />
        <Code
          v-else-if="step.type == StepType.CODE"
          class=""
          is-input
          :variant="Variant.STEALTH"
          :model-value="step.code"
          @update:model-value="(newCode) => flowCtx.tx.update(step!, { code: newCode }, { debounce: 'long' })"
        />
      </div>
    </div>

    <!-- Padding (to ensure height is a multiple of the grid) -->
    <div class="w-full" :style="{ height: paddingHeight + 'px' }" />
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- should never be rendered by containing flow -->
    <span>???</span>
  </div>
</template>
