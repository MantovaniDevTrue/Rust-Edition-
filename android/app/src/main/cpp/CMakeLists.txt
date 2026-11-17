cmake_minimum_required(VERSION 3.22)
project(rust_edition_android LANGUAGES C)

find_program(CARGO cargo REQUIRED)
get_filename_component(RUST_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../.." ABSOLUTE)
set(CARGO_OUT "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(RUST_ARCHIVE "${CARGO_OUT}/aarch64-linux-android/release/librust_edition.a")

add_custom_command(
    OUTPUT "${RUST_ARCHIVE}"
    COMMAND "${CARGO}" build --locked --release --lib --target aarch64-linux-android
        --target-dir "${CARGO_OUT}"
    WORKING_DIRECTORY "${RUST_ROOT}"
    DEPENDS "${RUST_ROOT}/Cargo.toml" "${RUST_ROOT}/Cargo.lock"
        "${RUST_ROOT}/src/lib.rs" "${RUST_ROOT}/src/android.rs"
        "${RUST_ROOT}/src/world_file.rs"
    VERBATIM
)
add_custom_target(rust_core DEPENDS "${RUST_ARCHIVE}")
add_library(rust_edition_android SHARED bridge.c)
add_dependencies(rust_edition_android rust_core)
target_link_libraries(rust_edition_android PRIVATE "${RUST_ARCHIVE}" dl log m)
