#include "ui.h"

#include "RmlUi_Include_GL3.h"
#define GLFW_INCLUDE_NONE
#include "RmlUi_Platform_GLFW.h"
#include "RmlUi_Renderer_GL3.h"
#include <GLFW/glfw3.h>
#include <RmlUi/Core.h>
#include <RmlUi/Core/Elements/ElementFormControl.h>
#include <RmlUi/Core/Elements/ElementFormControlSelect.h>

#include <algorithm>
#include <cstring>
#include <deque>
#include <limits>
#include <memory>
#include <unordered_map>
#include <vector>

static ui_host g_host = {};
static std::deque<ui_event> g_events;
static bool g_dark = false;
static bool g_gl_loaded = false;
static double g_last_frame = 0;
static GLFWwindow* g_cursor_window = nullptr;

static void PushEvent(int window, int kind, int value, Rml::Element* element)
{
	ui_event ev = {};
	ev.window = window;
	ev.kind = kind;
	ev.value = value;
	while (element && element->GetId().empty())
		element = element->GetParentNode();
	if (element)
	{
		const Rml::String& id = element->GetId();
		size_t n = std::min(id.size(), sizeof(ev.id) - 1);
		memcpy(ev.id, id.data(), n);
	}
	g_events.push_back(ev);
}

class HostFileInterface : public Rml::FileInterface {
	struct Handle {
		const uint8_t* data;
		size_t len;
		size_t pos;
	};

public:
	Rml::FileHandle Open(const Rml::String& path) override
	{
		const uint8_t* data = nullptr;
		size_t len = 0;
		if (!g_host.read_file || !g_host.read_file(path.c_str(), &data, &len))
			return 0;
		return reinterpret_cast<Rml::FileHandle>(new Handle{data, len, 0});
	}
	void Close(Rml::FileHandle file) override
	{
		Handle* h = reinterpret_cast<Handle*>(file);
		g_host.release_file(h->data, h->len);
		delete h;
	}
	size_t Read(void* buffer, size_t size, Rml::FileHandle file) override
	{
		Handle* h = reinterpret_cast<Handle*>(file);
		size_t n = std::min(size, h->len - h->pos);
		memcpy(buffer, h->data + h->pos, n);
		h->pos += n;
		return n;
	}
	bool Seek(Rml::FileHandle file, long offset, int origin) override
	{
		Handle* h = reinterpret_cast<Handle*>(file);
		long base = origin == SEEK_SET ? 0 : origin == SEEK_CUR ? long(h->pos) : long(h->len);
		long target = base + offset;
		if (target < 0 || size_t(target) > h->len)
			return false;
		h->pos = size_t(target);
		return true;
	}
	size_t Tell(Rml::FileHandle file) override { return reinterpret_cast<Handle*>(file)->pos; }
	size_t Length(Rml::FileHandle file) override { return reinterpret_cast<Handle*>(file)->len; }
};

class HostSystemInterface : public Rml::SystemInterface {
public:
	double GetElapsedTime() override { return glfwGetTime(); }

	void SetMouseCursor(const Rml::String& name) override
	{
		if (!g_cursor_window)
			return;
		int shape = 0;
		if (name == "pointer")
			shape = GLFW_HAND_CURSOR;
		else if (name == "text")
			shape = GLFW_IBEAM_CURSOR;
		else if (name == "move")
			shape = GLFW_RESIZE_ALL_CURSOR;
		glfwSetCursor(g_cursor_window, shape ? Cursor(shape) : nullptr);
	}

	void SetClipboardText(const Rml::String& text) override { glfwSetClipboardString(nullptr, text.c_str()); }
	void GetClipboardText(Rml::String& text) override
	{
		const char* s = glfwGetClipboardString(nullptr);
		text = s ? s : "";
	}

	static GLFWcursor* Cursor(int shape)
	{
		static std::unordered_map<int, GLFWcursor*> cursors;
		auto it = cursors.find(shape);
		if (it != cursors.end())
			return it->second;
		return cursors[shape] = glfwCreateStandardCursor(shape);
	}
};

