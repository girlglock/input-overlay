cmake_minimum_required(VERSION 3.24)
project(rmlui_bridge C CXX)

include(FetchContent)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)

foreach(cfg "" _DEBUG _RELEASE _RELWITHDEBINFO _MINSIZEREL)
	set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY${cfg} "${CMAKE_BINARY_DIR}/libs")
endforeach()

set(FT_DISABLE_ZLIB ON CACHE BOOL "" FORCE)
set(FT_DISABLE_BZIP2 ON CACHE BOOL "" FORCE)
set(FT_DISABLE_PNG ON CACHE BOOL "" FORCE)
set(FT_DISABLE_HARFBUZZ ON CACHE BOOL "" FORCE)
set(FT_DISABLE_BROTLI ON CACHE BOOL "" FORCE)
FetchContent_Declare(freetype
	GIT_REPOSITORY https://github.com/freetype/freetype
	GIT_TAG VER-2-13-3
	GIT_SHALLOW TRUE)

set(GLFW_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(GLFW_INSTALL OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_WAYLAND OFF CACHE BOOL "" FORCE)
FetchContent_Declare(glfw
	GIT_REPOSITORY https://github.com/glfw/glfw
	GIT_TAG 3.4
	GIT_SHALLOW TRUE)

FetchContent_MakeAvailable(freetype glfw)

if(NOT TARGET Freetype::Freetype)
	add_library(Freetype::Freetype ALIAS freetype)
endif()
set(RMLUI_SAMPLES OFF CACHE BOOL "" FORCE)
set(BUILD_TESTING OFF CACHE BOOL "" FORCE)
FetchContent_Declare(rmlui
	GIT_REPOSITORY https://github.com/mikke89/RmlUi
	GIT_TAG 6.3
	GIT_SHALLOW TRUE)
FetchContent_MakeAvailable(rmlui)

set(RMLUI_BACKENDS "${rmlui_SOURCE_DIR}/Backends")
add_library(rmlui_bridge STATIC
	ui.cpp
	"${RMLUI_BACKENDS}/RmlUi_Platform_GLFW.cpp"
	"${RMLUI_BACKENDS}/RmlUi_Renderer_GL3.cpp")
target_include_directories(rmlui_bridge PRIVATE "${RMLUI_BACKENDS}")
find_package(OpenGL REQUIRED)
target_link_libraries(rmlui_bridge PRIVATE RmlUi::RmlUi glfw OpenGL::GL)
