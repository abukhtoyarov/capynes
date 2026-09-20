#[test]
fn mmc1_a12_screen_oracle_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const SMOKE_CYCLES: u64 = 5_000_000;

    while total_cpu_cycles < SMOKE_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }
    let signature = mmc1_a12_signature(&read_nametable_text(&mut nes));
    assert!(signature.contains(r#"   ,,"  61 , #(2 !+$ 2" -+(-$   "#));
    assert!(signature.contains(r#"         ".4-3$1 3$23           "#));
    assert!(signature.contains(r#" 42$ 4;# +;1 3.  #)423 #$+ 8    "#));
}

#[test]
fn mmc1_a12_screen_oracle_is_stable_across_checkpoints() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;

    let mut snapshots = Vec::new();
    for checkpoint in [5_000_000u64, 10_000_000u64, 20_000_000u64] {
        while total_cpu_cycles < checkpoint {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
        }
        let signature = mmc1_a12_signature(&read_nametable_text(&mut nes));
        snapshots.push(signature);
    }

    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
}

#[test]
fn mmc1_a12_screen_oracle_is_reproducible_from_power_on() {
    fn run_to_signature() -> String {
        let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
        nes.reset();

        let mut pending_nmi = false;
        let mut total_cpu_cycles = 0u64;
        const TARGET_CYCLES: u64 = 5_000_000;

        while total_cpu_cycles < TARGET_CYCLES {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
        }

        mmc1_a12_signature(&read_nametable_text(&mut nes))
    }

    let first = run_to_signature();
    let second = run_to_signature();
    assert_eq!(first, second);
}

#[test]
fn mmc1_a12_console_reset_recovers_same_screen_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const BASELINE_CYCLES: u64 = 5_000_000;
    const PRERUN_CYCLES: u64 = 12_000_000;

    while total_cpu_cycles < BASELINE_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }
    let baseline = mmc1_a12_signature(&read_nametable_text(&mut nes));

    while total_cpu_cycles < PRERUN_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    nes.console_reset();
    pending_nmi = false;
    total_cpu_cycles = 0;

    while total_cpu_cycles < BASELINE_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }
    let after_reset = mmc1_a12_signature(&read_nametable_text(&mut nes));
    assert_eq!(baseline, after_reset);
}

#[test]
fn mmc1_a12_runtime_oracle_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 5_000_000;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    assert_eq!(
        mmc1_a12_runtime_signature(&mut nes),
        (0xC653, 0x9F, 0x10, 0x00, 0x67, 0xFD, 0x9F, 0x00, 0x00, 0x00)
    );
}

#[test]
fn mmc1_a12_runtime_oracle_after_console_reset() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 5_000_000;
    const PRERUN_CYCLES: u64 = 12_000_000;

    while total_cpu_cycles < PRERUN_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    nes.console_reset();
    pending_nmi = false;
    total_cpu_cycles = 0;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    assert_eq!(
        mmc1_a12_runtime_signature(&mut nes),
        (0xC653, 0x9F, 0x10, 0x00, 0x67, 0xFD, 0x9F, 0x00, 0x00, 0x00)
    );
}

#[test]
fn mmc1_a12_runtime_oracle_is_reproducible_between_cold_boots() {
    fn run_to_runtime_signature() -> (u16, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
        let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
        nes.reset();

        let mut pending_nmi = false;
        let mut total_cpu_cycles = 0u64;
        const TARGET_CYCLES: u64 = 5_000_000;

        while total_cpu_cycles < TARGET_CYCLES {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
        }

        mmc1_a12_runtime_signature(&mut nes)
    }

    let first = run_to_runtime_signature();
    let second = run_to_runtime_signature();
    assert_eq!(first, second);
}

