* same-file.sp — HSPICE: Go to Definition within one file
* Try F12 on: buffer (X1 line), nch (M1 line)
* Outline shows .param / .model / .subckt

.option scale=1u
.param rload=1k cload=1p
.param vdd=1.8

.model nch nmos level=1 vto=0.7
.model pch pmos level=1 vto=-0.7

.subckt buffer in out vdd
R1 in mid rload
C1 mid 0 cload
Mp mid in vdd vdd pch
Mn mid in 0 0 nch
R2 mid out 1k
.ends buffer

* Instances — F12 on the model / subckt name at end of line
X1 a b vdd buffer
M1 d g s 0 nch
Rload out 0 rload

.dc vgs 0 vdd 0.1
.end
