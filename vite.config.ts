import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, loadEnv } from "vite";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const devHost = env.BENTOLIFE_DEV_HOST || "127.0.0.1";
  const devPort = Number(env.BENTOLIFE_DEV_PORT || 1420);
  const previewPort = Number(env.BENTOLIFE_PREVIEW_PORT || 4173);

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        "@": path.resolve(dirname, "./src"),
      },
    },
    server: {
      host: devHost,
      port: devPort,
      strictPort: true,
    },
    preview: {
      host: devHost,
      port: previewPort,
      strictPort: true,
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            const normalized = id.replace(/\\/g, "/");
            if (normalized.includes("/node_modules/react") || normalized.includes("/node_modules/react-dom")) return "react";
            if (normalized.includes("/node_modules/@tauri-apps/")) return "tauri";
            if (normalized.includes("/node_modules/lucide-react")) return "icons";
            if (normalized.includes("/node_modules/@radix-ui/")) return "radix";
            if (
              normalized.includes("/node_modules/remark-") ||
              normalized.includes("/node_modules/unified/") ||
              normalized.includes("/node_modules/mdast") ||
              normalized.includes("/node_modules/micromark")
            ) {
              return "markdown";
            }
            if (normalized.includes("/src/components/architect/")) return "architect";
            if (normalized.includes("/src/components/settings/")) return "settings";
            if (normalized.includes("/src/components/widgets/") || normalized.includes("/src/services/widget")) return "widgets";
            if (normalized.includes("/src/components/modules/")) return "modules";
            return undefined;
          },
        },
      },
    },
    clearScreen: false,
    test: {
      environment: "jsdom",
      setupFiles: "./src/test/setup.ts",
      include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
      globals: true,
    },
  };
});
