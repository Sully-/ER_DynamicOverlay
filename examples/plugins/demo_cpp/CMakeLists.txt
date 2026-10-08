cmake_minimum_required(VERSION 3.20)
project(demo_cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Static CRT so the plugin does not depend on a matching VCRUNTIME DLL.
if(MSVC)
  set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endif()

add_library(demo_cpp SHARED demo_cpp.cpp)
target_include_directories(demo_cpp PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../sdk")

set_target_properties(demo_cpp PROPERTIES
  OUTPUT_NAME "demo_cpp"
  PREFIX ""
)

if(MSVC)
  target_compile_options(demo_cpp PRIVATE /W4 /EHsc)
else()
  target_compile_options(demo_cpp PRIVATE -Wall -Wextra)
endif()
