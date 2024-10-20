/* SPDX-License-Identifier: MIT
 * Copyright(c) 2024 Darek Stojaczyk
 */

// Use in your IDE; this doesn't build and it never will

struct SomeStruct {
    name: String,
    val1: u64,
}

fn main() {
    futures::executor::block_on(async {
        let some_object_with_whatever_name = SomeStruct {
            name: "Example".into(),
            val1: 42,
        };

        futures::select!(
            _ = std::future::pending::<()>() => {
                some_object_with_whatever_name.
                // place caret at the end;
                // there is no autocompletion
            }
        );

        async_proc::select!(
            _ = std::future::pending::<()>() => {
                some_object_with_whatever_name.
                // place caret at the end;
                // autocompletion as usual
            }
        );
    });
}
