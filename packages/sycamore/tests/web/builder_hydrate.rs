//! Tests for using builder API together with hydration.

use expect_test::{expect, Expect};
use sycamore::builder::html::*;

use super::*;

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

mod hello_world {
    use super::*;
    fn v<G: Html>() -> View<G> {
        p().text("Hello World!").build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(v),
            expect![[r#"<p data-hk="0.0">Hello World!</p>"#]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(v);
        let c = test_container();
        c.set_inner_html(&html);

        sycamore::hydrate_to(v, &c);

        // Hydration should not change inner html.
        assert_eq!(c.inner_html(), html);
    }
}

mod hydrate_recursive {
    use super::*;
    fn v<G: Html>() -> View<G> {
        div().child(p().text("Nested").build()).build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(v),
            expect![[r#"<div data-hk="0.0"><p data-hk="0.1">Nested</p></div>"#]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(v);
        let c = test_container();
        c.set_inner_html(&html);

        sycamore::hydrate_to(v, &c);

        // Hydration should not change inner html.
        assert_eq!(c.inner_html(), html);
    }
}

mod multiple_nodes_at_same_depth {
    use super::*;
    fn v<G: Html>() -> View<G> {
        div()
            .child(p().text("First").build())
            .child(p().text("Second").build())
            .build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(v),
            expect![[
                r#"<div data-hk="0.0"><p data-hk="0.1">First</p><p data-hk="0.2">Second</p></div>"#
            ]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(v);
        let c = test_container();
        c.set_inner_html(&html);

        sycamore::hydrate_to(v, &c);

        // Hydration should not change inner html.
        assert_eq!(c.inner_html(), html);
    }
}

mod top_level_fragment {
    use super::*;
    fn v<G: Html>() -> View<G> {
        fragment([p().text("First").build(), p().text("Second").build()])
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(v),
            expect![[r#"<p data-hk="0.0">First</p><p data-hk="0.1">Second</p>"#]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(v);
        let c = test_container();
        c.set_inner_html(&html);

        sycamore::hydrate_to(v, &c);

        // Hydration should not change inner html.
        assert_eq!(c.inner_html(), html);
    }
}

mod dynamic {
    use super::*;
    fn v<G: Html>(state: ReadSignal<i32>) -> View<G> {
        p().dyn_text(move || state.get().to_string()).build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(|| v(Signal::new(0).handle())),
            expect![[r##"<p data-hk="0.0"><!--#-->0<!--/--></p>"##]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(|| v(Signal::new(0).handle()));
        let c = test_container();
        c.set_inner_html(&html);

        let state = Signal::new(0);

        sycamore::hydrate_to(cloned!(state => move || v(state.handle())), &c);

        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "0"
        );

        // Reactivity should work normally.
        state.set(1);
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "1"
        );

        // P tag should still be the SSR-ed node, not a new node.
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .get_attribute("data-hk")
                .as_deref(),
            Some("0.0")
        );
    }
}

mod dynamic_with_siblings {
    use super::*;
    fn v<G: Html>(state: ReadSignal<i32>) -> View<G> {
        p().text("Value: ")
            .dyn_text(move || state.get().to_string())
            .text("!")
            .build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(|| v(Signal::new(0).handle())),
            expect![[r##"<p data-hk="0.0">Value: <!--#-->0<!--/-->!</p>"##]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(|| v(Signal::new(0).handle()));
        let c = test_container();
        c.set_inner_html(&html);

        let state = Signal::new(0);

        sycamore::hydrate_to(cloned!(state => move || v(state.handle())), &c);

        // Reactivity should work normally.
        state.set(1);
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "Value: 1!"
        );

        // P tag should still be the SSR-ed node, not a new node.
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .get_attribute("data-hk")
                .as_deref(),
            Some("0.0")
        );
    }
}

mod dynamic_template {
    use super::*;
    fn v<G: Html>(state: ReadSignal<View<G>>) -> View<G> {
        p().text("before")
            .dyn_child(move || state.get().as_ref().clone())
            .text("after")
            .build()
    }
    #[test]
    fn ssr() {
        check(
            &sycamore::render_to_string(|| v(Signal::new(view! { "text" }).into_handle())),
            expect![[r##"<p data-hk="0.0">before<!--#-->text<!--/-->after</p>"##]],
        );
    }
    #[wasm_bindgen_test]
    fn test() {
        let html = sycamore::render_to_string(|| v(Signal::new(view! { "text" }).into_handle()));
        let c = test_container();
        c.set_inner_html(&html);

        let state = Signal::new(view! { "text" });

        sycamore::hydrate_to(cloned!(state => move || v(state.handle())), &c);

        // Reactivity should work normally.
        state.set(view! { span { "nested node" } });
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "beforenested nodeafter"
        );

        // P tag should still be the SSR-ed node, not a new node.
        assert_eq!(
            c.query_selector("p")
                .unwrap()
                .unwrap()
                .get_attribute("data-hk")
                .as_deref(),
            Some("0.0")
        );
    }
}
