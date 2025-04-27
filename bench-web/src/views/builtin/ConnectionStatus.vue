<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { Orientation, RegionContinent } from "@/proto/wire";
import { isDeveloperMode } from "@/system/client";
import { connections, hasPendingConnections } from "@/system/connection";
import { ScrollbarWidth } from "@/ui/layout";
import { GEOLOCATION } from "@/utils/geolocation";
import Popover from "@/views/builtin/Popover.vue";
import Scroll from "@/views/containers/Scroll.vue";
import { ref, type Ref } from "vue";

const expandedConnectionId: Ref<number | null> = ref(null);
</script>
<template>
  <!-- Connection -->
  <Popover placement="bottom" :reference-margin="8" :container-margin="4">
    <template #trigger="{ toggle }">
      <!-- Current status -->
      <button
        class="cursor-pointer rounded-sm border-2 px-1 py-1 transition-colors select-none"
        :class="
          connections.some((c) => c.isPaused.value || c.txBuffer.isPaused.value)
            ? 'border-warning-600'
            : 'border-transparent'
        "
        @click.stop="toggle"
      >
        <i
          class="fa-globe fas transition-colors duration-75"
          :class="
            !hasPendingConnections ? 'text-gray-700 hover:text-gray-900' : 'text-warning-600 hover:text-warning-500'
          "
        />
      </button>
    </template>
    <template #content="{ close }">
      <!-- Individual connections -->
      <!-- (will probably move this to a Connections View (maybe keep summary on hover)) -->
      <div v-outside.click.stop="close" class="z-50 rounded-sm border border-gray-200 bg-white text-gray-900">
        <!-- Header -->
        <div class="my-1 flex flex-row border-b border-gray-200 px-3 py-1">
          <span class="font-semibold">Connections</span>
          <div class="ml-auto">
            <span v-if="GEOLOCATION?.area" class="text-gray-400"
              >{{ toCamelName(RegionContinent, GEOLOCATION?.area) }} / {{ GEOLOCATION?.detail?.city }}</span
            >
          </div>
        </div>
        <!-- Body -->
        <Scroll
          id="scroll"
          :size="{ width: 480, height: 600 }"
          size-is-dynamic
          :orientation="Orientation.VERTICAL"
          :track-width="ScrollbarWidth.sm"
        >
          <ul class="my-1.5 flex min-w-[480px] flex-col gap-y-1 px-3">
            <li v-for="connection in connections" :key="connection.id" class="">
              <!-- Header -->
              <div class="flex flex-row py-1">
                <!-- Expand/collapse -->
                <button
                  v-if="isDeveloperMode"
                  class="mr-2 cursor-pointer text-gray-400 hover:text-gray-700"
                  @click="expandedConnectionId = expandedConnectionId == connection.id ? null : connection.id"
                >
                  <i
                    :class="[
                      'fa fa-chevron-right transition-transform duration-75',
                      expandedConnectionId == connection.id ? 'rotate-90' : '',
                    ]"
                  />
                </button>
                <!-- Metadata -->
                <span class="bg-secondary-100 text-secondary-900 h-fit rounded-sm px-2 font-semibold">
                  {{ connection.kind }}
                </span>
                <span class="ml-2 truncate font-semibold">{{ connection.name }}</span>
                <span class="ml-2 text-gray-500">#{{ connection.id }}</span>
                <!-- Status -->
                <span class="ml-auto flex flex-row pl-4 align-top">
                  <template v-if="isDeveloperMode">
                    <span class="mr-2" :class="connection.referenceCount > 0 ? '' : 'text-gray-500'">
                      r={{ connection.referenceCount }}
                    </span>
                    <span class="mr-2"> e={{ connection.epoch }} </span>
                  </template>
                  <!-- Connected (status) -->
                  <span class="rounded-sm px-1">
                    <i
                      class="fas"
                      :class="
                        connection.isConnected.value
                          ? 'fa-check text-success-600'
                          : 'fa-circle-exclamation text-warning-600'
                      "
                    />
                  </span>
                  <template v-if="isDeveloperMode">
                    <!-- Receive (status & toggle) -->
                    <button
                      class="cursor-pointer rounded-sm px-1 hover:bg-amber-200"
                      @click="connection.togglePaused()"
                    >
                      <i
                        :class="
                          connection.isConnecting.value
                            ? 'fas fa-spinner-third animate-spin text-gray-500'
                            : connection.isLive && !connection.isPaused.value
                              ? 'fas fa-down text-success-600'
                              : 'fas fa-down text-secondary-500'
                        "
                      />
                    </button>
                    <!-- Send (toggle) -->
                    <button
                      class="cursor-pointer rounded-sm px-1 hover:bg-amber-200"
                      @click="connection.txBuffer.togglePaused()"
                    >
                      <i
                        class="fas fa-up"
                        :class="connection.txBuffer.isPaused.value ? 'text-secondary-500' : 'text-success-600'"
                      />
                    </button>
                  </template>
                </span>
              </div>
              <div
                v-if="connection.id == expandedConnectionId"
                class="max-h-60 overflow-y-auto rounded-md border border-gray-200 bg-gray-50 p-1 py-0.5"
              >
                {{ connection.params }}
              </div>
            </li>
          </ul>
        </Scroll>
      </div>
    </template>
  </Popover>
</template>
