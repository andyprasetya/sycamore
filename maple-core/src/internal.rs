//! Internal DOM manipulation utilities. Generated by the `template!` macro. Should not be used directly.

use std::cell::RefCell;

use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Element, HtmlElement, Node, Text};

use crate::reactive::*;

/// Create a new [`HtmlElement`] with the specified tag.
pub fn element(tag: &str) -> HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element(tag)
        .unwrap()
        .dyn_into()
        .unwrap()
}

pub fn text(value: impl Fn() -> String + 'static) -> Text {
    let text_node = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_text_node("");

    create_effect({
        let text_node = text_node.clone();
        move || {
            text_node.set_text_content(Some(&value()));
        }
    });

    text_node
}

/// Sets an attribute on an [`HtmlElement`].
pub fn attr(element: &HtmlElement, name: &str, value: &str) {
    element.set_attribute(name, value).unwrap();
}

thread_local! {
    /// A global event listener pool to prevent [`Closure`]s from being deallocated.
    /// TODO: remove events when elements are detached.
    static EVENT_LISTENERS: RefCell<Vec<Closure<dyn Fn()>>> = RefCell::new(Vec::new());
}

/// Sets an event listener on an [`HtmlElement`].
pub fn event(element: &HtmlElement, name: &str, handler: Box<dyn Fn()>) {
    let closure = Closure::wrap(handler);
    element
        .add_event_listener_with_callback(name, closure.as_ref().unchecked_ref())
        .unwrap();

    EVENT_LISTENERS.with(|event_listeners| event_listeners.borrow_mut().push(closure));
}

/// Appends a child node to an element.
pub fn append(element: &Element, child: &Node) {
    element.append_with_node_1(&child).unwrap();
}