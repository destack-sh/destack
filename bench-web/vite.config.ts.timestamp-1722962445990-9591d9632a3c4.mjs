// vite.config.ts
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "file:///Users/florian/symbolx/bench/bench-web/node_modules/vite/dist/node/index.js";
import vue from "file:///Users/florian/symbolx/bench/bench-web/node_modules/@vitejs/plugin-vue/dist/index.mjs";
import vueJsx from "file:///Users/florian/symbolx/bench/bench-web/node_modules/@vitejs/plugin-vue-jsx/dist/index.mjs";
var __vite_injected_original_import_meta_url = "file:///Users/florian/symbolx/bench/bench-web/vite.config.ts";
var vite_config_default = defineConfig(({ mode }) => ({
  logLevel: "info",
  plugins: [vue(), vueJsx()],
  resolve: {
    preserveSymlinks: true,
    alias: {
      "@": fileURLToPath(new URL("./src", __vite_injected_original_import_meta_url))
    }
  },
  build: {
    chunkSizeWarningLimit: 8 * 1024,
    minify: mode !== "unminified",
    rollupOptions: {
      output: {
        entryFileNames: "assets/[name].js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name].[ext]",
        manualChunks: () => "everything.js"
      },
      onwarn(warning, warn) {
        if (warning.message.includes("but also statically imported by")) {
          return;
        }
        warn(warning);
      }
    }
  }
}));
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvVXNlcnMvZmxvcmlhbi9zeW1ib2x4L2JlbmNoL2JlbmNoLXdlYlwiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9maWxlbmFtZSA9IFwiL1VzZXJzL2Zsb3JpYW4vc3ltYm9seC9iZW5jaC9iZW5jaC13ZWIvdml0ZS5jb25maWcudHNcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfaW1wb3J0X21ldGFfdXJsID0gXCJmaWxlOi8vL1VzZXJzL2Zsb3JpYW4vc3ltYm9seC9iZW5jaC9iZW5jaC13ZWIvdml0ZS5jb25maWcudHNcIjtpbXBvcnQgeyBmaWxlVVJMVG9QYXRoLCBVUkwgfSBmcm9tIFwibm9kZTp1cmxcIjtcblxuaW1wb3J0IHsgZGVmaW5lQ29uZmlnIH0gZnJvbSBcInZpdGVcIjtcbmltcG9ydCB2dWUgZnJvbSBcIkB2aXRlanMvcGx1Z2luLXZ1ZVwiO1xuaW1wb3J0IHZ1ZUpzeCBmcm9tIFwiQHZpdGVqcy9wbHVnaW4tdnVlLWpzeFwiO1xuXG4vLyBodHRwczovL3ZpdGVqcy5kZXYvY29uZmlnL1xuZXhwb3J0IGRlZmF1bHQgZGVmaW5lQ29uZmlnKCh7IG1vZGUgfSkgPT4gKHtcbiAgbG9nTGV2ZWw6IFwiaW5mb1wiLFxuICBwbHVnaW5zOiBbdnVlKCksIHZ1ZUpzeCgpXSxcbiAgcmVzb2x2ZToge1xuICAgIHByZXNlcnZlU3ltbGlua3M6IHRydWUsXG4gICAgYWxpYXM6IHtcbiAgICAgIFwiQFwiOiBmaWxlVVJMVG9QYXRoKG5ldyBVUkwoXCIuL3NyY1wiLCBpbXBvcnQubWV0YS51cmwpKSxcbiAgICB9LFxuICB9LFxuICBidWlsZDoge1xuICAgIGNodW5rU2l6ZVdhcm5pbmdMaW1pdDogOCAqIDEwMjQsXG4gICAgbWluaWZ5OiBtb2RlICE9PSBcInVubWluaWZpZWRcIixcbiAgICByb2xsdXBPcHRpb25zOiB7XG4gICAgICBvdXRwdXQ6IHtcbiAgICAgICAgZW50cnlGaWxlTmFtZXM6ICdhc3NldHMvW25hbWVdLmpzJyxcbiAgICAgICAgY2h1bmtGaWxlTmFtZXM6ICdhc3NldHMvW25hbWVdLmpzJyxcbiAgICAgICAgYXNzZXRGaWxlTmFtZXM6ICdhc3NldHMvW25hbWVdLltleHRdJyxcbiAgICAgICAgbWFudWFsQ2h1bmtzOiAoKSA9PiAnZXZlcnl0aGluZy5qcycsXG4gICAgICB9LFxuICAgICAgb253YXJuKHdhcm5pbmcsIHdhcm4pIHtcbiAgICAgICAgaWYgKHdhcm5pbmcubWVzc2FnZS5pbmNsdWRlcygnYnV0IGFsc28gc3RhdGljYWxseSBpbXBvcnRlZCBieScpKSB7XG4gICAgICAgICAgcmV0dXJuO1xuICAgICAgICB9XG4gICAgICAgIHdhcm4od2FybmluZyk7XG4gICAgICB9LFxuICAgIH0sXG4gIH0sXG59KSk7Il0sCiAgIm1hcHBpbmdzIjogIjtBQUFvUyxTQUFTLGVBQWUsV0FBVztBQUV2VSxTQUFTLG9CQUFvQjtBQUM3QixPQUFPLFNBQVM7QUFDaEIsT0FBTyxZQUFZO0FBSmlLLElBQU0sMkNBQTJDO0FBT3JPLElBQU8sc0JBQVEsYUFBYSxDQUFDLEVBQUUsS0FBSyxPQUFPO0FBQUEsRUFDekMsVUFBVTtBQUFBLEVBQ1YsU0FBUyxDQUFDLElBQUksR0FBRyxPQUFPLENBQUM7QUFBQSxFQUN6QixTQUFTO0FBQUEsSUFDUCxrQkFBa0I7QUFBQSxJQUNsQixPQUFPO0FBQUEsTUFDTCxLQUFLLGNBQWMsSUFBSSxJQUFJLFNBQVMsd0NBQWUsQ0FBQztBQUFBLElBQ3REO0FBQUEsRUFDRjtBQUFBLEVBQ0EsT0FBTztBQUFBLElBQ0wsdUJBQXVCLElBQUk7QUFBQSxJQUMzQixRQUFRLFNBQVM7QUFBQSxJQUNqQixlQUFlO0FBQUEsTUFDYixRQUFRO0FBQUEsUUFDTixnQkFBZ0I7QUFBQSxRQUNoQixnQkFBZ0I7QUFBQSxRQUNoQixnQkFBZ0I7QUFBQSxRQUNoQixjQUFjLE1BQU07QUFBQSxNQUN0QjtBQUFBLE1BQ0EsT0FBTyxTQUFTLE1BQU07QUFDcEIsWUFBSSxRQUFRLFFBQVEsU0FBUyxpQ0FBaUMsR0FBRztBQUMvRDtBQUFBLFFBQ0Y7QUFDQSxhQUFLLE9BQU87QUFBQSxNQUNkO0FBQUEsSUFDRjtBQUFBLEVBQ0Y7QUFDRixFQUFFOyIsCiAgIm5hbWVzIjogW10KfQo=
