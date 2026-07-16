# SPICE LSP demo (HSPICE)

Sample **HSPICE-style** netlists for trying **Go to Definition** (`F12`), **Find All References** (`Shift+F12`), and the **Outline** view.

The Create Demo Folder command sets `spiceLsp.dialect` to **hspice** so hover matches these files.

## Same file — `same-file.sp`

1. Open `same-file.sp`.
2. Place the cursor on `buffer` in the `X1` line → **F12** jumps to `.subckt buffer`.
3. Place the cursor on `nch` in the `M1` line → **F12** jumps to `.model nch`.
4. Open **View → Outline** to see `.param`, `.model`, `.subckt`, and instances.

## Across files via `.include` — `top.sp` + `models.sp`

1. Open `top.sp` (it `.include`s `models.sp`).
2. **F12** on `nch` or `inverter` jumps into `models.sp`.
3. **F12** on `buffer` (the `Xbuf` instance) also jumps to the shared `.subckt`.

## Across files via HSPICE `.lib` — `top-lib.sp` + `corners.lib`

1. Open `top-lib.sp` (it calls `.lib 'corners.lib' TT`).
2. **F12** on `nch_tt` jumps to the model inside the `TT` section of `corners.lib`.

## Tips

- **Find All References** (`Shift+F12`) lists uses of a subcircuit or model name in the open buffer.
- If hover looks wrong, run **SPICE LSP: Set Dialect…** and pick **HSPICE**.
