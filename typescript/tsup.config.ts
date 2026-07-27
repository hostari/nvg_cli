import { defineConfig } from "tsup";

export default defineConfig([
  {
    entry: ["src/index.ts"],
    format: ["esm"],
    dts: true,
    clean: true,
    sourcemap: true,
  },
  {
    entry: ["src/cli.ts"],
    format: ["esm"],
    banner: { js: "#!/usr/bin/env node" },
    sourcemap: true,
  },
]);