static HostFileInterface g_file_interface;
static HostSystemInterface g_system_interface;

class UiRenderer : public RenderInterface_GL3 {
	struct Anim {
		uint32_t id;
		int width, height;
		double next_due;
		uint64_t drawn_frame;
	};
	std::unordered_map<Rml::TextureHandle, Anim> anims;

public:
	uint64_t frame_no = 1;

	Rml::TextureHandle LoadTexture(Rml::Vector2i& dimensions, const Rml::String& source) override
	{
		uint8_t* rgba = nullptr;
		size_t len = 0;
		int width = 0, height = 0;
		uint32_t anim_id = 0;
		if (!g_host.load_image || !g_host.load_image(source.c_str(), &rgba, &len, &width, &height, &anim_id))
		{
			Rml::Log::Message(Rml::Log::LT_WARNING, "Could not load image '%s'", source.c_str());
			return 0;
		}
		dimensions = {width, height};
		Rml::TextureHandle handle = GenerateTexture({reinterpret_cast<const Rml::byte*>(rgba), len}, dimensions);
		g_host.free_image(rgba, len);
		if (handle && anim_id)
			anims[handle] = Anim{anim_id, width, height, 0.0, frame_no};
		else if (anim_id)
			g_host.anim_release(anim_id);
		return handle;
	}

	void ReleaseTexture(Rml::TextureHandle handle) override
	{
		auto it = anims.find(handle);
		if (it != anims.end())
		{
			g_host.anim_release(it->second.id);
			anims.erase(it);
		}
		RenderInterface_GL3::ReleaseTexture(handle);
	}

	void RenderGeometry(Rml::CompiledGeometryHandle geometry, Rml::Vector2f translation, Rml::TextureHandle texture) override
	{
		auto it = anims.find(texture);
		if (it != anims.end())
			it->second.drawn_frame = frame_no;
		RenderInterface_GL3::RenderGeometry(geometry, translation, texture);
	}

	double AdvanceAnimations(double now)
	{
		double next = std::numeric_limits<double>::infinity();
		for (auto& [handle, anim] : anims)
		{
			if (anim.drawn_frame + 1 < frame_no)
				continue;
			if (now >= anim.next_due)
			{
				const uint8_t* rgba = nullptr;
				size_t len = 0;
				double next_in = 0.1;
				if (g_host.anim_frame(anim.id, now, &rgba, &len, &next_in) && len == size_t(anim.width) * anim.height * 4)
				{
					glBindTexture(GL_TEXTURE_2D, GLuint(handle));
					glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, anim.width, anim.height, GL_RGBA, GL_UNSIGNED_BYTE, rgba);
					glBindTexture(GL_TEXTURE_2D, 0);
				}
				anim.next_due = now + std::max(next_in, 0.01);
			}
			next = std::min(next, anim.next_due - now);
		}
		return next;
	}
};

struct UiWindow;
static std::vector<std::unique_ptr<UiWindow>> g_windows;
static int g_next_window_id = 1;

class WindowListener : public Rml::EventListener {
public:
	UiWindow* window = nullptr;
	void ProcessEvent(Rml::Event& event) override;
};

struct UiWindow {
	int id = 0;
	GLFWwindow* glfw = nullptr;
	std::unique_ptr<UiRenderer> renderer;
	Rml::Context* context = nullptr;
	Rml::ElementDocument* document = nullptr;
	WindowListener listener;
	bool resizable = false;
	bool dims_dirty = true;
	int modifiers = 0;
	double next_anim_in = std::numeric_limits<double>::infinity();

	bool dragging = false;
	double drag_x = 0, drag_y = 0;

	int hover_edges = 0;
	int resize_edges = 0;
	double resize_cursor_x = 0, resize_cursor_y = 0;
	int resize_x = 0, resize_y = 0, resize_w = 0, resize_h = 0;
};

