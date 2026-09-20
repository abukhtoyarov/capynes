#[path = "common/mod.rs"]
mod common;

#[path = "launchers/blargg.rs"]
mod blargg_launcher;
#[path = "launchers/screen_text.rs"]
mod screen_text_launcher;
#[allow(dead_code)]
#[path = "launchers/scripted_input.rs"]
mod scripted_input_launcher;

macro_rules! mappers_manifest {
    (
        blargg: [$(($name:ident, $path:expr)),* $(,)?],
        screen_text: [$(($text_name:ident, $text_path:expr)),* $(,)?]
    ) => {
        mod blargg {
            use super::blargg_launcher::run_blargg_rom_test;

            $(
                #[test]
                fn $name() {
                    run_blargg_rom_test($path, false);
                }
            )*
        }

        mod screen_text {
            #[allow(unused_imports)]
            use super::screen_text_launcher::run_screen_rom_test;

            $(
                #[test]
                fn $text_name() {
                    run_screen_rom_test($text_path);
                }
            )*
        }
    };
}

include!("test_roms/mappers/manifest.rs");

mod screen_text_low_intrusion {
    use super::screen_text_launcher::run_low_intrusion_screen_rom_test;

    #[test]
    fn mmc3_irq_tests_details() {
        run_low_intrusion_screen_rom_test("./tests/test_roms/mappers/mmc3_irq_tests/2.Details.nes");
    }
}

mod mmc3_rev_a {
    use super::blargg_launcher::run_blargg_rom_test_mmc3_rev_a;
    use super::screen_text_launcher::run_screen_rom_test_mmc3_rev_a;

    #[test]
    fn blargg_mmc3_test_mmc6() {
        run_blargg_rom_test_mmc3_rev_a("./tests/test_roms/mappers/mmc3_test/6-MMC6.nes", false);
    }

    #[test]
    fn screen_text_mmc3_irq_tests_rev_a() {
        run_screen_rom_test_mmc3_rev_a("./tests/test_roms/mappers/mmc3_irq_tests/5.MMC3_rev_A.nes");
    }
}

mod frontier {
    use super::screen_text_launcher::run_screen_rom_test;
    use capynes_lib::rom::Rom;

    #[test]
    fn mmc3_irq_tests_scanline_timing() {
        run_screen_rom_test("./tests/test_roms/mappers/mmc3_irq_tests/4.Scanline_timing.nes");
    }

    #[test]
    fn fdsirqtests_v7_patched_is_rejected_by_ines_loader() {
        let err = match Rom::new("./tests/test_roms/mappers/fdsirqtests/fdsirqtestsV7_patched.fds")
        {
            Ok(_) => panic!("FDS image should be rejected by iNES loader"),
            Err(err) => err,
        };
        assert!(
            err.contains("Not a NES ROM"),
            "unexpected loader error: {err}"
        );
    }

    #[test]
    fn fdsirqtests_legacy_is_rejected_by_ines_loader() {
        let err = match Rom::new("./tests/test_roms/mappers/fdsirqtests/fdsirqtests.fds") {
            Ok(_) => panic!("FDS image should be rejected by iNES loader"),
            Err(err) => err,
        };
        assert!(
            err.contains("Not a NES ROM"),
            "unexpected loader error: {err}"
        );
    }
}
