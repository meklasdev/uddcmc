pub mod button;
pub mod label;
pub mod panel;

use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub mod hud;
pub mod module_btn;
pub mod slider;
pub mod toggle;
pub mod window;

pub use button::Button;
pub use hud::HudWidget;
pub use label::Label;
pub use module_btn::ModuleButton;
pub use panel::Panel;
pub use slider::Slider;
pub use toggle::Toggle;
pub use window::Window;

pub enum Widget {
    Button(Button),
    Label(Label),
    Panel(Panel),
    Window(Window),
    ModuleButton(ModuleButton),
    Slider(Slider),
    Toggle(Toggle),
}

impl Widget {
    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        match self {
            Widget::Button(b) => b.draw(renderer, theme, scale_f),
            Widget::Label(l) => l.draw(renderer, theme, scale_f),
            Widget::Panel(p) => p.draw(renderer, theme, scale_f),
            Widget::Window(w) => w.draw(renderer, theme, scale_f),
            Widget::ModuleButton(m) => m.draw(renderer, theme, scale_f),
            Widget::Slider(s) => s.draw(renderer, theme, scale_f),
            Widget::Toggle(t) => t.draw(renderer, theme, scale_f),
        }
    }

    pub fn handle_click(
        &mut self,
        mx: f32,
        my: f32,
        left_clicked: bool,
        right_clicked: bool,
        scale_f: f32,
    ) -> bool {
        match self {
            Widget::Button(b) => b.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::Label(l) => l.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::Panel(p) => p.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::Window(w) => w.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::ModuleButton(m) => m.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::Slider(s) => s.handle_click(mx, my, left_clicked, right_clicked, scale_f),
            Widget::Toggle(t) => t.handle_click(mx, my, left_clicked, right_clicked, scale_f),
        }
    }

    pub fn update(&mut self, mx: f32, my: f32, left_down: bool, scale_f: f32) {
        match self {
            Widget::Button(b) => b.update(mx, my, left_down, scale_f),
            Widget::Label(l) => l.update(mx, my, left_down, scale_f),
            Widget::Panel(p) => p.update(mx, my, left_down, scale_f),
            Widget::Window(w) => w.update(mx, my, left_down, scale_f),
            Widget::ModuleButton(m) => m.update(mx, my, left_down, scale_f),
            Widget::Slider(s) => s.update(mx, my, left_down, scale_f),
            Widget::Toggle(t) => t.update(mx, my, left_down, scale_f),
        }
    }
}
