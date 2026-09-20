input_manifest!(
    scripted_input: {
        results: [
            (
                read_joy3_count_errors,
                "./tests/test_roms/input/read_joy3/count_errors.nes",
                "conflicts: 0/1000"
            ),
            (
                read_joy3_count_errors_fast,
                "./tests/test_roms/input/read_joy3/count_errors_fast.nes",
                "errors: 0/1000"
            )
        ],
        scripted: [
            (
                read_joy3_test_buttons,
                "./tests/test_roms/input/read_joy3/test_buttons.nes"
            )
        ]
    }
);