static constexpr int EDGE_LEFT = 1, EDGE_RIGHT = 2, EDGE_TOP = 4, EDGE_BOTTOM = 8;
static constexpr int RESIZE_BORDER = 5;
static constexpr int MIN_WIDTH = 300, MIN_HEIGHT = 300;

void WindowListener::ProcessEvent(Rml::Event& event)
{
	Rml::Element* target = event.GetTargetElement();
	switch (event.GetId())
	{
	case Rml::EventId::Click: PushEvent(window->id, UI_EVENT_CLICK, 0, target); break;
	case Rml::EventId::Change: PushEvent(window->id, UI_EVENT_CHANGE, 0, target); break;
	case Rml::EventId::Blur: PushEvent(window->id, UI_EVENT_BLUR, 0, target); break;
	case Rml::EventId::Mousedown:
		if (event.GetParameter<int>("button", 0) != 0 || window->hover_edges)
			break;
		for (Rml::Element* e = target; e; e = e->GetParentNode())
		{
			if (e->GetTagName() == "button")
				break;
			if (e->HasAttribute("data-drag-region"))
			{
				window->dragging = true;
				glfwGetCursorPos(window->glfw, &window->drag_x, &window->drag_y);
				break;
			}
		}
		break;
	default: break;
	}
}

static UiWindow* FindWindow(int id)
{
	for (auto& w : g_windows)
		if (w->id == id)
			return w.get();
	return nullptr;
}

static UiWindow* FromGlfw(GLFWwindow* glfw)
{
	return static_cast<UiWindow*>(glfwGetWindowUserPointer(glfw));
}

static int EdgesAt(UiWindow* w, double x, double y)
{
	if (!w->resizable)
		return 0;
	int width, height;
	glfwGetWindowSize(w->glfw, &width, &height);
	int edges = 0;
	if (x < RESIZE_BORDER)
		edges |= EDGE_LEFT;
	else if (x >= width - RESIZE_BORDER)
		edges |= EDGE_RIGHT;
	if (y < RESIZE_BORDER)
		edges |= EDGE_TOP;
	else if (y >= height - RESIZE_BORDER)
		edges |= EDGE_BOTTOM;
	return edges;
}

static void SetEdgeCursor(UiWindow* w, int edges)
{
	int shape = 0;
	if ((edges & (EDGE_LEFT | EDGE_TOP)) == (EDGE_LEFT | EDGE_TOP) || (edges & (EDGE_RIGHT | EDGE_BOTTOM)) == (EDGE_RIGHT | EDGE_BOTTOM))
		shape = GLFW_RESIZE_NWSE_CURSOR;
	else if ((edges & (EDGE_RIGHT | EDGE_TOP)) == (EDGE_RIGHT | EDGE_TOP) || (edges & (EDGE_LEFT | EDGE_BOTTOM)) == (EDGE_LEFT | EDGE_BOTTOM))
		shape = GLFW_RESIZE_NESW_CURSOR;
	else if (edges & (EDGE_LEFT | EDGE_RIGHT))
		shape = GLFW_RESIZE_EW_CURSOR;
	else if (edges & (EDGE_TOP | EDGE_BOTTOM))
		shape = GLFW_RESIZE_NS_CURSOR;
	glfwSetCursor(w->glfw, shape ? HostSystemInterface::Cursor(shape) : nullptr);
}

