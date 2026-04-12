.text
.p2align 2

.globl _relative_dispatch_alpha
_relative_dispatch_alpha:
    add w0, w0, #11
    ret

.globl _relative_dispatch_beta
_relative_dispatch_beta:
    add w0, w0, #21
    ret

.globl _relative_exported_gamma
_relative_exported_gamma:
    add w0, w0, #31
    ret

.globl _relative_dispatch_second_slot
_relative_dispatch_second_slot:
    adrp x8, _relative_targets@PAGE
    add x8, x8, _relative_targets@PAGEOFF
    mov w9, #1
    ldrsw x10, [x8, x9, lsl #2]
    add x10, x8, x10
    blr x10
    ret

.globl _relative_load_export_target
_relative_load_export_target:
    adrp x8, _relative_targets@PAGE
    add x8, x8, _relative_targets@PAGEOFF
    mov w9, #0
    ldrsw x10, [x8, x9, lsl #2]
    add x0, x8, x10
    ret

.globl _relative_load_function_target
_relative_load_function_target:
    adrp x8, _relative_targets@PAGE
    add x8, x8, _relative_targets@PAGEOFF
    mov w9, #1
    ldrsw x10, [x8, x9, lsl #2]
    add x0, x8, x10
    ret

.section __DATA_CONST,__const
.p2align 2
_relative_targets:
    .long _relative_exported_gamma - _relative_targets
    .long _relative_dispatch_beta - _relative_targets

.text
.globl _main
_main:
    mov w0, #0
    ret
