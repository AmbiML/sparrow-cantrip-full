# We're not attempting to actually compile Rust files. All we want to make use
# of is the alternative language linkage behaviors in CMake.

set(CMAKE_Rust_COMPILER "/bin/false")
set(CMAKE_Rust_COMPILER_ID "Rust")
set(CMAKE_Rust_PLATFORM_ID "Rust")
set(CMAKE_Rust_COMPILER_VERSION "")

mark_as_advanced(CMAKE_Rust_COMPILER)
set(CMAKE_Rust_COMPILER_LOADED 1)

configure_file(
    "${CMAKE_CURRENT_LIST_DIR}/CMakeRustCompiler.cmake.in"
	"${CMAKE_BINARY_DIR}${CMAKE_FILES_DIRECTORY}/${CMAKE_VERSION}/CMakeRustCompiler.cmake"
    IMMEDIATE @ONLY)

# Silence CMake warnings about not setting this variable
set(CMAKE_Rust_COMPILER_ENV_VAR "")

# We don't need to test this.
set(CMAKE_Rust_COMPILER_WORKS 1 CACHE INTERNAL "")