static void SetupCallbacks(GLFWwindow* glfw)
{
	glfwSetKeyCallback(glfw, [](GLFWwindow* gw, int key, int, int action, int mods) {
		UiWindow* w = FromGlfw(gw);
		w->modifiers = mods;
		g_cursor_window = gw;
		if (action == GLFW_PRESS)
			PushEvent(w->id, UI_EVENT_KEY_DOWN, key, nullptr);
		RmlGLFW::ProcessKeyCallback(w->context, key, action, mods);
	});
	glfwSetCharCallback(glfw, [](GLFWwindow* gw, unsigned int codepoint) { RmlGLFW::ProcessCharCallback(FromGlfw(gw)->context, codepoint); });
	glfwSetCursorEnterCallback(glfw, [](GLFWwindow* gw, int entered) {
		UiWindow* w = FromGlfw(gw);
		if (entered)
			g_cursor_window = gw;
		RmlGLFW::ProcessCursorEnterCallback(w->context, entered);
	});
	glfwSetCursorPosCallback(glfw, [](GLFWwindow* gw, double x, double y) {
		UiWindow* w = FromGlfw(gw);
		g_cursor_window = gw;
		if (!w->resize_edges && !w->dragging)
		{
			int edges = EdgesAt(w, x, y);
			if (edges != w->hover_edges || edges)
				SetEdgeCursor(w, edges);
			w->hover_edges = edges;
		}
		RmlGLFW::ProcessCursorPosCallback(w->context, gw, x, y, w->modifiers);
	});
	glfwSetMouseButtonCallback(glfw, [](GLFWwindow* gw, int button, int action, int mods) {
		UiWindow* w = FromGlfw(gw);
		w->modifiers = mods;
		g_cursor_window = gw;
		if (action == GLFW_PRESS && button == GLFW_MOUSE_BUTTON_LEFT && w->hover_edges)
		{
			double x, y;
			int wx, wy;
			glfwGetCursorPos(gw, &x, &y);
			glfwGetWindowPos(gw, &wx, &wy);
			w->resize_edges = w->hover_edges;
			w->resize_cursor_x = wx + x;
			w->resize_cursor_y = wy + y;
			w->resize_x = wx;
			w->resize_y = wy;
			glfwGetWindowSize(gw, &w->resize_w, &w->resize_h);
			return;
		}
		if (action == GLFW_PRESS)
			PushEvent(w->id, UI_EVENT_MOUSE_DOWN, button, w->context->GetHoverElement());
		RmlGLFW::ProcessMouseButtonCallback(w->context, button, action, mods);
	});
	glfwSetScrollCallback(glfw, [](GLFWwindow* gw, double, double yoffset) {
		UiWindow* w = FromGlfw(gw);
		PushEvent(w->id, UI_EVENT_WHEEL, yoffset > 0 ? 1 : -1, w->context->GetHoverElement());
		RmlGLFW::ProcessScrollCallback(w->context, yoffset, w->modifiers);
	});
	glfwSetFramebufferSizeCallback(glfw, [](GLFWwindow* gw, int, int) { FromGlfw(gw)->dims_dirty = true; });
	glfwSetWindowContentScaleCallback(glfw, [](GLFWwindow* gw, float, float) { FromGlfw(gw)->dims_dirty = true; });
	glfwSetWindowCloseCallback(glfw, [](GLFWwindow* gw) {
		glfwSetWindowShouldClose(gw, GLFW_FALSE);
		PushEvent(FromGlfw(gw)->id, UI_EVENT_CLOSE, 0, nullptr);
	});
}

static void UpdateDragAndResize(UiWindow* w)
{
	if (w->dragging)
	{
		if (glfwGetMouseButton(w->glfw, GLFW_MOUSE_BUTTON_LEFT) != GLFW_PRESS)
			w->dragging = false;
		else
		{
			double x, y;
			int wx, wy;
			glfwGetCursorPos(w->glfw, &x, &y);
			glfwGetWindowPos(w->glfw, &wx, &wy);
			glfwSetWindowPos(w->glfw, wx + int(x - w->drag_x), wy + int(y - w->drag_y));
		}
	}
	if (w->resize_edges)
	{
		if (glfwGetMouseButton(w->glfw, GLFW_MOUSE_BUTTON_LEFT) != GLFW_PRESS)
		{
			w->resize_edges = 0;
			return;
		}
		double x, y;
		int wx, wy;
		glfwGetCursorPos(w->glfw, &x, &y);
		glfwGetWindowPos(w->glfw, &wx, &wy);
		const int dx = int(wx + x - w->resize_cursor_x), dy = int(wy + y - w->resize_cursor_y);
		int nx = w->resize_x, ny = w->resize_y, nw = w->resize_w, nh = w->resize_h;
		if (w->resize_edges & EDGE_RIGHT)
			nw = std::max(MIN_WIDTH, w->resize_w + dx);
		if (w->resize_edges & EDGE_BOTTOM)
			nh = std::max(MIN_HEIGHT, w->resize_h + dy);
		if (w->resize_edges & EDGE_LEFT)
		{
			nw = std::max(MIN_WIDTH, w->resize_w - dx);
			nx = w->resize_x + w->resize_w - nw;
		}
		if (w->resize_edges & EDGE_TOP)
		{
			nh = std::max(MIN_HEIGHT, w->resize_h - dy);
			ny = w->resize_y + w->resize_h - nh;
		}
		glfwSetWindowPos(w->glfw, nx, ny);
		glfwSetWindowSize(w->glfw, nw, nh);
	}
}

