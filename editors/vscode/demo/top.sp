* top.sp — HSPICE: Go to Definition across files via .include
* Try F12 on: nch, inverter, buffer → jumps into models.sp

.include 'models.sp'

.option scale=1u
.param vdd=1.8

Xinv in mid vdd inverter
Xbuf mid out vdd buffer
M1 out in 0 0 nch

.dc vin 0 vdd 0.05
.end
