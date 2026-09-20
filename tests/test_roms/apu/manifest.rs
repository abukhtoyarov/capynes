apu_manifest!(
    blargg: [
        (apu_reset_4015_cleared, "./tests/test_roms/apu/apu_reset/4015_cleared.nes", true),
        (apu_reset_irq_flag_cleared, "./tests/test_roms/apu/apu_reset/irq_flag_cleared.nes", true),
        (apu_test_len_ctr, "./tests/test_roms/apu/apu_test/rom_singles/1-len_ctr.nes", false),
        (apu_test_irq_flag, "./tests/test_roms/apu/apu_test/rom_singles/3-irq_flag.nes", false),
        (apu_test_jitter, "./tests/test_roms/apu/apu_test/rom_singles/4-jitter.nes", false),
        (apu_reset_len_ctrs_enabled, "./tests/test_roms/apu/apu_reset/len_ctrs_enabled.nes", true),
        (apu_test_len_table, "./tests/test_roms/apu/apu_test/rom_singles/2-len_table.nes", false),
        (apu_test_len_timing, "./tests/test_roms/apu/apu_test/rom_singles/5-len_timing.nes", false),
        (
            apu_test_irq_flag_timing,
            "./tests/test_roms/apu/apu_test/rom_singles/6-irq_flag_timing.nes",
            false
        ),
        (apu_test_dmc_basics, "./tests/test_roms/apu/apu_test/rom_singles/7-dmc_basics.nes", false),
        (apu_mixer_dmc, "./tests/test_roms/apu/apu_mixer/dmc.nes", false),
        (apu_mixer_noise, "./tests/test_roms/apu/apu_mixer/noise.nes", false),
        (apu_mixer_square, "./tests/test_roms/apu/apu_mixer/square.nes", false),
        (apu_mixer_triangle, "./tests/test_roms/apu/apu_mixer/triangle.nes", false),
        (apu_reset_works_immediately, "./tests/test_roms/apu/apu_reset/works_immediately.nes", true),
        (apu_reset_4017_written, "./tests/test_roms/apu/apu_reset/4017_written.nes", true),
        (apu_reset_4017_timing, "./tests/test_roms/apu/apu_reset/4017_timing.nes", true)
    ],
    screen_result_code: [
        (blargg_apu_2005_len_ctr, "./tests/test_roms/apu/blargg_apu_2005.07.30/01.len_ctr.nes"),
        (blargg_apu_2005_len_table, "./tests/test_roms/apu/blargg_apu_2005.07.30/02.len_table.nes"),
        (blargg_apu_2005_irq_flag, "./tests/test_roms/apu/blargg_apu_2005.07.30/03.irq_flag.nes"),
        (blargg_apu_2005_clock_jitter, "./tests/test_roms/apu/blargg_apu_2005.07.30/04.clock_jitter.nes"),
        (
            blargg_apu_2005_len_timing_mode0,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/05.len_timing_mode0.nes"
        ),
        (
            blargg_apu_2005_len_timing_mode1,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/06.len_timing_mode1.nes"
        ),
        (
            blargg_apu_2005_irq_flag_timing,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/07.irq_flag_timing.nes"
        ),
        (
            blargg_apu_2005_reset_timing,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/09.reset_timing.nes"
        ),
        (
            blargg_apu_2005_len_halt_timing,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/10.len_halt_timing.nes"
        ),
        (
            blargg_apu_2005_len_reload_timing,
            "./tests/test_roms/apu/blargg_apu_2005.07.30/11.len_reload_timing.nes"
        ),
        (pal_apu_len_ctr, "./tests/test_roms/apu/pal_apu_tests/01.len_ctr.nes"),
        (pal_apu_len_table, "./tests/test_roms/apu/pal_apu_tests/02.len_table.nes"),
        (pal_apu_irq_flag, "./tests/test_roms/apu/pal_apu_tests/03.irq_flag.nes"),
        (
            pal_apu_clock_jitter,
            "./tests/test_roms/apu/pal_apu_tests/04.clock_jitter.nes"
        ),
        (
            pal_apu_len_timing_mode0,
            "./tests/test_roms/apu/pal_apu_tests/05.len_timing_mode0.nes"
        ),
        (
            pal_apu_len_timing_mode1,
            "./tests/test_roms/apu/pal_apu_tests/06.len_timing_mode1.nes"
        ),
        (
            pal_apu_irq_flag_timing,
            "./tests/test_roms/apu/pal_apu_tests/07.irq_flag_timing.nes"
        ),
        (
            pal_apu_irq_timing,
            "./tests/test_roms/apu/pal_apu_tests/08.irq_timing.nes"
        ),
        (
            pal_apu_len_halt_timing,
            "./tests/test_roms/apu/pal_apu_tests/10.len_halt_timing.nes"
        ),
        (
            pal_apu_len_reload_timing,
            "./tests/test_roms/apu/pal_apu_tests/11.len_reload_timing.nes"
        )
    ]
);