static void RenderWindow(UiWindow* w)
{
	glfwMakeContextCurrent(w->glfw);
	if (w->dims_dirty)
	{
		w->dims_dirty = false;
		int fb_w = 0, fb_h = 0;
		float scale = 1.f;
		glfwGetFramebufferSize(w->glfw, &fb_w, &fb_h);
		glfwGetWindowContentScale(w->glfw, &scale, nullptr);
		w->renderer->SetViewport(fb_w, fb_h);
		w->context->SetDimensions({fb_w, fb_h});
		w->context->SetDensityIndependentPixelRatio(scale);
	}
	w->next_anim_in = w->renderer->AdvanceAnimations(glfwGetTime());
	w->context->Update();
	glClearColor(0, 0, 0, 0);
	glClear(GL_COLOR_BUFFER_BIT);
	w->renderer->BeginFrame();
	w->context->Render();
	w->renderer->EndFrame();
	w->renderer->frame_no++;
}

static void DestroyWindow(UiWindow* w)
{
	glfwMakeContextCurrent(w->glfw);
	Rml::RemoveContext(w->context->GetName());
	Rml::ReleaseRenderManagers();
	w->renderer.reset();
	glfwDestroyWindow(w->glfw);
	if (g_cursor_window == w->glfw)
		g_cursor_window = nullptr;
}

extern "C" int ui_init(const ui_host* host)
{
	g_host = *host;
#if defined(__linux__)
	glfwInitHint(GLFW_PLATFORM, GLFW_PLATFORM_X11);
#endif
	if (!glfwInit())
		return 0;
	Rml::SetSystemInterface(&g_system_interface);
	Rml::SetFileInterface(&g_file_interface);
	return Rml::Initialise() ? 1 : 0;
}

extern "C" void ui_shutdown(void)
{
	for (auto& w : g_windows)
		DestroyWindow(w.get());
	g_windows.clear();
	Rml::Shutdown();
	if (g_gl_loaded)
		RmlGL3::Shutdown();
	glfwTerminate();
}

extern "C" int ui_load_font(const char* path, const char* family, int weight, int fallback)
{
	return Rml::LoadFontFace(path, family, Rml::Style::FontStyle::Normal, static_cast<Rml::Style::FontWeight>(weight), fallback != 0) ? 1 : 0;
}

