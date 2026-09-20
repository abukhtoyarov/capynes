#[path = "common/mod.rs"]
mod common;

#[path = "launchers/blargg.rs"]
mod blargg_launcher;
#[path = "launchers/screen_result_code.rs"]
mod screen_result_code_launcher;
#[path = "launchers/screen_text.rs"]
mod screen_text_launcher;

macro_rules! apu_manifest {
    (
        blargg: [$(($blargg_name:ident, $blargg_path:expr, $blargg_reset:expr)),* $(,)?],
        screen_result_code: [$(($screen_name:ident, $screen_path:expr)),* $(,)?]
    ) => {
        mod blargg {
            use super::blargg_launcher::run_blargg_rom_test;

            $(
                #[test]
                fn $blargg_name() {
                    run_blargg_rom_test($blargg_path, $blargg_reset);
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
    };
}

include!("test_roms/apu/manifest.rs");
