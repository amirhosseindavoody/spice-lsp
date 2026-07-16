/** Demo netlists written by "SPICE LSP: Create Demo Folder". */

export const DEMO_FOLDER_NAME = "spice-lsp-demo";

export const DEMO_FILES: ReadonlyArray<{ relativePath: string; contents: string }> = [
  {
    relativePath: "README.md",
    contents: `# SPICE LSP demo

Sample netlists for trying **Go to Definition** (\`F12\`) and the **Outline** view.

## Same file — \`same-file.sp\`

1. Open \`same-file.sp\`.
2. Place the cursor on \`buffer\` in the \`X1\` line → **F12** jumps to \`.subckt buffer\`.
3. Place the cursor on \`nch\` in the \`M1\` line → **F12** jumps to \`.model nch\`.
4. Open **View → Outline** to see \`.param\`, \`.model\`, \`.subckt\`, and instances.

## Across files — \`top.sp\` + \`models.sp\`

1. Open \`top.sp\` (it \`.include\`s \`models.sp\`).
2. **F12** on \`nch\` or \`inverter\` jumps into \`models.sp\`.
3. **F12** on \`buffer\` (the \`Xbuf\` instance) also jumps to the shared \`.subckt\`.

## Tips

- **Find All References** (\`Shift+F12\`) lists uses of a subcircuit or model name in the open buffer.
- Switch dialect with **SPICE LSP: Set Dialect…** or the status-bar chip if hover docs look wrong.
`,
  },
  {
    relativePath: "same-file.sp",
    contents: `* same-file.sp — Go to Definition within one file
* Try F12 on: buffer (X1 line), nch (M1 line)
* Outline shows .param rload / cload, .model nch, .subckt buffer

.param rload=1k
.param cload=1p

.model nch nmos level=1 vto=0.7

.subckt buffer in out
R1 in mid rload
C1 mid 0 cload
R2 mid out 1k
.ends buffer

* Instances — F12 on the model / subckt name at end of line
X1 a b buffer
M1 d g s 0 nch
Rload out 0 rload
.end
`,
  },
  {
    relativePath: "models.sp",
    contents: `* models.sp — shared models and subcircuits (included by top.sp)

.model nch nmos level=1 vto=0.7
.model pch pmos level=1 vto=-0.7

.subckt inverter in out vdd
Mp out in vdd vdd pch
Mn out in 0 0 nch
.ends inverter

.subckt buffer in out vdd
Xi1 in mid vdd inverter
Xi2 mid out vdd inverter
.ends buffer
`,
  },
  {
    relativePath: "top.sp",
    contents: `* top.sp — Go to Definition across files via .include
* Try F12 on: nch, inverter, buffer → jumps into models.sp

.include 'models.sp'

.param vdd=1.8

Xinv in mid vdd inverter
Xbuf mid out vdd buffer
M1 out in 0 0 nch
.end
`,
  },
];