extern "C" int ui_open_window(const char* title, int width, int height, int resizable, const char* document, const uint8_t* icon_rgba,
	int icon_width, int icon_height)
{
	glfwDefaultWindowHints();
	glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
	glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
	glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
	glfwWindowHint(GLFW_OPENGL_FORWARD_COMPAT, GL_TRUE);
	glfwWindowHint(GLFW_DOUBLEBUFFER, GLFW_TRUE);
	glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);
	glfwWindowHint(GLFW_DECORATED, GLFW_FALSE);
	glfwWindowHint(GLFW_TRANSPARENT_FRAMEBUFFER, GLFW_TRUE);
	glfwWindowHint(GLFW_SCALE_TO_MONITOR, GLFW_TRUE);
	glfwWindowHint(GLFW_VISIBLE, GLFW_FALSE);

	GLFWwindow* glfw = glfwCreateWindow(width, height, title, nullptr, nullptr);
	if (!glfw)
		return 0;
	glfwMakeContextCurrent(glfw);
	glfwSwapInterval(0);

	if (!g_gl_loaded)
	{
		Rml::String message;
		if (!RmlGL3::Initialize(&message))
		{
			glfwDestroyWindow(glfw);
			return 0;
		}
		g_gl_loaded = true;
	}

	auto w = std::make_unique<UiWindow>();
	w->id = g_next_window_id++;
	w->glfw = glfw;
	w->resizable = resizable != 0;
	w->renderer = std::make_unique<UiRenderer>();
	if (!*w->renderer)
	{
		glfwDestroyWindow(glfw);
		return 0;
	}
	w->listener.window = w.get();
	glfwSetWindowUserPointer(glfw, w.get());

	int fb_w = 0, fb_h = 0;
	glfwGetFramebufferSize(glfw, &fb_w, &fb_h);
	w->renderer->SetViewport(fb_w, fb_h);
	w->context = Rml::CreateContext("window" + Rml::ToString(w->id), {fb_w, fb_h}, w->renderer.get());
	if (!w->context)
	{
		glfwDestroyWindow(glfw);
		return 0;
	}
	w->context->ActivateTheme("dark", g_dark);

	w->document = w->context->LoadDocument(document);
	if (!w->document)
	{
		Rml::RemoveContext(w->context->GetName());
		glfwDestroyWindow(glfw);
		return 0;
	}
	w->document->AddEventListener(Rml::EventId::Click, &w->listener);
	w->document->AddEventListener(Rml::EventId::Change, &w->listener);
	w->document->AddEventListener(Rml::EventId::Mousedown, &w->listener);
	w->document->AddEventListener(Rml::EventId::Blur, &w->listener, true);
	w->document->Show();

	if (icon_rgba && icon_width > 0 && icon_height > 0)
	{
		GLFWimage image = {icon_width, icon_height, const_cast<unsigned char*>(icon_rgba)};
		glfwSetWindowIcon(glfw, 1, &image);
	}

	if (GLFWmonitor* monitor = glfwGetPrimaryMonitor())
	{
		int mx, my, mw, mh, ww, wh;
		glfwGetMonitorWorkarea(monitor, &mx, &my, &mw, &mh);
		glfwGetWindowSize(glfw, &ww, &wh);
		glfwSetWindowPos(glfw, mx + (mw - ww) / 2, my + (mh - wh) / 2);
	}

	glfwSetInputMode(glfw, GLFW_LOCK_KEY_MODS, GLFW_TRUE);
	SetupCallbacks(glfw);
	UiWindow* raw = w.get();
	g_windows.push_back(std::move(w));
	RenderWindow(raw);
	glfwSwapBuffers(glfw);
	glfwShowWindow(glfw);
	glfwFocusWindow(glfw);
	return raw->id;
}

extern "C" void ui_close_window(int window)
{
	for (auto it = g_windows.begin(); it != g_windows.end(); ++it)
	{
		if ((*it)->id == window)
		{
			DestroyWindow(it->get());
			g_windows.erase(it);
			return;
		}
	}
}

extern "C" int ui_window_exists(int window)
{
	return FindWindow(window) ? 1 : 0;
}

extern "C" void ui_minimize_window(int window)
{
	if (UiWindow* w = FindWindow(window))
		glfwIconifyWindow(w->glfw);
}

extern "C" void ui_focus_window(int window)
{
	if (UiWindow* w = FindWindow(window))
	{
		glfwRestoreWindow(w->glfw);
		glfwShowWindow(w->glfw);
		glfwFocusWindow(w->glfw);
	}
}

extern "C" void ui_set_dark_theme(int dark)
{
	g_dark = dark != 0;
	for (auto& w : g_windows)
		w->context->ActivateTheme("dark", g_dark);
}

