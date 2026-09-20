#[path = "common/mod.rs"]
mod common;

#[path = "launchers/scripted_input.rs"]
mod scripted_input_launcher;

macro_rules! input_manifest {
    (
        scripted_input: {
            results: [$(($result_name:ident, $result_path:expr, $success_marker:expr)),* $(,)?],
            scripted: [$(($scripted_name:ident, $scripted_path:expr)),* $(,)?]
        }
    ) => {
        mod scripted_input {
            use super::scripted_input_launcher::{
                run_read_joy3_button_sequence_test, run_read_joy3_result_test,
            };

            $(
                #[test]
                fn $result_name() {
                    run_read_joy3_result_test($result_path, $success_marker);
                }
            )*

            $(
                #[test]
                fn $scripted_name() {
                    run_read_joy3_button_sequence_test($scripted_path);
                }
            )*
        }
    };
}

include!("test_roms/input/manifest.rs");