#[test]
fn mmc1_a12_screen_oracle_is_stable_for_common_startup_buttons() {
    fn run_with_startup_buttons(buttons: Buttons) -> String {
        let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
        nes.reset();
        nes.set_controller1(buttons);

        let mut pending_nmi = false;
        let mut total_cpu_cycles = 0u64;
        const HOLD_CYCLES: u64 = 800_000;
        const TARGET_CYCLES: u64 = 5_000_000;

        while total_cpu_cycles < TARGET_CYCLES {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
            if total_cpu_cycles >= HOLD_CYCLES {
                nes.set_controller1(Buttons::empty());
            }
        }

        mmc1_a12_signature(&read_nametable_text(&mut nes))
    }

    let baseline = run_with_startup_buttons(Buttons::empty());
    for buttons in [Buttons::START, Buttons::A, Buttons::SELECT] {
        assert_eq!(run_with_startup_buttons(buttons), baseline);
    }
}

#[test]
fn mmc1_a12_runtime_core_oracle_is_stable_across_checkpoints() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;

    let mut core_snapshots = Vec::new();
    let mut pcs = Vec::new();
    for checkpoint in [5_000_000u64, 10_000_000u64, 20_000_000u64] {
        while total_cpu_cycles < checkpoint {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
        }
        core_snapshots.push(mmc1_a12_runtime_core_signature(&mut nes));
        pcs.push(nes.cpu.pc);
    }
    for (acc, rx, flags, sp, s6000, s6001, s6002, s6003) in core_snapshots {
        assert_eq!(acc, 0x9F);
        assert_eq!(rx, 0x10);
        assert!(flags == 0x67 || flags == 0xE5);
        assert_eq!(sp, 0xFD);
        assert_eq!(s6000, 0x9F);
        assert_eq!(s6001, 0x00);
        assert_eq!(s6002, 0x00);
        assert_eq!(s6003, 0x00);
    }
    for pc in pcs {
        assert!((0xC64E..=0xC653).contains(&pc));
    }
}

#[test]
fn mmc1_a12_runtime_core_oracle_is_stable_for_common_startup_buttons() {
    fn run_with_startup_buttons(buttons: Buttons) -> ((u8, u8, u8, u8, u8, u8, u8, u8), u16, u8) {
        let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/mmc1_a12/mmc1_a12.nes");
        nes.reset();
        nes.set_controller1(buttons);

        let mut pending_nmi = false;
        let mut total_cpu_cycles = 0u64;
        const HOLD_CYCLES: u64 = 800_000;
        const TARGET_CYCLES: u64 = 5_000_000;

        while total_cpu_cycles < TARGET_CYCLES {
            total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
            if total_cpu_cycles >= HOLD_CYCLES {
                nes.set_controller1(Buttons::empty());
            }
        }

        (mmc1_a12_runtime_core_signature(&mut nes), nes.cpu.pc, nes.cpu.ry)
    }

    let baseline = run_with_startup_buttons(Buttons::empty());
    for buttons in [Buttons::START, Buttons::A, Buttons::SELECT] {
        let (core, pc, y) = run_with_startup_buttons(buttons);
        assert_eq!(core.0, baseline.0.0);
        assert_eq!(core.1, baseline.0.1);
        assert_eq!(core.3, baseline.0.3);
        assert_eq!(core.4, baseline.0.4);
        assert_eq!(core.5, baseline.0.5);
        assert_eq!(core.6, baseline.0.6);
        assert_eq!(core.7, baseline.0.7);
        assert!(core.2 == 0x67 || core.2 == 0xE5);
        assert!((0xC64E..=0xC653).contains(&pc));
        let _ = y;
    }
}

#[test]
fn nrom368_fail368_screen_oracle_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/nrom368/fail368.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 5_000_000;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    let screen = read_nametable_text(&mut nes);
    assert!(screen.contains("????????========================"));
    assert!(screen.contains("<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"));
}

#[test]
fn nrom368_test1_screen_oracle_is_blank() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/nrom368/test1.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 5_000_000;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    let screen = read_nametable_text(&mut nes);
    assert!(screen.trim().is_empty());
}

#[test]
fn action53_test28_screen_oracle_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/action53/test28.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 12_000_000;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    let screen = read_nametable_text(&mut nes);
    assert!(screen.contains("      B                 d       "));
    assert!(screen.contains("      C                 e       "));
    assert!(screen.contains("        @      X@   @  @   N \\  "));
    assert!(screen.contains("        A      YA   A  A   O ]  "));
}

