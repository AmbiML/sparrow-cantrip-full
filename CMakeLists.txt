cmake_minimum_required(VERSION 3.7.2)
project(ProcessManager C ASM)

set(project_dir "${CMAKE_CURRENT_LIST_DIR}/../../")
file(GLOB project_modules ${project_dir}/projects/*)
list(
    APPEND
        CMAKE_MODULE_PATH
        ${project_dir}/kernel
        ${project_dir}/tools/seL4/cmake-tool/helpers/
        ${project_dir}/tools/seL4/elfloader-tool/
        ${project_modules}
)
set(POLLY_DIR ${project_dir}/tools/polly CACHE INTERNAL "")

add_executable(ProcessManager src/main.c)

target_link_libraries(ProcessManager
  sel4runtime sel4
  muslc utils sel4muslcsys sel4platsupport sel4utils sel4debug)

include(rootserver)
DeclareRootserver(ProcessManager)

include(simulation)
GenerateSimulateScript()
