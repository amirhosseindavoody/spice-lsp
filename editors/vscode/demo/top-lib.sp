* top-lib.sp — HSPICE: Go to Definition across files via .lib
* Try F12 on: nch_tt → jumps into corners.lib (TT section)

.lib 'corners.lib' TT

.option scale=1u
.param vdd=1.8

M1 out in 0 0 nch_tt
M2 out in vdd vdd pch_tt

.dc vin 0 vdd 0.05
.end
