import * as esbuild from "esbuild";
import * as fs from "fs";
import * as path from "path";

const watch = process.argv.includes("--watch");
const outfile = "out/extension.js";

function copyTerminateHelper() {
  // vscode-languageclient resolves terminateProcess.sh via __dirname next to the
  // bundled output after esbuild inlines the dependency.
  const terminateSrc = path.join(
    "node_modules",
    "vscode-languageclient",
    "lib",
    "node",
    "terminateProcess.sh",
  );
  fs.mkdirSync("out", { recursive: true });
  fs.copyFileSync(terminateSrc, path.join("out", "terminateProcess.sh"));
  fs.chmodSync(path.join("out", "terminateProcess.sh"), 0o755);
}

const ctx = await esbuild.context({
  entryPoints: ["src/extension.ts"],
  bundle: true,
  outfile,
  external: ["vscode"],
  format: "cjs",
  platform: "node",
  target: "node18",
  sourcemap: true,
  plugins: [
    {
      name: "copy-terminate-helper",
      setup(build) {
        build.onEnd((result) => {
          if (result.errors.length === 0) {
            copyTerminateHelper();
            console.log(`Bundled ${outfile}`);
          }
        });
      },
    },
  ],
});

if (watch) {
  await ctx.watch();
  console.log("Watching extension sources…");
} else {
  await ctx.rebuild();
  await ctx.dispose();
}
