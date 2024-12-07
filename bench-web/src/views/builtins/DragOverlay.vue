<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { toCamelName } from "@/language/const";
import { _setDragImage, activeDrag } from "@/ui/drag";
import { Casing, toCasing } from "@/utils/string";
</script>
<template>
  <!-- Drag image overlay -->
  <!-- Wrapper to ensure dragImageRef is always set -->
  <div :ref="(ref) => _setDragImage(ref as any)" class="absolute -top-[100px] left-20 py-1 pl-2">
    <div v-if="activeDrag" class="max-w-48 rounded border border-gray-200 bg-white px-2 py-1 text-gray-900">
      <!-- And wrapper to offset within the image to ensure the text isn't obscured by the cursor -->
      <div v-if="activeDrag.kind == 'node'" class="flex flex-row items-center">
        <IconInline v-bind="getNodeIcon(activeDrag.nodes[0])" class="mr-1 w-5 text-gray-700" />
        <span class="truncate">
          {{
            (activeDrag.nodes[0] as any).title ??
            (activeDrag.nodes[0] as any).name ??
            toCamelName(NodeType, activeDrag.node.nodeType)
          }}
        </span>
      </div>
      <div v-else>
        <!-- NOTE :Incomplete: drag selections -->
        <span class="text-gray-700">{{ toCasing(activeDrag.kind, Casing.CAMEL) }}</span>
      </div>
    </div>
  </div>
</template>