#[test]
fn action53_streemerz_bundle_screen_oracle_signature() {
    let mut nes = NesTestFixture::new("./tests/test_roms/oracle/mappers/action53/Streemerz_bundle.nes");
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    const TARGET_CYCLES: u64 = 80_000_000;

    while total_cpu_cycles < TARGET_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    let screen = read_nametable_text(&mut nes);
    assert!(screen.contains("HIJKLM"));
    assert!(screen.contains("abcdefghijklmno"));
    assert!(screen.contains("pqrstuvwxyz{|}~"));
}

oracle_visual_frame_manifest!(
    (full_palette, "./tests/test_roms/oracle/ppu/full_palette/full_palette.nes", 5_000_000, 10105057333893973949),
    (full_palette_smooth, "./tests/test_roms/oracle/ppu/full_palette/full_palette_smooth.nes", 5_000_000, 16494434667675830205),
    (flowing_palette, "./tests/test_roms/oracle/ppu/full_palette/flowing_palette.nes", 5_000_000, 13339882661999697654),
    (nmi_sync_ntsc, "./tests/test_roms/oracle/ppu/nmi_sync/demo_ntsc.nes", 5_000_000, 3653187904045765765),
    (nmi_sync_pal, "./tests/test_roms/oracle/ppu/nmi_sync/demo_pal.nes", 5_000_000, 6722431151643623437),
    (scrolltest, "./tests/test_roms/oracle/ppu/scrolltest/scroll.nes", 5_000_000, 10847042202027041422),
    (window5_ntsc, "./tests/test_roms/oracle/ppu/window5/colorwin_ntsc.nes", 5_000_000, 119208544377508404),
    (window5_pal, "./tests/test_roms/oracle/ppu/window5/colorwin_pal.nes", 5_000_000, 4479787509553643629),
    (pixel_240pee_nrom, "./tests/test_roms/oracle/ppu/240pee/240pee.nes", 5_000_000, 10704525882044693853),
    (pixel_240pee_bnrom, "./tests/test_roms/oracle/ppu/240pee/240pee-bnrom.nes", 5_000_000, 10704525882044693853),
    (spritecans_2011, "./tests/test_roms/oracle/ppu/spritecans-2011/spritecans.nes", 5_000_000, 9788268847233173584),
    (ny2011, "./tests/test_roms/oracle/ppu/ny2011/ny2011.nes", 5_000_000, 15186849822972945274),
    (blargg_litewall_9, "./tests/test_roms/oracle/ppu/blargg_litewall/blargg_litewall-9.nes", 5_000_000, 14860152543352677157),
    (blargg_litewall_10c, "./tests/test_roms/oracle/ppu/blargg_litewall/blargg_litewall-10c.nes", 5_000_000, 14860152543352677157),
    (blargg_litewall_litewall2, "./tests/test_roms/oracle/ppu/blargg_litewall/litewall2.nes", 5_000_000, 14056916110911006245),
    (blargg_litewall_litewall3, "./tests/test_roms/oracle/ppu/blargg_litewall/litewall3.nes", 5_000_000, 413694458079085093),
    (blargg_litewall_litewall5, "./tests/test_roms/oracle/ppu/blargg_litewall/litewall5.nes", 5_000_000, 3335069300613883685),
    (window2_ntsc, "./tests/test_roms/oracle/ppu/other/window2_ntsc.nes", 5_000_000, 8627912229228847036),
    (window2_pal, "./tests/test_roms/oracle/ppu/other/window2_pal.nes", 5_000_000, 8366741701986193944),
    (window_old_ntsc, "./tests/test_roms/oracle/ppu/other/window_old_ntsc.nes", 5_000_000, 6388501417672795805),
    (window_old_pal, "./tests/test_roms/oracle/ppu/other/window_old_pal.nes", 5_000_000, 10671454696366844985),
    (raster_chroma_luma, "./tests/test_roms/oracle/ppu/other/RasterChromaLuma.NES", 5_000_000, 1641224879694029477),
    (raster_demo, "./tests/test_roms/oracle/ppu/other/RasterDemo.NES", 5_000_000, 3661543751739766531),
    (raster_test1, "./tests/test_roms/oracle/ppu/other/RasterTest1.NES", 5_000_000, 938389820808225385),
    (raster_test2, "./tests/test_roms/oracle/ppu/other/RasterTest2.NES", 5_000_000, 13337925953418388607),
    (raster_test3, "./tests/test_roms/oracle/ppu/other/RasterTest3.NES", 5_000_000, 15657613735921430212),
    (raster_test3a, "./tests/test_roms/oracle/ppu/other/RasterTest3a.NES", 5_000_000, 2916917801859912412),
    (raster_test3b, "./tests/test_roms/oracle/ppu/other/RasterTest3b.NES", 5_000_000, 17595949017544032436),
    (raster_test3c, "./tests/test_roms/oracle/ppu/other/RasterTest3c.NES", 5_000_000, 9904937564910670236),
    (raster_test3d, "./tests/test_roms/oracle/ppu/other/RasterTest3d.NES", 5_000_000, 13161866547007455235),
    (raster_test3e, "./tests/test_roms/oracle/ppu/other/RasterTest3e.NES", 5_000_000, 13521214955345574229),
    (other_2003_test, "./tests/test_roms/oracle/ppu/other/2003-test.nes", 5_000_000, 1335489928300412519),
    (other_8bitpeoples_deadline_console_invitro, "./tests/test_roms/oracle/ppu/other/8bitpeoples_-_deadline_console_invitro.nes", 5_000_000, 7663094697711018585),
    (other_blocks, "./tests/test_roms/oracle/ppu/other/BLOCKS.NES", 5_000_000, 2420303280567117448),
    (other_bladebuster, "./tests/test_roms/oracle/ppu/other/BladeBuster.nes", 5_000_000, 11641093314848264172),
    (other_cmc80s, "./tests/test_roms/oracle/ppu/other/CMC80s.NES", 5_000_000, 14233574445560923096),
    (other_dropoff7, "./tests/test_roms/oracle/ppu/other/DROPOFF7.NES", 5_000_000, 2145227594786238308),
    (other_duelito, "./tests/test_roms/oracle/ppu/other/Duelito.nes", 5_000_000, 7604151323677178186),
    (other_flame, "./tests/test_roms/oracle/ppu/other/FLAME.NES", 5_000_000, 1986111278819701495),
    (other_genie, "./tests/test_roms/oracle/ppu/other/GENIE.NES", 5_000_000, 5120787513058107125),
    (other_greys, "./tests/test_roms/oracle/ppu/other/GREYS.NES", 5_000_000, 4915748681690325845),
    (other_motion, "./tests/test_roms/oracle/ppu/other/MOTION.NES", 5_000_000, 12476500591789887013),
    (other_pcm_demo_wgraphics, "./tests/test_roms/oracle/ppu/other/PCM.demo.wgraphics.nes", 5_000_000, 3686904176144612008),
    (other_retrocoders_years_behind, "./tests/test_roms/oracle/ppu/other/Retrocoders - Years behind.NES", 5_000_000, 8170025950045659485),
    (other_s0, "./tests/test_roms/oracle/ppu/other/S0.NES", 5_000_000, 15989528277253033181),
    (other_sprite, "./tests/test_roms/oracle/ppu/other/SPRITE.NES", 5_000_000, 2355697073638740491),
    (other_sayoonara, "./tests/test_roms/oracle/ppu/other/Sayoonara!.NES", 5_000_000, 13164680592149568180),
    (other_simple_parallax_demo, "./tests/test_roms/oracle/ppu/other/SimpleParallaxDemo.nes", 5_000_000, 12665201615559784159),
    (other_tanespot, "./tests/test_roms/oracle/ppu/other/TANESPOT.NES", 5_000_000, 17389986306788949421),
    (other_test, "./tests/test_roms/oracle/ppu/other/TEST.NES", 5_000_000, 1752075246302248909),
    (other_apocalypse, "./tests/test_roms/oracle/ppu/other/apocalypse.nes", 5_000_000, 6494010197755677940),
    (other_blargg_litewall_2, "./tests/test_roms/oracle/ppu/other/blargg_litewall-2.nes", 5_000_000, 13215386552059895161),
    (other_blargg_litewall_9, "./tests/test_roms/oracle/ppu/other/blargg_litewall-9.nes", 5_000_000, 14860152543352677157),
    (other_demo_jitter, "./tests/test_roms/oracle/ppu/other/demo jitter.nes", 5_000_000, 7216072243751678359),
    (other_demo, "./tests/test_roms/oracle/ppu/other/demo.nes", 5_000_000, 8772895741644466464),
    (other_fceuxd, "./tests/test_roms/oracle/ppu/other/fceuxd.nes", 5_000_000, 1109164905547445189),
    (other_firefly, "./tests/test_roms/oracle/ppu/other/firefly.nes", 5_000_000, 15586089140450295738),
    (other_high_hopes, "./tests/test_roms/oracle/ppu/other/high-hopes.nes", 5_000_000, 14860152543352677157),
    (other_logo_e, "./tests/test_roms/oracle/ppu/other/logo (E).nes", 5_000_000, 8369744202683067463),
    (other_manhole, "./tests/test_roms/oracle/ppu/other/manhole.nes", 5_000_000, 11139055597400209700),
    (other_max_300, "./tests/test_roms/oracle/ppu/other/max-300.nes", 5_000_000, 11131869775611985114),
    (other_midscanline, "./tests/test_roms/oracle/ppu/other/midscanline.nes", 5_000_000, 12142848669297891668),
    (other_minipack, "./tests/test_roms/oracle/ppu/other/minipack.nes", 5_000_000, 15313411019569891317),
    (other_nescafe, "./tests/test_roms/oracle/ppu/other/nescafe.nes", 5_000_000, 15672099615146731595),
    (other_nestopia, "./tests/test_roms/oracle/ppu/other/nestopia.nes", 5_000_000, 430298754087585925),
    (other_new_game, "./tests/test_roms/oracle/ppu/other/new-game.nes", 5_000_000, 1559281326496060436),
    (other_nintendulator, "./tests/test_roms/oracle/ppu/other/nintendulator.nes", 5_000_000, 9383864145615849637),
    (apu_volume_tests, "./tests/test_roms/oracle/apu/volume_tests/volumes.nes", 5_000_000, 1664604141990159141),
    (apu_soundtest, "./tests/test_roms/oracle/apu/soundtest/SNDTEST.NES", 5_000_000, 10429803556814967957),
    (apu_dpcm_letterbox, "./tests/test_roms/oracle/apu/dpcmletterbox/dpcmletterbox.nes", 5_000_000, 17545988446370336003),
    (apu_stars_se, "./tests/test_roms/oracle/apu/stars_se/StarsSE.NES", 5_000_000, 4148785516070457366),
    (apu_dmc_dma_during_read4_dma_2007_read, "./tests/test_roms/oracle/apu/dmc_dma_during_read4/dma_2007_read.nes", 5_000_000, 14860152543352677157),
    (apu_dmc_dma_during_read4_dma_2007_write, "./tests/test_roms/oracle/apu/dmc_dma_during_read4/dma_2007_write.nes", 5_000_000, 14860152543352677157),
    (apu_dmc_dma_during_read4_dma_4016_read, "./tests/test_roms/oracle/apu/dmc_dma_during_read4/dma_4016_read.nes", 5_000_000, 14860152543352677157),
    (apu_dmc_dma_during_read4_double_2007_read, "./tests/test_roms/oracle/apu/dmc_dma_during_read4/double_2007_read.nes", 5_000_000, 6098319203839135669),
    (apu_dmc_dma_during_read4_read_write_2007, "./tests/test_roms/oracle/apu/dmc_dma_during_read4/read_write_2007.nes", 5_000_000, 2262369106537174935),
    (apu_sprdma_and_dmc_dma, "./tests/test_roms/oracle/apu/sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes", 5_000_000, 14860152543352677157),
    (apu_sprdma_and_dmc_dma_512, "./tests/test_roms/oracle/apu/sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes", 5_000_000, 14860152543352677157),
    (apu_dmc_tests_status, "./tests/test_roms/oracle/apu/dmc_tests/status.nes", 5_000_000, 3746497375122105125),
    (apu_dmc_tests_status_irq, "./tests/test_roms/oracle/apu/dmc_tests/status_irq.nes", 5_000_000, 3746497375122105125),
    (apu_dmc_tests_latency, "./tests/test_roms/oracle/apu/dmc_tests/latency.nes", 5_000_000, 3746497375122105125),
    (apu_dmc_tests_buffer_retained, "./tests/test_roms/oracle/apu/dmc_tests/buffer_retained.nes", 5_000_000, 3746497375122105125),
    (other_oam3, "./tests/test_roms/oracle/ppu/other/oam3.nes", 5_000_000, 3835881263698062472),
    (other_oc, "./tests/test_roms/oracle/ppu/other/oc.nes", 5_000_000, 1028054381780641928),
    (other_physics_0_1, "./tests/test_roms/oracle/ppu/other/physics.0.1.nes", 5_000_000, 11011821810218814605),
    (other_pulsar, "./tests/test_roms/oracle/ppu/other/pulsar.nes", 5_000_000, 5894870755810238435),
    (
        other_quantum_disco_brothers,
        "./tests/test_roms/oracle/ppu/other/quantum_disco_brothers_by_wAMMA.nes",
        5_000_000,
        8500665167762133059
    ),
    (other_rastesam4, "./tests/test_roms/oracle/ppu/other/rastesam4.nes", 5_000_000, 5763986545882770766),
    (other_read2004, "./tests/test_roms/oracle/ppu/other/read2004.nes", 5_000_000, 3361711280807477509),
    (other_snow, "./tests/test_roms/oracle/ppu/other/snow.nes", 5_000_000, 10386003429096078943),
    (other_test001, "./tests/test_roms/oracle/ppu/other/test001.nes", 5_000_000, 4270858295516261157),
    (ppu_scanline, "./tests/test_roms/oracle/ppu/scanline/scanline.nes", 5_000_000, 17054538527757850893),
    (ppu_scanline_a1, "./tests/test_roms/oracle/ppu/scanline_a1/scanline.nes", 5_000_000, 17054538527757850893),
    (mappers_mmc3_test_scanline_timing, "./tests/test_roms/mappers/mmc3_test/4-scanline_timing.nes", 5_000_000, 16381457529581557815),
    (mappers_mmc5exram, "./tests/test_roms/oracle/mappers/mmc5exram/mmc5exram.nes", 5_000_000, 16305494919132020628),
    (mappers_mmc5test, "./tests/test_roms/oracle/mappers/mmc5test/mmc5test.nes", 5_000_000, 15063546817011655453),
    (mappers_mmc5test_v2, "./tests/test_roms/oracle/mappers/mmc5test_v2/mmc5test.nes", 5_000_000, 12769034722009975170),
    (mappers_m22chrbankingtest_0_127, "./tests/test_roms/oracle/mappers/m22chrbankingtest/0-127.nes", 5_000_000, 4450613487799308865),
    (cpu_nes15_ntsc, "./tests/test_roms/oracle/cpu/nes15/nes15-NTSC.nes", 5_000_000, 9245133309976804706),
    (cpu_nes15_pal, "./tests/test_roms/oracle/cpu/nes15/nes15-PAL.nes", 5_000_000, 9245133309976804706),
    (cpu_tutor, "./tests/test_roms/oracle/cpu/tutor/tutor.nes", 5_000_000, 4816086339735395223),
    (cpu_stomper, "./tests/test_roms/oracle/cpu/stomper/smwstomp.nes", 5_000_000, 12949662192999370153),
    (input_paddle_test3, "./tests/test_roms/oracle/input/paddle_test3/PaddleTest.nes", 5_000_000, 14050046864434340983),
    (input_vaus_test, "./tests/test_roms/oracle/input/vaus_test/vaus-test.nes", 5_000_000, 10607559978482236128),
    (tvpassfail, "./tests/test_roms/oracle/ppu/tvpassfail/tv.nes", 5_000_000, 12247344030265243765),
);
