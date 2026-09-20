#[path = "common/mod.rs"]
mod common;

#[path = "launchers/blargg.rs"]
mod blargg_launcher;
#[path = "launchers/log_cmp.rs"]
mod log_cmp_launcher;
#[path = "launchers/screen_text.rs"]
mod screen_text_launcher;
#[allow(dead_code)]
#[path = "launchers/scripted_input.rs"]
mod scripted_input_launcher;
#[path = "launchers/startup_buttons.rs"]
mod startup_buttons_launcher;

macro_rules! cpu_manifest {
    (
        blargg: [$(($blargg_name:ident, $blargg_path:expr, $blargg_reset:expr)),* $(,)?],
        log_cmp: [$(($log_name:ident, $log_rom:expr, $log_path:expr)),* $(,)?],
        screen_text: [$(($screen_name:ident, $screen_path:expr)),* $(,)?],
        startup_buttons: [$(($startup_name:ident, $startup_path:expr, $startup_buttons:expr, $startup_mode:expr)),* $(,)?]
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

        mod log_cmp {
            use super::log_cmp_launcher::run_log_cmp_test;

            $(
                #[test]
                fn $log_name() {
                    run_log_cmp_test($log_rom, $log_path);
                }
            )*
        }

        mod screen_text {
            use super::screen_text_launcher::run_screen_rom_test;

            $(
                #[test]
                fn $screen_name() {
                    run_screen_rom_test($screen_path);
                }
            )*
        }

        mod startup_buttons {
            use super::startup_buttons_launcher::run_cpu_timing_test6;
            use capynes_lib::ctrl::Buttons;

            $(
                #[test]
                fn $startup_name() {
                    run_cpu_timing_test6($startup_path, $startup_buttons, $startup_mode);
                }
            )*
        }
    };
}

include!("test_roms/cpu/manifest.rs");

mod scripted_input {
    use super::scripted_input_launcher::run_nestress_menu_probe;

    #[test]
    fn nestress_cpu_test_code_scripted() {
        run_nestress_menu_probe("./tests/test_roms/cpu/stress/NEStress.NES");
    }
}

mod frontier {
    use super::log_cmp_launcher::run_log_cmp_test;

    #[test]
    fn nestest_other_log_cmp() {
        run_log_cmp_test(
            "./tests/test_roms/cpu/nestest.nes",
            "./tests/test_roms/cpu/nestest.log",
        );
    }
}
