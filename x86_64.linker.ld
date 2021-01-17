SECTIONS {
    .debug_info 0 : {
        __my_old_debug_info_start = .;
        KEEP (*(.debug_info)) *(.debug_info)
        __my_old_debug_info_end = .;
    }
}
