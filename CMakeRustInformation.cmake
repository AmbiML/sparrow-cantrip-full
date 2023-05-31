include(CMakeLanguageInformation)

# We just want to use C's linkages, but without CAmkES' extra junk like the crt0
# pre- and postamble. So we're essentially bastardizing CMake's language support
# infrastructure to allow for linking Rust executables without C runtimes.
set(CMAKE_Rust_LINK_EXECUTABLE
  "<CMAKE_C_COMPILER> <FLAGS> <CMAKE_C_LINK_FLAGS> <LINK_FLAGS> <OBJECTS> <LINK_LIBRARIES> -o <TARGET>")

set(CMAKE_Rust_INFORMATION_LOADED 1)
