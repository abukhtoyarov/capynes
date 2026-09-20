#[path = "common/mod.rs"]
mod common;

#[path = "launchers/blargg.rs"]
mod blargg_launcher;
#[path = "launchers/screen_result_code.rs"]
mod screen_result_code_launcher;
#[path = "launchers/screen_text.rs"]
mod screen_text_launcher;

macro_rules! ppu_manifest {
    (
        blargg: [$(($blargg_name:ident, $blargg_path:expr)),* $(,)?],
        screen_result_code: [$(($screen_name:ident, $screen_path:expr)),* $(,)?],
        screen_text: [$(($text_name:ident, $text_path:expr)),* $(,)?]
    ) => {
        mod blargg {
            use super::blargg_launcher::run_blargg_rom_test;

            $(
                #[test]
                fn $blargg_name() {
                    run_blargg_rom_test($blargg_path, false);
                }
            )*
        }

        mod screen_result_code {
            use super::screen_result_code_launcher::run_screen_result_code_rom_test;

            $(
                #[test]
                fn $screen_name() {
                    run_screen_result_code_rom_test($screen_path);
                }
            )*
        }

        mod screen_text {
            use super::screen_text_launcher::run_low_intrusion_screen_rom_test;

            $(
                #[test]
                fn $text_name() {
                    run_low_intrusion_screen_rom_test($text_path);
                }
            )*
        }
    };
}

include!("test_roms/ppu/manifest.rs");

mod frontier {
    use capynes_lib::rom::Rom;

    #[test]
    fn other_linusmus_invalid_ines_header() {
        let err = match Rom::new("./tests/test_roms/ppu/other/LINUSMUS.NES") {
            Ok(_) => panic!("LINUSMUS is expected to have invalid iNES header"),
            Err(err) => err,
        };
        assert!(
            err.contains("Not a NES ROM"),
            "unexpected error for LINUSMUS: {err}"
        );
    }

}
