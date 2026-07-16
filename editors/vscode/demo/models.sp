* models.sp — HSPICE shared models and subcircuits (included by top.sp)

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
