//! This is corresponding to a page.
//!
//! https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/frame/
//! https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/frame/local_frame_view.h

use crate::alloc::string::ToString;
use crate::browser::Browser;
use crate::display_item::DisplayItem;
use crate::http::HttpResponse;
use crate::log::Log;
use crate::log::LogLevel;
use crate::renderer::css::cssom::CssParser;
use crate::renderer::css::cssom::StyleSheet;
use crate::renderer::css::token::CssTokenizer;
use crate::renderer::dom::api::{get_js_content, get_style_content};
use crate::renderer::html::dom::HtmlParser;
use crate::renderer::html::dom::Node;
use crate::renderer::html::dom::Window;
use crate::renderer::html::html_builder::dom_to_html;
use crate::renderer::html::token::HtmlTokenizer;
use crate::renderer::js::ast::JsParser;
use crate::renderer::js::runtime::JsRuntime;
use crate::renderer::js::token::JsLexer;
use crate::renderer::layout::layout_view::LayoutView;
use alloc::rc::{Rc, Weak};
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

#[derive(Debug, Clone)]
struct Subresource {
    src: String,
    resource: String,
}

impl Subresource {
    fn new(src: String) -> Self {
        Self {
            src,
            resource: String::new(),
        }
    }
}

/// Represents a page.
#[derive(Debug, Clone)]
pub struct Page {
    browser: Weak<RefCell<Browser>>,
    url: Option<String>,
    window: Option<Rc<RefCell<Window>>>,
    style: Option<StyleSheet>,
    layout_view: Option<LayoutView>,
    subresources: Vec<Subresource>,
    display_items: Vec<DisplayItem>,
    logs: Vec<Log>,
    modified: bool,
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}

impl Page {
    pub fn new() -> Self {
        Self {
            browser: Weak::new(),
            url: None,
            window: None,
            style: None,
            layout_view: None,
            subresources: Vec::new(),
            display_items: Vec::new(),
            logs: Vec::new(),
            modified: false,
        }
    }

    pub fn receive_response(&mut self, response: HttpResponse) {
        self.console_debug("receive_response start".to_string());

        self.set_window(response.body());
        self.set_style();

        self.execute_js();

        while self.modified {
            let dom = match &self.window {
                Some(window) => window.borrow().document(),
                None => {
                    self.set_layout_view();
                    return;
                }
            };

            let modified_html = dom_to_html(&Some(dom));

            self.set_window(modified_html);
            self.set_style();

            self.modified = false;

            self.execute_js();
        }

        self.set_layout_view();

        self.paint_tree();
    }

    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    pub fn set_browser(&mut self, browser: Weak<RefCell<Browser>>) {
        self.browser = browser;
    }

    fn set_window(&mut self, html: String) {
        let html_tokenizer = HtmlTokenizer::new(html);

        let window = HtmlParser::new(self.browser.clone(), html_tokenizer).construct_tree();
        self.window = Some(window);
    }

    fn set_style(&mut self) {
        let dom = match &self.window {
            Some(window) => window.borrow().document(),
            None => return,
        };

        let style = get_style_content(dom);
        let css_tokenizer = CssTokenizer::new(style);
        let cssom = CssParser::new(self.browser.clone(), css_tokenizer).parse_stylesheet();
        self.style = Some(cssom);
    }

    fn set_layout_view(&mut self) {
        let dom = match &self.window {
            Some(window) => window.borrow().document(),
            None => return,
        };

        let style = match self.style.clone() {
            Some(style) => style,
            None => return,
        };

        let layout_view = LayoutView::new(self.browser.clone(), dom, &style);
        self.layout_view = Some(layout_view);
    }

    fn execute_js(&mut self) {
        let dom = match &self.window {
            Some(window) => window.borrow().document(),
            None => return,
        };

        let js = get_js_content(dom.clone());
        let lexer = JsLexer::new(js);

        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();

        let url = match self.url.clone() {
            Some(url) => url,
            None => return,
        };

        let mut runtime = JsRuntime::new(dom, url);
        runtime.execute(&ast);

        self.modified = runtime.dom_modified();
    }

    pub fn dom_root(&self) -> Option<Rc<RefCell<Node>>> {
        match &self.window {
            Some(window) => Some(window.borrow().document()),
            None => None,
        }
    }

    pub fn style(&self) -> Option<StyleSheet> {
        self.style.clone()
    }

    pub fn push_url_for_subresource(&mut self, src: String) {
        // TODO: send a request to url and get a resource.
        self.subresources.push(Subresource::new(src));
    }

    pub fn subresource(&self, src: String) -> String {
        for s in &self.subresources {
            if s.src == src {
                return s.resource.clone();
            }
        }
        String::new()
    }

    pub fn display_items(&self) -> Vec<DisplayItem> {
        self.display_items.clone()
    }

    pub fn clear_display_items(&mut self) {
        self.display_items = Vec::new();
    }

    pub fn logs(&self) -> Vec<Log> {
        self.logs.clone()
    }

    pub fn clear_logs(&mut self) {
        self.logs = Vec::new();
    }

    pub fn console_debug(&mut self, log: String) {
        self.logs.push(Log::new(LogLevel::Debug, log));
    }

    pub fn console_warning(&mut self, log: String) {
        self.logs.push(Log::new(LogLevel::Warning, log));
    }

    pub fn console_error(&mut self, log: String) {
        self.logs.push(Log::new(LogLevel::Error, log));
    }

    /// https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/frame/local_frame_view.h;drc=0e9a0b6e9bb6ec59521977eec805f5d0bca833e0;bpv=1;bpt=1;l=907
    fn paint_tree(&mut self) {
        if let Some(layout_view) = &self.layout_view {
            self.display_items = layout_view.paint();
        }
    }
}