static Rml::Element* ById(int window, const char* id)
{
	UiWindow* w = FindWindow(window);
	return w && w->document ? w->document->GetElementById(id) : nullptr;
}

extern "C" void ui_set_inner_rml(int window, const char* id, const char* rml)
{
	if (Rml::Element* e = ById(window, id))
		e->SetInnerRML(rml);
}

extern "C" void ui_set_attribute(int window, const char* id, const char* name, const char* value)
{
	if (Rml::Element* e = ById(window, id))
		e->SetAttribute(name, Rml::String(value));
}

extern "C" void ui_remove_attribute(int window, const char* id, const char* name)
{
	if (Rml::Element* e = ById(window, id))
		e->RemoveAttribute(name);
}

extern "C" int ui_has_attribute(int window, const char* id, const char* name)
{
	Rml::Element* e = ById(window, id);
	return e && e->HasAttribute(name) ? 1 : 0;
}

extern "C" void ui_set_class(int window, const char* id, const char* class_name, int active)
{
	if (Rml::Element* e = ById(window, id))
		e->SetClass(class_name, active != 0);
}

extern "C" void ui_set_property(int window, const char* id, const char* name, const char* value)
{
	if (Rml::Element* e = ById(window, id))
		e->SetProperty(name, value);
}

extern "C" void ui_remove_property(int window, const char* id, const char* name)
{
	if (Rml::Element* e = ById(window, id))
		e->RemoveProperty(name);
}

extern "C" size_t ui_get_value(int window, const char* id, char* out, size_t cap)
{
	auto* control = rmlui_dynamic_cast<Rml::ElementFormControl*>(ById(window, id));
	if (!control || cap == 0)
		return 0;
	Rml::String value = control->GetValue();
	size_t n = std::min(value.size(), cap - 1);
	memcpy(out, value.data(), n);
	out[n] = 0;
	return value.size();
}

extern "C" void ui_set_value(int window, const char* id, const char* value)
{
	if (auto* control = rmlui_dynamic_cast<Rml::ElementFormControl*>(ById(window, id)))
		control->SetValue(value);
}

extern "C" void ui_select_clear(int window, const char* id)
{
	if (auto* select = rmlui_dynamic_cast<Rml::ElementFormControlSelect*>(ById(window, id)))
		select->RemoveAll();
}

extern "C" void ui_select_add(int window, const char* id, const char* label, const char* value)
{
	if (auto* select = rmlui_dynamic_cast<Rml::ElementFormControlSelect*>(ById(window, id)))
		select->Add(label, value);
}

extern "C" void ui_blur(int window)
{
	if (UiWindow* w = FindWindow(window))
		if (Rml::Element* focus = w->context->GetFocusElement())
			focus->Blur();
}

extern "C" int ui_poll_event(ui_event* out)
{
	if (g_events.empty())
		return 0;
	*out = g_events.front();
	g_events.pop_front();
	return 1;
}

extern "C" void ui_frame(double max_wait)
{
	double wait = max_wait;
	bool busy = false;
	for (auto& w : g_windows)
	{
		busy = busy || w->dragging || w->resize_edges;
		wait = std::min(wait, w->context->GetNextUpdateDelay());
		wait = std::min(wait, w->next_anim_in);
	}
	const double since = glfwGetTime() - g_last_frame;
	wait = busy ? 1.0 / 60.0 - since : std::max(wait, 1.0 / 30.0 - since);
	if (wait > 0.0)
		glfwWaitEventsTimeout(wait);
	else
		glfwPollEvents();

	for (auto& w : g_windows)
	{
		UpdateDragAndResize(w.get());
		RenderWindow(w.get());
		glfwSwapBuffers(w->glfw);
	}
	g_last_frame = glfwGetTime();
}

extern "C" void ui_wake(void)
{
	glfwPostEmptyEvent();
}

extern "C" void ui_set_clipboard(const char* text)
{
	glfwSetClipboardString(nullptr, text);
}
