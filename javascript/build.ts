// usage: bun run build
import { delimiter, dirname } from "node:path";

const rustc = Bun.spawnSync(["rustup", "which", "rustc"], {
	cwd: import.meta.dir,
	stderr: "ignore",
});
const rustPath = rustc.success
	? `${dirname(rustc.stdout.toString().trim())}${delimiter}${Bun.env.PATH}`
	: Bun.env.PATH;
async function build(target: "bundler" | "nodejs", outDir: string) {
	const build = Bun.spawn(
		[
			"wasm-pack",
			"build",
			"../wasm",
			"--target",
			target,
			"--release",
			"--out-dir",
			outDir,
		],
		{
			cwd: import.meta.dir,
			env: { ...Bun.env, PATH: rustPath },
			stderr: "inherit",
			stdout: "inherit",
		},
	);

	const exitCode = await build.exited;
	if (exitCode !== 0) {
		process.exit(exitCode);
	}
}

await build("bundler", "../javascript/generated");
await build("nodejs", "../javascript/generated/node");

// The package is ESM at its root, so mark the generated CommonJS output as
// CommonJS in its own package boundary.
await Bun.write(`${import.meta.dir}/generated/.gitignore`, "");
await Bun.write(`${import.meta.dir}/generated/node/.gitignore`, "");
await Bun.write(
	`${import.meta.dir}/generated/node/package.json`,
	JSON.stringify({ type: "commonjs" }, null, "\t") + "\n",
);
