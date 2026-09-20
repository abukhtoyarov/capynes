ppu_manifest!(
    blargg: [
        (oam_read, "./tests/test_roms/ppu/oam_read/oam_read.nes"),
        (oam_stress, "./tests/test_roms/ppu/oam_stress/oam_stress.nes"),
        (ppu_open_bus, "./tests/test_roms/ppu/ppu_open_bus/ppu_open_bus.nes"),
        (ppu_read_buffer, "./tests/test_roms/ppu/ppu_read_buffer/test_ppu_read_buffer.nes")
    ],
    screen_result_code: [
        (
            blargg_ppu_palette_ram,
            "./tests/test_roms/ppu/blargg_ppu_tests_2005_09_15b/palette_ram.nes"
        ),
        (
            blargg_ppu_sprite_ram,
            "./tests/test_roms/ppu/blargg_ppu_tests_2005_09_15b/sprite_ram.nes"
        ),
        (
            blargg_ppu_vbl_clear_time,
            "./tests/test_roms/ppu/blargg_ppu_tests_2005_09_15b/vbl_clear_time.nes"
        ),
        (
            blargg_ppu_vram_access,
            "./tests/test_roms/ppu/blargg_ppu_tests_2005_09_15b/vram_access.nes"
        ),
        (
            blargg_ppu_power_up_palette,
            "./tests/test_roms/ppu/blargg_ppu_tests_2005_09_15b/power_up_palette.nes"
        ),
        (vbl_nmi_timing_1_frame_basics, "./tests/test_roms/ppu/vbl_nmi_timing/1.frame_basics.nes"),
        (vbl_nmi_timing_2_vbl_timing, "./tests/test_roms/ppu/vbl_nmi_timing/2.vbl_timing.nes"),
        (
            vbl_nmi_timing_3_even_odd_frames,
            "./tests/test_roms/ppu/vbl_nmi_timing/3.even_odd_frames.nes"
        ),
        (
            vbl_nmi_timing_4_vbl_clear_timing,
            "./tests/test_roms/ppu/vbl_nmi_timing/4.vbl_clear_timing.nes"
        ),
        (
            vbl_nmi_timing_5_nmi_suppression,
            "./tests/test_roms/ppu/vbl_nmi_timing/5.nmi_suppression.nes"
        ),
        (vbl_nmi_timing_6_nmi_disable, "./tests/test_roms/ppu/vbl_nmi_timing/6.nmi_disable.nes"),
        (vbl_nmi_timing_7_nmi_timing, "./tests/test_roms/ppu/vbl_nmi_timing/7.nmi_timing.nes")
    ],
    screen_text: [
        (ppu_vbl_nmi, "./tests/test_roms/ppu/ppu_vbl_nmi/ppu_vbl_nmi.nes"),
        (sprite_hit_01_basics, "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/01.basics.nes"),
        (
            sprite_hit_02_alignment,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/02.alignment.nes"
        ),
        (sprite_hit_03_corners, "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/03.corners.nes"),
        (sprite_hit_04_flip, "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/04.flip.nes"),
        (
            sprite_hit_05_left_clip,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/05.left_clip.nes"
        ),
        (
            sprite_hit_06_right_edge,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/06.right_edge.nes"
        ),
        (
            sprite_hit_07_screen_bottom,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/07.screen_bottom.nes"
        ),
        (
            sprite_hit_08_double_height,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/08.double_height.nes"
        ),
        (
            sprite_hit_09_timing_basics,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/09.timing_basics.nes"
        ),
        (
            sprite_hit_10_timing_order,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/10.timing_order.nes"
        ),
        (
            sprite_hit_11_edge_timing,
            "./tests/test_roms/ppu/sprite_hit_tests_2005_10_05/11.edge_timing.nes"
        ),
        (sprite_overflow_1_basics, "./tests/test_roms/ppu/sprite_overflow_tests/1.Basics.nes"),
        (sprite_overflow_2_details, "./tests/test_roms/ppu/sprite_overflow_tests/2.Details.nes"),
        (sprite_overflow_3_timing, "./tests/test_roms/ppu/sprite_overflow_tests/3.Timing.nes"),
        (sprite_overflow_4_obscure, "./tests/test_roms/ppu/sprite_overflow_tests/4.Obscure.nes"),
        (sprite_overflow_5_emulator, "./tests/test_roms/ppu/sprite_overflow_tests/5.Emulator.nes")
    ]
);
