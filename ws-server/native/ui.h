#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ui_host {
	int (*read_file)(const char* path, const uint8_t** data, size_t* len);
	void (*release_file)(const uint8_t* data, size_t len);
	int (*load_image)(const char* path, uint8_t** rgba, size_t* len, int* width, int* height, uint32_t* anim_id);
	void (*free_image)(uint8_t* rgba, size_t len);
	int (*anim_frame)(uint32_t anim_id, double now, const uint8_t** rgba, size_t* len, double* next_in);
	void (*anim_release)(uint32_t anim_id);
} ui_host;

enum ui_event_kind {
	UI_EVENT_CLICK = 1,
	UI_EVENT_CHANGE = 2,
	UI_EVENT_BLUR = 3,
	UI_EVENT_KEY_DOWN = 4,
	UI_EVENT_MOUSE_DOWN = 5,
	UI_EVENT_WHEEL = 6,
	UI_EVENT_CLOSE = 7,
};

typedef struct ui_event {
	int window;
	int kind;
	int value;
	char id[64];
} ui_event;

int ui_init(const ui_host* host);
void ui_shutdown(void);
int ui_load_font(const char* path, const char* family, int weight, int fallback);

int ui_open_window(const char* title, int width, int height, int resizable, const char* document,
	const uint8_t* icon_rgba, int icon_width, int icon_height);
void ui_close_window(int window);
int ui_window_exists(int window);
void ui_minimize_window(int window);
void ui_focus_window(int window);
void ui_set_dark_theme(int dark);

void ui_set_inner_rml(int window, const char* id, const char* rml);
void ui_set_attribute(int window, const char* id, const char* name, const char* value);
void ui_remove_attribute(int window, const char* id, const char* name);
int ui_has_attribute(int window, const char* id, const char* name);
void ui_set_class(int window, const char* id, const char* class_name, int active);
void ui_set_property(int window, const char* id, const char* name, const char* value);
void ui_remove_property(int window, const char* id, const char* name);
size_t ui_get_value(int window, const char* id, char* out, size_t cap);
void ui_set_value(int window, const char* id, const char* value);
void ui_select_clear(int window, const char* id);
void ui_select_add(int window, const char* id, const char* label, const char* value);
void ui_blur(int window);

int ui_poll_event(ui_event* out);
void ui_frame(double max_wait);
void ui_wake(void);
void ui_set_clipboard(const char* text);

#ifdef __cplusplus
}
#endif
