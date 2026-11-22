use std::cell::RefCell;
use std::rc::Rc;

use litehtml::pixbuf::PixbufContainer;
use litehtml::{
    BackgroundLayer, BorderRadiuses, Borders, Color, ConicGradient, DocumentContainer, DrawContext,
    FontDescription, FontHandle, FontMetrics, LinearGradient, ListMarker, MediaFeatures,
    MouseEvent, Position, RadialGradient, Size, TextTransform,
};

pub struct Shared {
    canvas: Rc<RefCell<PixbufContainer>>,
    default_font_name: String,
}

impl Shared {
    pub fn new(canvas: Rc<RefCell<PixbufContainer>>) -> Self {
        let default_font_name = canvas.borrow().default_font_name().to_owned();

        Self {
            canvas,
            default_font_name,
        }
    }
}

impl DocumentContainer for Shared {
    fn create_font(&mut self, descr: &FontDescription) -> (FontHandle, FontMetrics) {
        self.canvas.borrow_mut().create_font(descr)
    }

    fn delete_font(&mut self, font: FontHandle) {
        self.canvas.borrow_mut().delete_font(font);
    }

    fn text_width(&self, text: &str, font: FontHandle) -> f32 {
        self.canvas.borrow().text_width(text, font)
    }

    fn draw_text(
        &mut self,
        hdc: DrawContext,
        text: &str,
        font: FontHandle,
        color: Color,
        pos: Position,
    ) {
        self.canvas
            .borrow_mut()
            .draw_text(hdc, text, font, color, pos);
    }

    fn pt_to_px(&self, pt: f32) -> f32 {
        self.canvas.borrow().pt_to_px(pt)
    }

    fn default_font_size(&self) -> f32 {
        self.canvas.borrow().default_font_size()
    }

    fn default_font_name(&self) -> &str {
        &self.default_font_name
    }

    fn draw_list_marker(&mut self, hdc: DrawContext, marker: &ListMarker) {
        self.canvas.borrow_mut().draw_list_marker(hdc, marker);
    }

    fn load_image(&mut self, src: &str, baseurl: &str, redraw_on_ready: bool) {
        self.canvas
            .borrow_mut()
            .load_image(src, baseurl, redraw_on_ready);
    }

    fn get_image_size(&self, src: &str, baseurl: &str) -> Size {
        let canvas = self.canvas.borrow();
        let size = canvas.get_image_size(src, baseurl);
        let scale = canvas.scale_factor();

        Size {
            width: size.width / scale,
            height: size.height / scale,
        }
    }

    fn draw_image(&mut self, hdc: DrawContext, layer: &BackgroundLayer, url: &str, base_url: &str) {
        self.canvas
            .borrow_mut()
            .draw_image(hdc, layer, url, base_url);
    }

    fn draw_solid_fill(&mut self, hdc: DrawContext, layer: &BackgroundLayer, color: Color) {
        self.canvas.borrow_mut().draw_solid_fill(hdc, layer, color);
    }

    fn draw_linear_gradient(
        &mut self,
        hdc: DrawContext,
        layer: &BackgroundLayer,
        gradient: &LinearGradient,
    ) {
        self.canvas
            .borrow_mut()
            .draw_linear_gradient(hdc, layer, gradient);
    }

    fn draw_radial_gradient(
        &mut self,
        hdc: DrawContext,
        layer: &BackgroundLayer,
        gradient: &RadialGradient,
    ) {
        self.canvas
            .borrow_mut()
            .draw_radial_gradient(hdc, layer, gradient);
    }

    fn draw_conic_gradient(
        &mut self,
        hdc: DrawContext,
        layer: &BackgroundLayer,
        gradient: &ConicGradient,
    ) {
        self.canvas
            .borrow_mut()
            .draw_conic_gradient(hdc, layer, gradient);
    }

    fn draw_borders(
        &mut self,
        hdc: DrawContext,
        borders: &Borders,
        draw_pos: Position,
        root: bool,
    ) {
        self.canvas
            .borrow_mut()
            .draw_borders(hdc, borders, draw_pos, root);
    }

    fn set_caption(&mut self, caption: &str) {
        self.canvas.borrow_mut().set_caption(caption);
    }

    fn set_base_url(&mut self, base_url: &str) {
        self.canvas.borrow_mut().set_base_url(base_url);
    }

    fn link(&mut self) {
        self.canvas.borrow_mut().link();
    }

    fn on_anchor_click(&mut self, url: &str) {
        self.canvas.borrow_mut().on_anchor_click(url);
    }

    fn on_mouse_event(&mut self, event: MouseEvent) {
        self.canvas.borrow_mut().on_mouse_event(event);
    }

    fn set_cursor(&mut self, cursor: &str) {
        self.canvas.borrow_mut().set_cursor(cursor);
    }

    fn transform_text(&self, text: &str, tt: TextTransform) -> String {
        self.canvas.borrow().transform_text(text, tt)
    }

    fn import_css(&self, url: &str, baseurl: &str) -> (String, Option<String>) {
        self.canvas.borrow().import_css(url, baseurl)
    }

    fn set_clip(&mut self, pos: Position, radius: BorderRadiuses) {
        self.canvas.borrow_mut().set_clip(pos, radius);
    }

    fn del_clip(&mut self) {
        self.canvas.borrow_mut().del_clip();
    }

    fn get_viewport(&self) -> Position {
        self.canvas.borrow().get_viewport()
    }

    fn get_media_features(&self) -> MediaFeatures {
        self.canvas.borrow().get_media_features()
    }

    fn get_language(&self) -> (String, String) {
        self.canvas.borrow().get_language()
    }
}
